use crate::catalog::{
    delete_videos_not_in, get_source, list_sources, set_source_available, upsert_video,
};
use crate::db::now_iso;
use crate::error::AppResult;
use crate::ffmpeg::Ffmpeg;
use crate::metadata::{is_video_file, resolve};
use crate::models::ScanStats;
use crate::pathutil::normalize_path;
use rusqlite::Connection;
use std::path::Path;
use walkdir::WalkDir;

pub fn scan_source(conn: &Connection, source_id: i64, thumbs_dir: &Path) -> AppResult<ScanStats> {
    let src = get_source(conn, source_id)?;
    let root = Path::new(&src.path);
    if !root.is_dir() {
        set_source_available(conn, source_id, false, None)?;
        return Ok(ScanStats::default());
    }
    let ffmpeg = Ffmpeg::detect();
    let mut stats = ScanStats::default();
    let mut keep = Vec::new();
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => {
                stats.skipped_dirs += 1;
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !is_video_file(path) {
            continue;
        }
        let norm = normalize_path(path).unwrap_or_else(|_| path.to_path_buf());
        let meta = resolve(&norm, root, thumbs_dir, ffmpeg);
        let parent = norm
            .parent()
            .unwrap_or(root)
            .to_string_lossy()
            .into_owned();
        let file_meta = entry.metadata().ok();
        let mtime = file_meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let size = file_meta.map(|m| m.len() as i64).unwrap_or(0);
        match upsert_video(conn, source_id, &norm, &meta, &parent, mtime, size) {
            Ok(out) => {
                if out.created {
                    stats.imported += 1;
                } else {
                    stats.updated += 1;
                }
            }
            Err(_) => {
                stats.skipped_dirs += 1;
            }
        }
        keep.push(norm.to_string_lossy().into_owned());
    }
    stats.removed = delete_videos_not_in(conn, source_id, &keep)?;
    set_source_available(conn, source_id, true, Some(&now_iso()))?;
    Ok(stats)
}

pub fn rescan_all(conn: &Connection, thumbs_dir: &Path) -> AppResult<ScanStats> {
    let mut total = ScanStats::default();
    for src in list_sources(conn)? {
        let s = scan_source(conn, src.id, thumbs_dir)?;
        total.imported += s.imported;
        total.updated += s.updated;
        total.removed += s.removed;
        total.skipped_dirs += s.skipped_dirs;
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{add_source, list_channels, list_home};
    use crate::db::migrate;
    use rusqlite::Connection;
    use std::fs;
    use std::path::PathBuf;

    fn mem() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        migrate(&c).unwrap();
        c
    }

    fn lib() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "mytube-scan-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(p.join("Channel")).unwrap();
        fs::write(p.join("Channel").join("a.mp4"), b"x").unwrap();
        fs::write(
            p.join("Channel").join("a.info.json"),
            r#"{"title":"Sidecar Title","channel":"Channel"}"#,
        )
        .unwrap();
        fs::write(p.join("skip.txt"), b"no").unwrap();
        fs::write(p.join("flat.webm"), b"x").unwrap();
        p
    }

    #[test]
    fn scan_imports_sidecars_and_ignores_txt() {
        let conn = mem();
        let root = lib();
        let src = add_source(&conn, &root).unwrap();
        let thumbs = root.join("thumbs");
        let stats = scan_source(&conn, src.id, &thumbs).unwrap();
        assert_eq!(stats.imported, 2);
        let home = list_home(&conn, 0).unwrap();
        assert_eq!(home.total, 2);
        assert!(home.items.iter().any(|v| v.title == "Sidecar Title"));
        assert!(home.items.iter().any(|v| v.title == "flat"));
        let ch = list_channels(&conn).unwrap();
        assert_eq!(ch.len(), 1);
        assert_eq!(ch[0].name, "Channel");
        assert!(home
            .items
            .iter()
            .find(|v| v.title == "flat")
            .unwrap()
            .channel_name
            .is_none());
    }

    #[test]
    fn rescan_deletes_missing_file() {
        let conn = mem();
        let root = lib();
        let src = add_source(&conn, &root).unwrap();
        let thumbs = root.join("thumbs");
        scan_source(&conn, src.id, &thumbs).unwrap();
        fs::remove_file(root.join("Channel").join("a.mp4")).unwrap();
        let stats = scan_source(&conn, src.id, &thumbs).unwrap();
        assert_eq!(stats.removed, 1);
        assert_eq!(list_home(&conn, 0).unwrap().total, 1);
        assert_eq!(list_home(&conn, 0).unwrap().items[0].title, "flat");
    }

    #[test]
    fn missing_folder_marks_unavailable_keeps_rows() {
        let conn = mem();
        let root = lib();
        let src = add_source(&conn, &root).unwrap();
        let thumbs = root.join("thumbs");
        scan_source(&conn, src.id, &thumbs).unwrap();
        let gone = root.with_extension("moved");
        fs::rename(&root, &gone).unwrap();
        scan_source(&conn, src.id, &thumbs).unwrap();
        assert_eq!(list_home(&conn, 0).unwrap().total, 0);
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM videos", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 2);
        let avail: i64 = conn
            .query_row("SELECT available FROM sources WHERE id = ?1", [src.id], |r| r.get(0))
            .unwrap();
        assert_eq!(avail, 0);
    }
}
