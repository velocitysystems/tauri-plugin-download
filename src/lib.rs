use tauri::{
   Manager, RunEvent, Runtime,
   plugin::{self, TauriPlugin},
};

#[cfg(desktop)]
use tauri::Emitter;
#[cfg(desktop)]
use tracing::warn;

mod commands;
mod error;
mod models;

pub use models::CreateOptions;

use error::Result;

#[cfg(desktop)]
use download_manager::{DownloadManager, DownloadManagerConfig};

#[cfg(mobile)]
mod mobile;
#[cfg(mobile)]
use mobile::Download;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the download APIs.
///
/// The trait is split by platform because the return type differs:
/// - Desktop uses the Tauri-agnostic `DownloadManager` (Rust implementation).
/// - Mobile delegates to the native plugin via a `PluginHandle`, so the return type
///   carries the `R: Runtime` generic required by Tauri's mobile plugin bridge.
#[cfg(desktop)]
pub trait DownloadExt<R: Runtime> {
   fn download(&self) -> &DownloadManager;
}

#[cfg(mobile)]
pub trait DownloadExt<R: Runtime> {
   fn download(&self) -> &Download<R>;
}

/// Blanket impl over any `T: Manager<R>` (i.e. `App`, `AppHandle`, `Window`) so callers
/// can use `app.download()` without explicitly referencing the managed state.
#[cfg(desktop)]
impl<R: Runtime, T: Manager<R>> crate::DownloadExt<R> for T {
   fn download(&self) -> &DownloadManager {
      self.state::<DownloadManager>().inner()
   }
}

#[cfg(mobile)]
impl<R: Runtime, T: Manager<R>> crate::DownloadExt<R> for T {
   fn download(&self) -> &Download<R> {
      self.state::<Download<R>>().inner()
   }
}

/// Plugin builder for configuring the download manager before initialization.
///
/// # Examples
///
/// ```no_run
/// tauri::Builder::default()
///    .plugin(
///       tauri_plugin_download::Builder::new()
///          .user_agent("my-app/1.0")
///          .build(),
///    );
/// ```
#[derive(Debug, Default)]
pub struct Builder {
   user_agent: Option<String>,
}

impl Builder {
   /// Creates a new builder, leaving every platform's own defaults in place.
   pub fn new() -> Self {
      Self::default()
   }

   /// Sets the `User-Agent` header sent with every download request.
   ///
   /// Applies on desktop, Android and iOS; unset, each keeps what its own HTTP stack
   /// sends. Must be printable ASCII or horizontal tab — the rule all three accept.
   /// Anything else fails plugin initialization rather than surfacing later as a
   /// failed download on one platform.
   pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
      self.user_agent = Some(user_agent.into());
      self
   }

   /// Builds the Tauri plugin with the configured settings.
   pub fn build<R: Runtime>(self) -> TauriPlugin<R> {
      let user_agent = self.user_agent;

      plugin::Builder::new("download")
         .invoke_handler(tauri::generate_handler![
            commands::create,
            commands::list,
            commands::get,
            commands::start,
            commands::cancel,
            commands::pause,
            commands::resume,
            commands::is_native,
         ])
         .setup(move |app, _api| {
            // Validated before either platform branch so an invalid value fails the
            // same way everywhere, rather than only where the transport rejects it.
            if let Some(ref user_agent) = user_agent {
               download_manager::validate_user_agent(user_agent)?;
            }

            #[cfg(desktop)]
            {
               // Resolve the app data directory for store persistence.
               let data_dir = app.path().app_data_dir().unwrap_or_else(|e| {
                  warn!("Failed to resolve app data dir, falling back to '.': {}", e);
                  std::path::PathBuf::from(".")
               });

               // Wire Tauri event emission as the on_changed callback.
               let app_handle = app.app_handle().clone();
               let manager = DownloadManager::new(
                  data_dir,
                  std::sync::Arc::new(move |item| {
                     if let Err(e) = app_handle.emit("tauri-plugin-download:changed", &item) {
                        warn!("Failed to emit change event: {}", e);
                     }
                  }),
                  DownloadManagerConfig { user_agent },
               );
               app.manage(manager);
            }

            #[cfg(mobile)]
            {
               // Mobile download management is handled natively by the platform plugin.
               let download = mobile::init(app, _api, user_agent)?;
               app.manage(download);
            }

            Ok(())
         })
         .on_event(|_app_handle, event| {
            if let RunEvent::Ready = event {
               // Initialize the download plugin.
               #[cfg(desktop)]
               _app_handle.state::<DownloadManager>().init();
            }
         })
         .build()
   }
}

/// Initializes the plugin with default settings.
///
/// Use [`Builder`] to configure it.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
   Builder::new().build()
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn test_user_agent_setter_stores_the_value() {
      // Not tautological: the field is read once, in `build`, and a setter that
      // dropped its argument would leave every download on the platform default with
      // nothing failing.
      let builder = Builder::new().user_agent("my-app/1.0");

      assert_eq!(builder.user_agent, Some("my-app/1.0".to_string()));
   }

   #[test]
   fn test_an_invalid_user_agent_fails_plugin_initialization() {
      // The promise the README makes. `validate_user_agent` runs ahead of the
      // desktop/mobile split, so this resolves on any host without a mobile target.
      let app = tauri::test::mock_builder()
         .plugin(Builder::new().user_agent("Caf\u{e9}/1.0").build())
         .build(tauri::test::mock_context(tauri::test::noop_assets()));

      assert!(matches!(app, Err(tauri::Error::PluginInitialization(_, _))));
   }

   #[test]
   fn test_a_valid_user_agent_initializes() {
      // Pairs with the case above: without it, any unrelated setup failure would
      // satisfy that assertion.
      let app = tauri::test::mock_builder()
         .plugin(Builder::new().user_agent("my-app/1.0").build())
         .build(tauri::test::mock_context(tauri::test::noop_assets()));

      assert!(app.is_ok());
   }
}
