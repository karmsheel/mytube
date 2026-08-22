use crate::catalog;
use crate::error::{AppError, AppResult};
use crate::models::{Channel, Page, Playlist, ScanStats, Source, VideoCard, VideoDetail};
use crate::playlists;
use crate::scan::{apply_source_scan, plan_source_scan};
use rusqlite::Connection;
use serde::Serialize;
use std::path::{Path, PathBuf};
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
pub async fn add_source(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> AppResult<AddSourceResult> {
    let source = {
        let db = lock(&state)?;
        catalog::add_source(&db, path.as_ref())?
    };
    let stats = scan_source_releasing(&state, source.id)?;
    allow_dir(&app, &source.path);
    Ok(AddSourceResult { source, stats })
}

#[tauri::command]
pub async fn remove_source(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    let db = lock(&state)?;
    catalog::remove_source(&db, id)
}

#[tauri::command]
pub async fn list_sources(state: State<'_, AppState>) -> AppResult<Vec<Source>> {
    let db = lock(&state)?;
    catalog::list_sources(&db)
}

fn scan_source_releasing(state: &AppState, source_id: i64) -> AppResult<ScanStats> {
    let (root, fps) = {
        let db = lock(state)?;
        let src = catalog::get_source(&db, source_id)?;
        let fps = catalog::source_fingerprints(&db, source_id)?;
        (src.path, fps)
    };
    let plan = plan_source_scan(Path::new(&root), &fps, &state.thumbs_dir);
    let db = lock(state)?;
    apply_source_scan(&db, source_id, plan)
}

#[tauri::command]
pub async fn rescan(state: State<'_, AppState>) -> AppResult<ScanStats> {
    let sources = {
        let db = lock(&state)?;
        catalog::list_sources(&db)?
    };
    let mut total = ScanStats::default();
    for src in sources {
        let s = scan_source_releasing(&state, src.id)?;
        total.imported += s.imported;
        total.updated += s.updated;
        total.removed += s.removed;
        total.skipped_dirs += s.skipped_dirs;
    }
    Ok(total)
}

#[tauri::command]
pub async fn list_home(state: State<'_, AppState>, page: i64) -> AppResult<Page<VideoCard>> {
    let db = lock(&state)?;
    catalog::list_home(&db, page)
}

#[tauri::command]
pub async fn search(
    state: State<'_, AppState>,
    query: String,
    page: i64,
) -> AppResult<Page<VideoCard>> {
    let db = lock(&state)?;
    catalog::search(&db, &query, page)
}

#[tauri::command]
pub async fn list_channels(state: State<'_, AppState>) -> AppResult<Vec<Channel>> {
    let db = lock(&state)?;
    catalog::list_channels(&db)
}

#[tauri::command]
pub async fn get_channel(
    state: State<'_, AppState>,
    slug: String,
    page: i64,
) -> AppResult<ChannelPage> {
    let db = lock(&state)?;
    let (channel, videos) = catalog::get_channel(&db, &slug, page)?;
    Ok(ChannelPage { channel, videos })
}

#[tauri::command]
pub async fn get_video(state: State<'_, AppState>, id: i64) -> AppResult<VideoDetail> {
    let db = lock(&state)?;
    let mut v = catalog::get_video(&db, id)?;
    v.path = crate::pathutil::display_path(std::path::Path::new(&v.path));
    Ok(v)
}

#[tauri::command]
pub async fn list_more(state: State<'_, AppState>, video_id: i64) -> AppResult<Vec<VideoCard>> {
    let db = lock(&state)?;
    catalog::list_more(&db, video_id)
}

#[tauri::command]
pub async fn video_url(state: State<'_, AppState>, id: i64) -> AppResult<String> {
    let db = lock(&state)?;
    let v = catalog::get_video(&db, id)?;
    let p = std::path::Path::new(&v.path);
    if !p.is_file() {
        return Err(AppError::NotFound(v.path));
    }
    Ok(crate::pathutil::display_path(p))
}

#[tauri::command]
pub async fn set_progress(
    state: State<'_, AppState>,
    id: i64,
    position_sec: f64,
) -> AppResult<()> {
    let db = lock(&state)?;
    catalog::set_progress(&db, id, position_sec)
}

#[tauri::command]
pub async fn start_watch(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    let db = lock(&state)?;
    catalog::start_watch(&db, id)
}

#[tauri::command]
pub async fn list_history(state: State<'_, AppState>, page: i64) -> AppResult<Page<VideoCard>> {
    let db = lock(&state)?;
    catalog::list_history(&db, page)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistPage {
    pub playlist: Playlist,
    pub videos: Page<VideoCard>,
}

#[tauri::command]
pub async fn list_playlists(state: State<'_, AppState>) -> AppResult<Vec<Playlist>> {
    let db = lock(&state)?;
    playlists::list_playlists(&db)
}

#[tauri::command]
pub async fn create_playlist(state: State<'_, AppState>, name: String) -> AppResult<Playlist> {
    let db = lock(&state)?;
    playlists::create_playlist(&db, &name)
}

#[tauri::command]
pub async fn delete_playlist(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    let db = lock(&state)?;
    playlists::delete_playlist(&db, id)
}

#[tauri::command]
pub async fn get_playlist(
    state: State<'_, AppState>,
    id: i64,
    page: i64,
) -> AppResult<PlaylistPage> {
    let db = lock(&state)?;
    let (playlist, videos) = playlists::get_playlist(&db, id, page)?;
    Ok(PlaylistPage { playlist, videos })
}

#[tauri::command]
pub async fn add_to_playlist(
    state: State<'_, AppState>,
    playlist_id: i64,
    video_id: i64,
) -> AppResult<Playlist> {
    let db = lock(&state)?;
    playlists::add_to_playlist(&db, playlist_id, video_id)
}

#[tauri::command]
pub async fn remove_from_playlist(
    state: State<'_, AppState>,
    playlist_id: i64,
    video_id: i64,
) -> AppResult<Playlist> {
    let db = lock(&state)?;
    playlists::remove_from_playlist(&db, playlist_id, video_id)
}
