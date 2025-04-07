use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;

use crate::{DownloadRecord, Error};

static DOWNLOAD_STORE_PATH: &str = "downloads.json";

pub fn get_records<R: Runtime>(app: &AppHandle<R>) -> crate::Result<Vec<DownloadRecord>> {
   let store = app
      .store(DOWNLOAD_STORE_PATH)
      .map_err(|e| Error::Store(format!("Failed to load store: {}", e)))?;

   let mut records = Vec::new();
   for key in store.keys() {
      if let Some(value) = store.get(&key) {
         let record: DownloadRecord = serde_json::from_value(value)
            .map_err(|e| Error::Store(format!("Failed to parse record: {}", e)))?;
         records.push(record);
      }
   }

   Ok(records)
}

pub fn get_record<R: Runtime>(app: &AppHandle<R>, key: String) -> crate::Result<DownloadRecord> {
   let store = app
      .store(DOWNLOAD_STORE_PATH)
      .map_err(|e| Error::Store(format!("Failed to load store: {}", e)))?;

   match store.get(&key) {
      Some(value) => Ok(serde_json::from_value(value).unwrap()),
      None => Err(Error::Store(format!(
         "No download record found for key: {}",
         key
      ))),
   }
}

pub fn create_record<R: Runtime>(
   app: &AppHandle<R>,
   record: DownloadRecord,
) -> crate::Result<DownloadRecord> {
   let store = app
      .store(DOWNLOAD_STORE_PATH)
      .map_err(|e| Error::Store(format!("Failed to load store: {}", e)))?;

   match store.get(&record.key) {
      Some(_) => {
         return Err(Error::Store(format!(
            "Record already exists for key: {}",
            &record.key
         )))
      }
      None => {
         store.set(&record.key, serde_json::to_value(&record).unwrap());
         store
            .save()
            .map_err(|e| Error::Store(format!("Failed to save store: {}", e)))?;
      }
   }

   Ok(record)
}

pub fn update_record<R: Runtime>(app: &AppHandle<R>, record: DownloadRecord) -> crate::Result<()> {
   let store = app
      .store(DOWNLOAD_STORE_PATH)
      .map_err(|e| Error::Store(format!("Failed to load store: {}", e)))?;

   store.set(&record.key, serde_json::to_value(&record).unwrap());
   store
      .save()
      .map_err(|e| Error::Store(format!("Failed to save store: {}", e)))?;

   Ok(())
}

pub fn remove_record<R: Runtime>(app: &AppHandle<R>, key: String) -> crate::Result<()> {
   let store = app
      .store(DOWNLOAD_STORE_PATH)
      .map_err(|e| Error::Store(format!("Failed to load store: {}", e)))?;

   if store.has(&key) {
      store.delete(&key);
   }

   store
      .save()
      .map_err(|e| Error::Store(format!("Failed to save store: {}", e)))?;

   Ok(())
}
