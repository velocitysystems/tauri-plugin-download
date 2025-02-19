use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRecord {
  pub key: String,
  pub url: String,
  pub path: String,
  pub progress: f64,
  pub state: DownloadState,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
  pub key: String,
  pub progress: f64,   
}

pub trait DownloadRecordExt {
  fn with_progress(&self, new_progress: f64) -> DownloadRecord;
  fn with_state(&self, new_state: DownloadState) -> DownloadRecord;
}

impl DownloadRecordExt for DownloadRecord {
  fn with_progress(&self, new_progress: f64) -> DownloadRecord {
      DownloadRecord {
          progress: new_progress,
          state: DownloadState::InProgress,
          ..self.clone() // Clone the rest of the fields
      }
  }

  fn with_state(&self, new_state: DownloadState) -> DownloadRecord {
      DownloadRecord {
          state: new_state,
          ..self.clone() // Clone the rest of the fields
      }
  }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DownloadState {
   #[default]
   Unknown,
   Created,
   InProgress,
   Paused,
   Cancelled,
}
