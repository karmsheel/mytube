use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: i64,
    pub path: String,
    pub added_at: String,
    pub last_scanned_at: Option<String>,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub video_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoCard {
    pub id: i64,
    pub title: String,
    pub channel_name: Option<String>,
    pub channel_slug: Option<String>,
    pub duration_sec: Option<f64>,
    pub thumbnail_path: Option<String>,
    pub upload_date: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoDetail {
    pub id: i64,
    pub title: String,
    pub channel_name: Option<String>,
    pub channel_slug: Option<String>,
    pub channel_id: Option<i64>,
    pub source_id: i64,
    pub path: String,
    pub parent_dir: String,
    pub description: Option<String>,
    pub duration_sec: Option<f64>,
    pub thumbnail_path: Option<String>,
    pub upload_date: Option<String>,
    pub progress_sec: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T: Serialize> {
    pub items: Vec<T>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStats {
    pub imported: i64,
    pub updated: i64,
    pub removed: i64,
    pub skipped_dirs: i64,
}
