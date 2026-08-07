// Desktop model types
#[cfg(desktop)]
pub use download_manager::{CreateOptions, DownloadActionResponse, DownloadItem};

// Mobile model types (iOS, Android)
#[cfg(mobile)]
mod mobile_types {
   use serde::{Deserialize, Serialize};

   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
   #[serde(rename_all = "camelCase")]
   pub struct CreateOptions {
      #[serde(default = "default_allow_metered")]
      pub allow_metered: bool,
   }

   impl Default for CreateOptions {
      fn default() -> Self {
         Self {
            allow_metered: true,
         }
      }
   }

   const fn default_allow_metered() -> bool {
      true
   }

   #[derive(Serialize)]
   #[serde(rename_all = "camelCase")]
   pub struct PathArgs {
      pub path: String,
   }

   #[derive(Serialize)]
   #[serde(rename_all = "camelCase")]
   pub struct CreateArgs {
      pub path: String,
      pub url: String,
   }

   #[derive(Debug, Clone, Default, Deserialize, Serialize)]
   #[serde(rename_all = "camelCase")]
   pub struct DownloadItem {
      pub url: String,
      pub path: String,
      #[serde(default)]
      pub received_bytes: u64,
      #[serde(default)]
      pub total_bytes: Option<u64>,
      #[serde(default)]
      pub progress: f64,
      pub status: DownloadStatus,
   }

   #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
   #[serde(rename_all = "camelCase")]
   pub enum DownloadStatus {
      #[default]
      Unknown,
      Pending,
      Idle,
      InProgress,
      Paused,
      Canceled,
      Completed,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   #[serde(rename_all = "camelCase")]
   pub struct DownloadActionResponse {
      pub download: DownloadItem,
      pub expected_status: DownloadStatus,
      pub is_expected_status: bool,
   }

   impl DownloadActionResponse {
      pub fn new(download: DownloadItem) -> Self {
         let expected_status = download.status.clone();
         Self {
            download,
            expected_status,
            is_expected_status: true,
         }
      }

      pub fn with_expected_status(download: DownloadItem, expected_status: DownloadStatus) -> Self {
         let is_expected_status = download.status == expected_status;
         Self {
            download,
            expected_status,
            is_expected_status,
         }
      }
   }
}

#[cfg(mobile)]
pub use mobile_types::{CreateArgs, CreateOptions, DownloadActionResponse, DownloadItem, PathArgs};
