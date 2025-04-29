use serde::de::DeserializeOwned;
use tauri::{AppHandle, Runtime};
use tauri::plugin::{PluginApi, PluginHandle};

use crate::models::*;

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_download);

pub fn init<R: Runtime, C: DeserializeOwned>(
   app: &AppHandle<R>,
   _api: PluginApi<R, C>,
) -> crate::Result<Download<R>> {
   #[cfg(target_os = "ios")]
   let handle = _api.register_ios_plugin(init_plugin_download)?;
   Ok(Download(handle))
}

/// Access to the download APIs.
pub struct Download<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> Download<R> {
   ///
   /// Initializes the API.
   /// Updates the state of any download operations which are still marked as "In Progress". This can occur if the
   /// application was suspended or terminated before a download was completed.
   ///
   pub fn init(&self) {
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
      self
         .0
         .run_mobile_plugin("create", CreateRequest { key, url, path })
         .map_err(Into::into)
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
      self
         .0
         .run_mobile_plugin("get", GetRequest { key })
         .map_err(Into::into)
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
      self
         .0
         .run_mobile_plugin("list", ())
         .map_err(Into::into)
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
      self
         .0
         .run_mobile_plugin("start", GetRequest { key })
         .map_err(Into::into)
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
      self
         .0
         .run_mobile_plugin("cancel", GetRequest { key })
         .map_err(Into::into)
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
      self
         .0
         .run_mobile_plugin("pause", GetRequest { key })
         .map_err(Into::into)
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
      self
         .0
         .run_mobile_plugin("resume", GetRequest { key })
         .map_err(Into::into)
   }
}
