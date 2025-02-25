use tauri::{
   plugin::{Builder, TauriPlugin}, Manager, RunEvent, Runtime
};

pub use models::*;

mod commands;
mod error;
mod models;
mod shared;
mod store;
mod utils;

pub use error::{Error, Result};

use shared::Download;
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
         let download = shared::init(app, api)?;
         app.manage(download);

         // Initialize the store plugin.
         // https://docs.rs/tauri/latest/tauri/struct.AppHandle.html#method.plugin
         let handle = app.app_handle().clone();
         std::thread::spawn(move || {
            handle
               .plugin(tauri_plugin_store::Builder::new().build())
               .unwrap();
            handle.store("downloads.json").unwrap();
         });

         Ok(())
      }).on_event(|app_handle, event| {
         if let RunEvent::Ready = event {
            // Initialize the download plugin.
            app_handle.state::<Download<R>>().init();
         }
     })
      .build()
}
