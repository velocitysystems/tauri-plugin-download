mod downloader;
mod error;
mod manager;
mod models;
mod store;
mod validate;

pub use error::{Error, Result};
pub use manager::{DownloadManager, DownloadManagerConfig, OnChanged};
pub use models::{CreateOptions, DownloadActionResponse, DownloadItem, DownloadStatus};
pub use validate::user_agent as validate_user_agent;
