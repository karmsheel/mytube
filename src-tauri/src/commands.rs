use crate::catalog;
use crate::error::{AppError, AppResult};
use crate::models::{Channel, Page, ScanStats, Source, VideoCard, VideoDetail};
use crate::scan::{rescan_all, scan_source};
use rusqlite::Connection;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

pub struct AppState {
    pub db: Mutex<Connection>,
    pub thumbs_dir: PathBuf,
}

fn lock(state: &AppState) -> AppResult<std::sync::MutexGuard<'_, Connection>> {
    state.db.lock().map_err(|e| AppError::Db(e.to_string()))
}

fn allow_dir(app: &AppHandle, path: &str) {
    let _ = app.asset_protocol_scope().allow_directory(path, true);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddSourceResult {
    pub source: Source,
    pub stats: ScanStats,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelPage {
    pub channel: Channel,
    pub videos: Page<VideoCard>,
}

#[tauri::command]
pub fn pick_folder(app: AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .blocking_pick_folder()
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn add_source(app: AppHandle, state: State<AppState>, path: String) -> AppResult<AddSourceResult> {
    let db = lock(&state)?;
    let source = catalog::add_source(&db, path.as_ref())?;
    let stats = scan_source(&db, source.id, &state.thumbs_dir)?;
    drop(db);
    allow_dir(&app, &source.path);
    Ok(AddSourceResult { source, stats })
}

#[tauri::command]
pub fn remove_source(state: State<AppState>, id: i64) -> AppResult<()> {
    let db = lock(&state)?;
    catalog::remove_source(&db, id)
}

#[tauri::command]
pub fn list_sources(state: State<AppState>) -> AppResult<Vec<Source>> {
    let db = lock(&state)?;
    catalog::list_sources(&db)
}

#[tauri::command]
pub fn rescan(state: State<AppState>) -> AppResult<ScanStats> {
    let db = lock(&state)?;
    rescan_all(&db, &state.thumbs_dir)
}

#[tauri::command]
pub fn list_home(state: State<AppState>, page: i64) -> AppResult<Page<VideoCard>> {
    let db = lock(&state)?;
    catalog::list_home(&db, page)
}

#[tauri::command]
pub fn search(state: State<AppState>, query: String, page: i64) -> AppResult<Page<VideoCard>> {
    let db = lock(&state)?;
    catalog::search(&db, &query, page)
}

#[tauri::command]
pub fn list_channels(state: State<AppState>) -> AppResult<Vec<Channel>> {
    let db = lock(&state)?;
    catalog::list_channels(&db)
}

#[tauri::command]
pub fn get_channel(state: State<AppState>, slug: String, page: i64) -> AppResult<ChannelPage> {
    let db = lock(&state)?;
    let (channel, videos) = catalog::get_channel(&db, &slug, page)?;
    Ok(ChannelPage { channel, videos })
}

#[tauri::command]
pub fn get_video(state: State<AppState>, id: i64) -> AppResult<VideoDetail> {
    let db = lock(&state)?;
    let mut v = catalog::get_video(&db, id)?;
    v.path = crate::pathutil::display_path(std::path::Path::new(&v.path));
    Ok(v)
}

#[tauri::command]
pub fn list_more(state: State<AppState>, video_id: i64) -> AppResult<Vec<VideoCard>> {
    let db = lock(&state)?;
    catalog::list_more(&db, video_id)
}

#[tauri::command]
pub fn video_url(state: State<AppState>, id: i64) -> AppResult<String> {
    let db = lock(&state)?;
    let v = catalog::get_video(&db, id)?;
    let p = std::path::Path::new(&v.path);
    if !p.is_file() {
        return Err(AppError::NotFound(v.path));
    }
    Ok(crate::pathutil::display_path(p))
}

#[tauri::command]
pub fn set_progress(state: State<AppState>, id: i64, position_sec: f64) -> AppResult<()> {
    let db = lock(&state)?;
    catalog::set_progress(&db, id, position_sec)
}

#[tauri::command]
pub fn start_watch(state: State<AppState>, id: i64) -> AppResult<()> {
    let db = lock(&state)?;
    catalog::start_watch(&db, id)
}

#[tauri::command]
pub fn list_history(state: State<AppState>, page: i64) -> AppResult<Page<VideoCard>> {
    let db = lock(&state)?;
    catalog::list_history(&db, page)
}
