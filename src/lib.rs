use std::path::PathBuf;

use tauri::{
   AppHandle, Manager, RunEvent, Runtime,
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

/// Closure type for the deferred [`Builder::on_setup`] hook.
///
/// Boxed error rather than [`crate::Result`]: this crate's `Error` is a different type
/// on desktop and mobile, so neither can appear in a cross-platform signature. It is
/// also Tauri's own `setup` return type, so `app.path().app_data_dir()?` just works.
type OnSetupHook<R> = Box<
   dyn FnOnce(&AppHandle<R>, &mut SetupConfig) -> std::result::Result<(), Box<dyn std::error::Error>>
      + Send,
>;

/// Collects the settings that can only be resolved once the `app` instance exists.
///
/// Passed to the [`Builder::on_setup`] hook during plugin setup. Named for the payload
/// rather than the setting, as [`ConfigArgs`](crate::models) is, so a second
/// runtime-resolved option is a new method here rather than a second hook.
#[derive(Debug, Default)]
pub struct SetupConfig {
   store_dir: Option<PathBuf>,
}

impl SetupConfig {
   /// Sets the directory the download store is persisted in.
   ///
   /// The filename is fixed at `downloads.json` everywhere; this names the directory
   /// holding it. Must be absolute, and on mobile inside the app sandbox — which only
   /// `app.path()` can name, and the reason this is a hook rather than a setter.
   ///
   /// Unset, each platform keeps its default: the app data directory on desktop,
   /// internal storage on Android, `Documents` on iOS.
   ///
   /// Changing it does not migrate an existing store; its records become invisible.
   pub fn store_dir(&mut self, dir: impl Into<PathBuf>) -> &mut Self {
      self.store_dir = Some(dir.into());
      self
   }
}

/// Plugin builder for configuring the download manager before initialization.
///
/// # Examples
///
/// ```no_run
/// use tauri::Manager;
///
/// tauri::Builder::default()
///    .plugin(
///       tauri_plugin_download::Builder::new()
///          .user_agent("my-app/1.0")
///          .on_setup(|app, config| {
///             config.store_dir(app.path().app_data_dir()?.join("downloads"));
///             Ok(())
///          })
///          .build(),
///    );
/// ```
pub struct Builder<R: Runtime> {
   user_agent: Option<String>,
   on_setup: Option<OnSetupHook<R>>,
}

/// Hand-written rather than derived: a boxed closure is not [`Debug`], and deriving
/// would additionally require `R: Debug`, which no runtime satisfies.
impl<R: Runtime> std::fmt::Debug for Builder<R> {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      f.debug_struct("Builder")
         .field("user_agent", &self.user_agent)
         .field("on_setup", &self.on_setup.is_some())
         .finish()
   }
}

/// Hand-written for the same reason `Debug` is: `#[derive(Default)]` would require
/// `R: Default`.
impl<R: Runtime> Default for Builder<R> {
   fn default() -> Self {
      Self {
         user_agent: None,
         on_setup: None,
      }
   }
}

impl<R: Runtime> Builder<R> {
   /// Creates a new builder, leaving every platform's own defaults in place.
   pub fn new() -> Self {
      Self::default()
   }

