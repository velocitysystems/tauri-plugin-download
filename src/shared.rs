use futures::StreamExt;
use serde::de::DeserializeOwned;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use tauri::AppHandle;
use tauri::{plugin::PluginApi, Emitter, Runtime};
use tauri_plugin_http::reqwest;
use tauri_plugin_http::reqwest::header::{HeaderMap, RANGE};

use crate::{models::*, store, utils};
use crate::Error;

static DOWNLOAD_SUFFIX: &str = ".download";

pub fn init<R: Runtime, C: DeserializeOwned>(
   app: &AppHandle<R>,
   _api: PluginApi<R, C>,
) -> crate::Result<Download<R>> {
   Ok(Download(app.clone()))
}

/// Access to the download APIs.
pub struct Download<R: Runtime>(AppHandle<R>);

impl<R: Runtime> Download<R> {
   ///
   /// Initializes the API.
   /// Updates the state of any download operations which are still marked as "In Progress". This can occur if the
   /// application was suspended or terminated before a download was completed.
   ///
   pub fn init(&self)
   {
      let records: Vec<_> = store::get_records(&self.0)
         .unwrap()
         .iter()
         .filter(|item| item.state == DownloadState::InProgress)
         .cloned()
         .collect();

      records.into_iter().for_each(|record| {
         let new_state = if record.progress == 0.0 { DownloadState::Created } else { DownloadState::Paused };
         store::update_record(&self.0, record.with_state(new_state.clone())).unwrap();
         println!(
            "[{}] Found download operation - {}",
            &record.key,
            new_state
         );
      });
   }

   ///
   /// Creates a download operation.
   ///
   /// # Arguments
   /// - `app` - The application handle.
   /// - `key` - The key identifier.
   /// - `url` - The download URL  for the resource.
   /// - `path` - The download path on the filesystem.
   ///
   /// # Returns
   /// The download operation.
   pub fn create(
      &self,
      app: AppHandle<R>,
      key: String,
      url: String,
      path: String,
   ) -> crate::Result<DownloadRecord> {
      let path = format!("{}{}", path, DOWNLOAD_SUFFIX);
      store::create_record(
         &app,
         DownloadRecord {
            key,
            url,
            path,
            progress: 0.0,
            state: DownloadState::Created,
         },
      )
   }

   ///
   /// Gets a download operation.
   ///
   /// # Arguments
   /// - `app` - The application handle.
   /// - `key` - The key identifier.
   ///
   /// # Returns
   /// The download operation.
   pub fn get(&self, app: AppHandle<R>, key: String) -> crate::Result<DownloadRecord> {
      store::get_record(&app, key)
   }

   ///
   /// Lists all download operations.
   ///
   /// # Arguments
   /// - `app` - The application handle.
   ///
   /// # Returns
   /// The list of download operations.
   pub fn list(&self, app: AppHandle<R>) -> crate::Result<Vec<DownloadRecord>> {
      store::get_records(&app)
   }

   ///
   /// Starts a download operation.
   ///
   /// # Arguments
   /// - `app` - The application handle.
   /// - `key` - The key identifier.
   ///
   /// # Returns
   /// The download operation.
   pub fn start(&self, app: AppHandle<R>, key: String) -> crate::Result<DownloadRecord> {
      let record = store::get_record(&app, key).unwrap();
      match record.state {
         // Allow download to be started when created.
         DownloadState::Created => {
            let record_started = record.with_state(DownloadState::InProgress);
            tokio::spawn(async move {
               Download::download(&app, record_started).await.unwrap();
            });

            Ok(record.with_state(DownloadState::InProgress))
         }

         // Throw if in any other state.
         _ => Err(Error::InvalidState),
      }
   }

   ///
   /// Cancels a download operation.
   ///
   /// # Arguments
   /// - `app` - The application handle.
   /// - `key` - The key identifier.
   ///
   /// # Returns
   /// The download operation.
   pub fn cancel(&self, app: AppHandle<R>, key: String) -> crate::Result<DownloadRecord> {
      let record = store::get_record(&app, key).unwrap();
      match record.state {
         // Allow download to be cancelled when created, in progress or paused.
         DownloadState::Created | DownloadState::InProgress | DownloadState::Paused => {
            store::remove_record(&app, record.key.clone()).unwrap();
            if fs::remove_file(record.path.clone()).is_err() {
               println!(
                  "[{}] File was not found or could not be deleted",
                  &record.key
               );
            }

            Download::emit_changed(&app, record.with_state(DownloadState::Cancelled));
            Ok(record.with_state(DownloadState::Cancelled))
         }

         // Throw if in any other state.
         _ => Err(Error::InvalidState),
      }
   }

   ///
   /// Pauses a download operation.
   ///
   /// # Arguments
   /// - `app` - The application handle.
   /// - `key` - The key identifier.
   ///
   /// # Returns
   /// The download operation.
   pub fn pause(&self, app: AppHandle<R>, key: String) -> crate::Result<DownloadRecord> {
      let record = store::get_record(&app, key).unwrap();
      match record.state {
         // Allow download to be paused when in progress.
         DownloadState::InProgress => {
            store::update_record(&app, record.with_state(DownloadState::Paused)).unwrap();
            Download::emit_changed(&app, record.with_state(DownloadState::Paused));
            Ok(record.with_state(DownloadState::Paused))
         }

         // Throw if in any other state.
         _ => Err(Error::InvalidState),
      }
   }

