// Desktop model types
#[cfg(desktop)]
pub use download_manager::{CreateOptions, DownloadActionResponse, DownloadItem};

/// Wire form of [`CreateOptions`] for the `create` command. TypeScript declares
/// `allowMetered` optional, so `create(url, {})` must mean "unstated" rather than
/// fail; holding that tolerance here lets the persisted [`CreateOptions`] require
/// the value. Unknown keys are ignored, not rejected.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateOptionsArgs {
   pub allow_metered: Option<bool>,
}

impl From<CreateOptionsArgs> for CreateOptions {
   fn from(arg: CreateOptionsArgs) -> Self {
      Self {
         allow_metered: arg.allow_metered.unwrap_or(Self::default().allow_metered),
      }
   }
}

// Mobile model types (iOS, Android)
#[cfg(mobile)]
mod mobile_types {
   use serde::{Deserialize, Serialize};

   /// Shared with desktop rather than mirrored. The type sits on the bridge in both
   /// directions now, and a second definition of it — with its own copy of the
   /// default — could drift without the compiler, the tests, or CI noticing.
   pub use download_manager::CreateOptions;

   #[derive(Serialize)]
   #[serde(rename_all = "camelCase")]
   pub struct PathArgs {
      pub path: String,
   }

   /// Settings pushed to the native plugin once at startup.
   ///
   /// Named for the payload rather than the setting so a second builder option is a
   /// new field here, not a second command mirrored across Kotlin and Swift.
   #[derive(Serialize)]
   #[serde(rename_all = "camelCase")]
   pub struct ConfigArgs {
      pub user_agent: String,
   }

   #[derive(Serialize)]
   #[serde(rename_all = "camelCase")]
   pub struct CreateArgs {
      pub path: String,
      pub url: String,
      pub options: CreateOptions,
   }

   #[derive(Debug, Clone, Default, Deserialize, Serialize)]
   #[serde(rename_all = "camelCase")]
   pub struct DownloadItem {
      pub url: String,
      pub path: String,
      pub options: CreateOptions,
      pub received_bytes: u64,
      pub total_bytes: Option<u64>,
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
pub use mobile_types::{
   ConfigArgs, CreateArgs, CreateOptions, DownloadActionResponse, DownloadItem, PathArgs,
};

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn test_unstated_policy_takes_the_api_default() {
      // `create(url, {})` from TypeScript decodes to this: serde reads an absent
      // Option as None, so the default is resolved here rather than by tolerating
      // a missing field on the persisted `CreateOptions`.
      let arg = CreateOptionsArgs {
         allow_metered: None,
      };

      assert_eq!(
         CreateOptions::from(arg).allow_metered,
         CreateOptions::default().allow_metered
      );
   }

   #[test]
   fn test_stated_policy_is_carried_through() {
      let arg = CreateOptionsArgs {
         allow_metered: Some(false),
      };

      assert!(!CreateOptions::from(arg).allow_metered);
   }

   #[test]
   fn test_empty_json_object_resolves_to_the_api_default() {
      // The `create(url, {})` case the struct's doc comment describes, decoded
      // through serde rather than constructed, so it holds at the wire.
      let arg: CreateOptionsArgs = serde_json::from_str("{}").unwrap();

      assert_eq!(arg.allow_metered, None);
      assert_eq!(
         CreateOptions::from(arg).allow_metered,
         CreateOptions::default().allow_metered
      );
   }

   #[test]
   fn test_json_false_is_carried_through_to_the_resolved_options() {
      let arg: CreateOptionsArgs = serde_json::from_str(r#"{"allowMetered":false}"#).unwrap();

      assert_eq!(arg.allow_metered, Some(false));
      assert!(!CreateOptions::from(arg).allow_metered);
   }

   #[test]
   fn test_json_true_is_carried_through_to_the_resolved_options() {
      let arg: CreateOptionsArgs = serde_json::from_str(r#"{"allowMetered":true}"#).unwrap();

      assert_eq!(arg.allow_metered, Some(true));
      assert!(CreateOptions::from(arg).allow_metered);
   }

   #[test]
   fn test_json_null_is_treated_the_same_as_the_field_being_absent() {
      // Null and absent are deliberately the same under "unstated" semantics.
      let arg: CreateOptionsArgs = serde_json::from_str(r#"{"allowMetered":null}"#).unwrap();

      assert_eq!(arg.allow_metered, None);
      assert_eq!(
         CreateOptions::from(arg).allow_metered,
         CreateOptions::default().allow_metered
      );
   }

   #[test]
   fn test_unrecognised_key_is_ignored_and_resolves_to_the_default() {
      // Characterisation test, not a design goal: `guest-js/actions.ts` forwards
      // the caller's options object verbatim, so `deny_unknown_fields` would break
      // a caller whose variable is typed wider than the options shape.
      let arg: CreateOptionsArgs = serde_json::from_str(r#"{"allowmetered":false}"#).unwrap();

      assert_eq!(arg.allow_metered, None);
      assert_eq!(
         CreateOptions::from(arg).allow_metered,
         CreateOptions::default().allow_metered
      );
   }
}