   /// Registers a hook that runs during plugin setup, once the `app` instance exists.
   ///
   /// The only way to set a store directory: legitimate directories come from
   /// `app.path()`, and the builder runs before `tauri::Builder::run` creates the app.
   /// On mobile it is also the only way to name a writable sandbox directory.
   ///
   /// Returning `Err` aborts startup, as an invalid [`user_agent`](Self::user_agent) does.
   pub fn on_setup(
      mut self,
      f: impl FnOnce(
         &AppHandle<R>,
         &mut SetupConfig,
      ) -> std::result::Result<(), Box<dyn std::error::Error>>
      + Send
      + 'static,
   ) -> Self {
      self.on_setup = Some(Box::new(f));
      self
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
   pub fn build(self) -> TauriPlugin<R> {
      let user_agent = self.user_agent;
      let on_setup = self.on_setup;

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
            // Runs first so the settings it resolves are validated below alongside the
            // ones set on the builder, rather than reaching a platform unchecked.
            let mut config = SetupConfig::default();
            if let Some(on_setup) = on_setup {
               on_setup(app, &mut config)?;
            }

            // Validated before either platform branch so an invalid value fails the
            // same way everywhere, rather than only where the transport rejects it.
            if let Some(ref user_agent) = user_agent {
               download_manager::validate_user_agent(user_agent)?;
            }

            if let Some(ref store_dir) = config.store_dir {
               download_manager::validate_store_dir(store_dir)?;
            }

            #[cfg(desktop)]
            {
               // A configured store directory is used as given. Only the default is
               // resolved — and only it falls back, since a caller who named a
               // directory should not silently get a different one.
               let data_dir = config.store_dir.unwrap_or_else(|| {
                  app.path().app_data_dir().unwrap_or_else(|e| {
                     warn!("Failed to resolve app data dir, falling back to '.': {}", e);
                     std::path::PathBuf::from(".")
                  })
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
               // The bridge to native is JSON, so the directory has to be UTF-8. A path
               // that is not fails here rather than being dropped, which would leave the
               // store at the platform default with nothing having failed.
               let store_dir = match config.store_dir {
                  Some(dir) => Some(
                     dir.to_str()
                        .ok_or_else(|| format!("Store directory is not valid UTF-8: {:?}", dir))?
                        .to_string(),
                  ),
                  None => None,
               };

               // Mobile download management is handled natively by the platform plugin.
               let download = mobile::init(app, _api, user_agent, store_dir)?;
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
   Builder::<R>::new().build()
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn test_user_agent_setter_stores_the_value() {
      // Not tautological: the field is read once, in `build`, and a setter that
      // dropped its argument would leave every download on the platform default with
      // nothing failing.
      let builder = Builder::<tauri::test::MockRuntime>::new().user_agent("my-app/1.0");

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

   #[test]
   fn test_store_dir_setter_stores_the_value() {
      // Same reasoning as the user agent setter above: the field is read once, in
      // `build`, so a setter that dropped its argument would silently leave the store
      // at the platform default.
      let mut config = SetupConfig::default();

      config.store_dir("/var/lib/myapp");

      assert_eq!(config.store_dir, Some(PathBuf::from("/var/lib/myapp")));
   }

   #[test]
   fn test_the_setup_hook_runs_during_plugin_initialization() {
      // The hook exists to be run with an app handle that only exists at setup time.
      // Without this, `on_setup` could store a closure that is never called and every
      // other test here would still pass.
      let ran = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
      let flag = ran.clone();

      let app = tauri::test::mock_builder()
         .plugin(
            Builder::new()
               .on_setup(move |_app, _config| {
                  flag.store(true, std::sync::atomic::Ordering::SeqCst);
                  Ok(())
               })
               .build(),
         )
         .build(tauri::test::mock_context(tauri::test::noop_assets()));

      assert!(app.is_ok());
      assert!(ran.load(std::sync::atomic::Ordering::SeqCst));
   }

   #[test]
   fn test_a_relative_store_dir_fails_plugin_initialization() {
      // The mirror of the invalid user agent case. `validate_store_dir` runs ahead of
      // the desktop/mobile split, so this resolves on any host without a mobile target.
      let app = tauri::test::mock_builder()
         .plugin(
            Builder::new()
               .on_setup(|_app, config| {
                  config.store_dir("downloads");
                  Ok(())
               })
               .build(),
         )
         .build(tauri::test::mock_context(tauri::test::noop_assets()));

      assert!(matches!(app, Err(tauri::Error::PluginInitialization(_, _))));
   }

   #[test]
   fn test_an_error_from_the_setup_hook_fails_plugin_initialization() {
      // The documented contract: a caller whose `app.path()` lookup fails aborts
      // startup rather than silently falling back to a directory they did not choose.
      let app = tauri::test::mock_builder()
         .plugin(
            Builder::new()
               .on_setup(|_app, _config| Err("no directory for you".into()))
               .build(),
         )
         .build(tauri::test::mock_context(tauri::test::noop_assets()));

      assert!(matches!(app, Err(tauri::Error::PluginInitialization(_, _))));
   }

   #[cfg(desktop)]
   #[test]
   fn test_the_store_is_persisted_in_the_configured_directory() {
      use download_manager::DownloadItem;

      // The feature itself, end to end through the plugin: creating a download has to
      // write `downloads.json` into the configured directory. Asserting on the setter
      // alone would pass even if `build` ignored the value.
      let dir = tempfile::tempdir().unwrap();
      let store_dir = dir.path().join("configured");
      let expected = store_dir.join("downloads.json");

      let configured = store_dir.clone();
      let app = tauri::test::mock_builder()
         .plugin(
            Builder::new()
               .on_setup(move |_app, config| {
                  config.store_dir(configured.clone());
                  Ok(())
               })
               .build(),
         )
         .build(tauri::test::mock_context(tauri::test::noop_assets()))
         .unwrap();

      let download: DownloadItem = app
         .download()
         .create(
            dir.path().join("file.mp4").to_str().unwrap(),
            "https://example.com/file.mp4",
         )
         .unwrap()
         .download;

      assert_eq!(download.status, download_manager::DownloadStatus::Idle);
      assert!(expected.exists(), "store not written to {:?}", expected);
   }
}
