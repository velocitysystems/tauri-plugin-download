use tauri::{
   plugin::{Builder, TauriPlugin},
   Manager, Runtime,
};

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;
mod store;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::Download;
#[cfg(mobile)]
use mobile::Download;
use tauri_plugin_store::StoreExt;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the download APIs.
pub trait DownloadExt<R: Runtime> {
   fn download(&self) -> &Download<R>;
}

impl<R: Runtime, T: Manager<R>> crate::DownloadExt<R> for T {
   fn download(&self) -> &Download<R> {
      self.state::<Download<R>>().inner()
   }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
   Builder::new("download")
      .invoke_handler(tauri::generate_handler![
         commands::create,
         commands::list,
         commands::get,
         commands::start,
         commands::cancel,
         commands::pause,
         commands::resume
      ])
      .setup(|app, api| {
         #[cfg(mobile)]
         let download = mobile::init(app, api)?;
         #[cfg(desktop)]
         let download = desktop::init(app, api)?;
         app.manage(download);

         // Dynamically register/initialize the store plugin.
         // https://docs.rs/tauri/latest/tauri/struct.AppHandle.html#method.plugin
         let handle = app.app_handle().clone();
         std::thread::spawn(move || {
            handle
               .plugin(tauri_plugin_store::Builder::new().build())
               .unwrap();
            handle.store("downloads.json").unwrap();
         });

         Ok(())
      })
      .build()
}
