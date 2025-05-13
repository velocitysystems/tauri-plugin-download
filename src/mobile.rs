use serde::de::DeserializeOwned;
use tauri::{AppHandle, Runtime};
use tauri::plugin::{PluginApi, PluginHandle};

use crate::models::*;

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_download);

pub fn init<R: Runtime, C: DeserializeOwned>(
   _app: &AppHandle<R>,
   _api: PluginApi<R, C>,
) -> crate::Result<Download<R>> {
   #[cfg(target_os = "android")]
   let handle = _api.register_android_plugin("com.plugin.download", "DownloadPlugin")?;
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
      // Not implemented on mobile platforms since initialization is handled by the plugin.
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
   pub fn create(&self, _app: AppHandle<R>, key: String, url: String, path: String) -> crate::Result<DownloadItem> {
      self
         .0
         .run_mobile_plugin("create", CreateArgs { key, url, path })
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
   pub fn get(&self, _app: AppHandle<R>, key: String) -> crate::Result<DownloadItem> {
      self
         .0
         .run_mobile_plugin("get", KeyArgs { key })
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
   pub fn list(&self, _app: AppHandle<R>) -> crate::Result<Vec<DownloadItem>> {
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
   pub fn start(&self, _app: AppHandle<R>, key: String) -> crate::Result<DownloadItem> {
      self
         .0
         .run_mobile_plugin("start", KeyArgs { key })
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
   pub fn cancel(&self, _app: AppHandle<R>, key: String) -> crate::Result<DownloadItem> {
      self
         .0
         .run_mobile_plugin("cancel", KeyArgs { key })
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
   pub fn pause(&self, _app: AppHandle<R>, key: String) -> crate::Result<DownloadItem> {
      self
         .0
         .run_mobile_plugin("pause", KeyArgs { key })
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
   pub fn resume(&self, _app: AppHandle<R>, key: String) -> crate::Result<DownloadItem> {
      self
         .0
         .run_mobile_plugin("resume", KeyArgs { key })
         .map_err(Into::into)
   }
}