   ///
   /// Resumes a download operation.
   ///
   /// # Arguments
   /// - `app` - The application handle.
   /// - `key` - The key identifier.
   ///
   /// # Returns
   /// The download operation.
   pub fn resume(&self, app: AppHandle<R>, key: String) -> crate::Result<DownloadRecord> {
      let record = store::get_record(&app, key).unwrap();
      match record.state {
         // Allow download to be resumed when paused.
         DownloadState::Paused => {
            let record_resumed = record.with_state(DownloadState::InProgress);
            tokio::spawn(async move {
               Download::download(&app, record_resumed).await.unwrap();
            });

            Ok(record.with_state(DownloadState::InProgress))
         }

         // Throw if in any other state.
         _ => Err(Error::InvalidState),
      }
   }

   async fn download(app: &AppHandle<R>, record: DownloadRecord) -> crate::Result<()> {
      let client = reqwest::Client::new();

      // Check the size of the already downloaded part, if any.
      let downloaded_size = if std::path::Path::new(&record.path).exists() {
         fs::metadata(&record.path)
            .map(|metadata| metadata.len())
            .unwrap_or(0)
      } else {
         0
      };

      // Set the Range header for resuming the download.
      let mut headers = HeaderMap::new();
      if downloaded_size > 0 {
         headers.insert(
            RANGE,
            format!("bytes={}-", downloaded_size).parse().unwrap(),
         );
      }

      // Send the request.
      let response = match client.get(&record.url).headers(headers).send().await {
         Ok(res) => res,
         Err(e) => {
            return Err(Error::HttpError(format!("Failed to send request: {}", e)));
         }
      };

      // Ensure the server supports partial downloads.
      if downloaded_size > 0 && response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
         return Err(Error::HttpError(
            "Server does not support partial downloads".to_string(),
         ));
      }

      // Get the total size of the file from headers (if available).
      let total_size = response
         .headers()
         .get("content-length")
         .and_then(|len| len.to_str().ok())
         .and_then(|len| len.parse::<u64>().ok())
         .map(|len| len + downloaded_size)
         .unwrap_or(0);

      // Ensure the output folder exists.
      let folder = Path::new(&record.path).parent().unwrap();
      if !folder.exists() {
         fs::create_dir(folder).unwrap();
      }

      // Open the file in append mode.
      let mut file = OpenOptions::new()
         .create(true)
         .append(true)
         .open(&record.path)
         .map_err(|e| Error::FileError(format!("Failed to open file: {}", e)))?;

      // Write the response body to the file in chunks.
      let mut downloaded = downloaded_size;
      let mut stream = response.bytes_stream();

      // Throttle progress updates.
      let mut last_emitted_progress = 0.0;
      const PROGRESS_THRESHOLD: f64 = 1.0; // Only update if progress increases by at least 1%.

      store::update_record(app, record.with_state(DownloadState::InProgress)).unwrap();
      Download::emit_changed(app, record.with_state(DownloadState::InProgress));

      'reader: while let Some(chunk) = stream.next().await {
         match chunk {
            Ok(data) => {
               file
                  .write_all(&data)
                  .map_err(|e| Error::FileError(format!("Failed to write file: {}", e)))?;

               downloaded += data.len() as u64;
               let progress = (downloaded as f64 / total_size as f64) * 100.0;
               if progress < 100.0 && progress - last_emitted_progress <= PROGRESS_THRESHOLD {
                  // Ignore any progress updates below the threshold.
                  continue;
               }

               last_emitted_progress = progress;
               if let Ok(record) = store::get_record(app, record.key.clone()) {
                  match record.state {
                     // Download is in progress.
                     DownloadState::InProgress => {
                        if progress < 100.0 {
                           // Download is not yet complete.
                           // Update record in store and emit change event.
                           store::update_record(app, record.with_progress(progress)).unwrap();
                           Download::emit_changed(app, record.with_progress(progress));
                        } else if progress == 100.0 {
                           // Download has completed.
                           // Remove record from store, rename output file and emit change event.
                           store::remove_record(app, record.key.clone()).unwrap();

                           let output_path = utils::remove_suffix(&record.path, DOWNLOAD_SUFFIX);
                           fs::rename(&record.path, output_path)?;
                           Download::emit_changed(app, record
                              .with_path(output_path.into())
                              .with_state(DownloadState::Completed));
                        }
                     }
                     // Download was paused.
                     DownloadState::Paused => {
                        break 'reader;
                     }
                     _ => (),
                  }
               } else {
                  // Download record was not found i.e. removed.
                  break 'reader;
               }
            }
            Err(e) => {
               // Download error occured.
               // Remove record from store and partial download.
               store::remove_record(app, record.key.clone()).unwrap();
               if std::path::Path::new(&record.path).exists() {
                  fs::remove_file(&record.path)?;
               }

               return Err(Error::HttpError(format!("Failed to download: {}", e)))
            },
         }
      }

      Ok(())
   }

   fn emit_changed(app: &AppHandle<R>, record: DownloadRecord) {
      app.emit("tauri-plugin-download:changed", &record).unwrap();
      println!(
         "[{}] {} - {:.0}%",
         record.key, record.state, record.progress
      );
   }
}
