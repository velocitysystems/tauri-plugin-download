use serde::de::DeserializeOwned;
use tauri::{
  plugin::{PluginApi, PluginHandle},
  AppHandle, Runtime,
};

use crate::models::*;

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_download);

// initializes the Kotlin or Swift plugin classes
pub fn init<R: Runtime, C: DeserializeOwned>(
  _app: &AppHandle<R>,
  api: PluginApi<R, C>,
) -> crate::Result<Download<R>> {
  #[cfg(target_os = "android")]
  let handle = api.register_android_plugin("", "ExamplePlugin")?;
  #[cfg(target_os = "ios")]
  let handle = api.register_ios_plugin(init_plugin_download)?;
  Ok(Download(handle))
}

/// Access to the download APIs.
pub struct Download<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> Download<R> {
  pub fn create(&self, key: String, url: String, path: String) -> crate::Result<DownloadRecord> {
    self
      .0
      .run_mobile_plugin("create", payload, url, path)
      .map_err(Into::into)
  }
}
