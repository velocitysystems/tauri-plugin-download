use std::fmt;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateArgs {
  pub key: String,
  pub url: String,
  pub path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyArgs {
  pub key: String
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadItem {
   pub key: String,
   pub url: String,
   pub path: String,
   pub progress: f64,
   pub state: DownloadState,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadState {
   #[default]
   Unknown,
   Created,
   InProgress,
   Paused,
   Cancelled,
   Completed,
}

#[cfg(desktop)]
pub trait DownloadItemExt {
   fn with_path(&self, new_path: String) -> DownloadItem;
   fn with_progress(&self, new_progress: f64) -> DownloadItem;
   fn with_state(&self, new_state: DownloadState) -> DownloadItem;
}

#[cfg(desktop)]
impl DownloadItemExt for DownloadItem {
   fn with_path(&self, new_path: String) -> DownloadItem {
      DownloadItem {
         path: new_path,
         ..self.clone() // Clone the rest of the fields
      }
   }

   fn with_progress(&self, new_progress: f64) -> DownloadItem {
      DownloadItem {
         progress: new_progress,
         state: DownloadState::InProgress,
         ..self.clone() // Clone the rest of the fields
      }
   }

   fn with_state(&self, new_state: DownloadState) -> DownloadItem {
      DownloadItem {
         progress: if new_state == DownloadState::Completed {
            100.0
         } else {
            self.progress
         },
         state: new_state,
         ..self.clone() // Clone the rest of the fields
      }
   }
}

#[cfg(desktop)]
impl fmt::Display for DownloadState {
   fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      let text = match self {
         DownloadState::Unknown => "Unknown",
         DownloadState::Created => "Created",
         DownloadState::InProgress => "InProgress",
         DownloadState::Paused => "Paused",
         DownloadState::Cancelled => "Cancelled",
         DownloadState::Completed => "Completed",
      };
      write!(f, "{}", text)
   }
}
