use tauri::{command, AppHandle, Runtime};

use crate::models::*;
use crate::DownloadExt;
use crate::Result;

#[command]
pub(crate) async fn create<R: Runtime>(
   app: AppHandle<R>,
   key: String,
   url: String,
   path: String,
) -> Result<DownloadItem> {
   app.download().create(app.clone(), key, url, path)
}

#[command]
pub(crate) async fn list<R: Runtime>(app: AppHandle<R>) -> Result<Vec<DownloadItem>> {
   app.download().list(app.clone())
}

#[command]
pub(crate) async fn get<R: Runtime>(app: AppHandle<R>, key: String) -> Result<DownloadItem> {
   app.download().get(app.clone(), key)
}

#[command]
pub(crate) async fn start<R: Runtime>(app: AppHandle<R>, key: String) -> Result<DownloadItem> {
   app.download().start(app.clone(), key)
}

#[command]
pub(crate) async fn cancel<R: Runtime>(app: AppHandle<R>, key: String) -> Result<DownloadItem> {
   app.download().cancel(app.clone(), key)
}

#[command]
pub(crate) async fn pause<R: Runtime>(app: AppHandle<R>, key: String) -> Result<DownloadItem> {
   app.download().pause(app.clone(), key)
}

#[command]
pub(crate) async fn resume<R: Runtime>(app: AppHandle<R>, key: String) -> Result<DownloadItem> {
   app.download().resume(app.clone(), key)
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn is_native<R: Runtime>(_app: AppHandle<R>) -> Result<bool> {
   #[cfg(any(target_os = "ios"))]
   {
      Ok(true)
   }
   #[cfg(any(desktop, target_os = "android"))]
   {
      Ok(false)
   }
}
