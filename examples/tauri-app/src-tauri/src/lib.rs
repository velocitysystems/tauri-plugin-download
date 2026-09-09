use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .plugin(
            tauri_plugin_download::Builder::new()
                .user_agent("tauri-app/0.1.0")
                // A subdirectory of the app data directory rather than the directory
                // itself, so the store is visibly somewhere the plugin was told to put
                // it. `app.path()` resolves inside the sandbox on Android and iOS too,
                // which a path written at the call site could not.
                .on_setup(|app, config| {
                    config.store_dir(app.path().app_data_dir()?.join("downloads"));
                    Ok(())
                })
                .build(),
        )
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
