use tauri::{AppHandle, command, Runtime};

use crate::models::*;
use crate::Result;
use crate::DownloadExt;

#[command]
pub(crate) async fn create<R: Runtime>(
    app: AppHandle<R>,
    key: String,
    url: String,
    path: String,
) -> Result<DownloadRecord> {
    app.download().create(app.clone(), key, url, path)
}

#[command]
pub(crate) async fn list<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Vec<DownloadRecord>> {
    app.download().list(app.clone())
}

#[command]
pub(crate) async fn get<R: Runtime>(
    app: AppHandle<R>,
    key: String,
) -> Result<DownloadRecord> {
    app.download().get(app.clone(), key)
}

#[command]
pub(crate) async fn start<R: Runtime>(
    app: AppHandle<R>,
    key: String,
) -> Result<DownloadRecord> {
    app.download().start(app.clone(), key)
}

#[command]
pub(crate) async fn cancel<R: Runtime>(
    app: AppHandle<R>,
    key: String,
) -> Result<DownloadRecord> {
    app.download().cancel(app.clone(), key)
}

#[command]
pub(crate) async fn pause<R: Runtime>(
    app: AppHandle<R>,
    key: String,
) -> Result<DownloadRecord> {
    app.download().pause(app.clone(), key)
}

#[command]
pub(crate) async fn resume<R: Runtime>(
    app: AppHandle<R>,
    key: String,
) -> Result<DownloadRecord> {
    app.download().resume(app.clone(), key)
}
