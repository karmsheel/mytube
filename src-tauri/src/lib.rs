pub mod catalog;
pub mod commands;
pub mod db;
pub mod error;
pub mod ffmpeg;
pub mod metadata;
pub mod models;
pub mod pathutil;
pub mod playlists;
pub mod resume;
pub mod scan;
pub mod slug;

use crate::commands::AppState;
use crate::db::open;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let thumbs = dir.join("thumbs");
            std::fs::create_dir_all(&thumbs)?;
            let conn = open(&dir.join("library.db"))?;
            if let Ok(sources) = crate::catalog::list_sources(&conn) {
                for s in sources {
                    let _ = app.asset_protocol_scope().allow_directory(&s.path, true);
                }
            }
            app.manage(AppState {
                db: Mutex::new(conn),
                thumbs_dir: thumbs,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::pick_folder,
            commands::add_source,
            commands::remove_source,
            commands::list_sources,
            commands::rescan,
            commands::list_home,
            commands::search,
            commands::list_channels,
            commands::get_channel,
            commands::get_video,
            commands::list_more,
            commands::video_url,
            commands::set_progress,
            commands::start_watch,
            commands::list_history,
            commands::list_playlists,
            commands::create_playlist,
            commands::delete_playlist,
            commands::get_playlist,
            commands::add_to_playlist,
            commands::remove_from_playlist,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
