use crate::error::AppResult;
use rusqlite::Connection;
use std::path::Path;

pub const PAGE_SIZE: i64 = 24;
pub const MORE_SIZE: i64 = 12;

pub fn now_iso() -> String {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format_unix(t)
}

fn format_unix(secs: u64) -> String {
    let days = secs / 86400;
    let rem = secs % 86400;
    let hour = rem / 3600;
    let min = (rem % 3600) / 60;
    let sec = rem % 60;
    let (y, m, d) = civil_from_days(days as i64);
    format!("{y:04}-{m:02}-{d:02}T{hour:02}:{min:02}:{sec:02}Z")
}

fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

pub fn open(path: &Path) -> AppResult<Connection> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "foreign_keys", 1)?;
    migrate(&conn)?;
    Ok(conn)
}

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.pragma_update(None, "foreign_keys", 1)?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS sources (
            id INTEGER PRIMARY KEY,
            path TEXT UNIQUE NOT NULL,
            added_at TEXT NOT NULL,
            last_scanned_at TEXT,
            available INTEGER NOT NULL DEFAULT 1
        );
        CREATE TABLE IF NOT EXISTS channels (
            id INTEGER PRIMARY KEY,
            slug TEXT UNIQUE NOT NULL,
            name TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS videos (
            id INTEGER PRIMARY KEY,
            path TEXT UNIQUE NOT NULL,
            source_id INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
            channel_id INTEGER REFERENCES channels(id),
            title TEXT NOT NULL,
            description TEXT,
            duration_sec REAL,
            thumbnail_path TEXT,
            upload_date TEXT,
            parent_dir TEXT NOT NULL,
            mtime INTEGER NOT NULL,
            size_bytes INTEGER NOT NULL,
            added_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS watch_progress (
            video_id INTEGER PRIMARY KEY REFERENCES videos(id) ON DELETE CASCADE,
            position_sec REAL NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS watch_history (
            video_id INTEGER PRIMARY KEY REFERENCES videos(id) ON DELETE CASCADE,
            watched_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_videos_source ON videos(source_id);
        CREATE INDEX IF NOT EXISTS idx_videos_channel ON videos(channel_id);
        CREATE INDEX IF NOT EXISTS idx_videos_dates ON videos(upload_date, mtime);
        CREATE INDEX IF NOT EXISTS idx_videos_title ON videos(title);
        CREATE INDEX IF NOT EXISTS idx_history_watched ON watch_history(watched_at);
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn migrate_creates_tables_and_fk() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('sources','channels','videos','watch_progress','watch_history')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 5);

        conn.execute(
            "INSERT INTO sources (path, added_at) VALUES ('C:\\v', 't')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO videos (path, source_id, title, parent_dir, mtime, size_bytes, added_at)
             VALUES ('C:\\v\\a.mp4', 1, 'A', 'C:\\v', 1, 1, 't')",
            [],
        )
        .unwrap();
        conn.execute("DELETE FROM sources WHERE id = 1", []).unwrap();
        let left: i64 = conn
            .query_row("SELECT COUNT(*) FROM videos", [], |r| r.get(0))
            .unwrap();
        assert_eq!(left, 0, "videos must cascade when source is deleted");
    }
}
