use crate::db::{now_iso, MORE_SIZE, PAGE_SIZE};
use crate::error::{AppError, AppResult};
use crate::metadata::ResolvedMeta;
use crate::models::{Channel, Page, Source, VideoCard, VideoDetail};
use crate::pathutil::{normalize_path, paths_equal, source_overlap};
use crate::slug::channel_slug;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

pub struct UpsertOutcome {
    pub id: i64,
    pub created: bool,
}

pub fn add_source(conn: &Connection, path: &Path) -> AppResult<Source> {
    let norm = normalize_path(path)?;
    if !norm.is_dir() {
        return Err(AppError::Invalid("Folder does not exist".into()));
    }
    for existing in list_sources(conn)? {
        let ep = Path::new(&existing.path);
        if let Some(kind) = source_overlap(ep, &norm) {
            return Err(AppError::Overlap {
                reason: kind.reason(),
            });
        }
    }
    let added = now_iso();
    conn.execute(
        "INSERT INTO sources (path, added_at, available) VALUES (?1, ?2, 1)",
        params![norm.to_string_lossy(), added],
    )?;
    let id = conn.last_insert_rowid();
    get_source(conn, id)
}

pub fn get_source(conn: &Connection, id: i64) -> AppResult<Source> {
    conn.query_row(
        "SELECT id, path, added_at, last_scanned_at, available FROM sources WHERE id = ?1",
        [id],
        row_source,
    )
    .map_err(|_| AppError::NotFound(format!("source {id}")))
}

pub fn list_sources(conn: &Connection) -> AppResult<Vec<Source>> {
    let mut stmt = conn.prepare(
        "SELECT id, path, added_at, last_scanned_at, available FROM sources ORDER BY id",
    )?;
    let rows = stmt.query_map([], row_source)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn remove_source(conn: &Connection, id: i64) -> AppResult<()> {
    let n = conn.execute("DELETE FROM sources WHERE id = ?1", [id])?;
    if n == 0 {
        return Err(AppError::NotFound(format!("source {id}")));
    }
    prune_empty_channels(conn)
}

pub fn set_source_available(
    conn: &Connection,
    id: i64,
    available: bool,
    last_scanned: Option<&str>,
) -> AppResult<()> {
    conn.execute(
        "UPDATE sources SET available = ?1, last_scanned_at = COALESCE(?2, last_scanned_at) WHERE id = ?3",
        params![available as i64, last_scanned, id],
    )?;
    Ok(())
}

pub fn prune_empty_channels(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "DELETE FROM channels WHERE id NOT IN (SELECT DISTINCT channel_id FROM videos WHERE channel_id IS NOT NULL)",
        [],
    )?;
    Ok(())
}

fn get_or_create_channel(conn: &Connection, name: &str) -> AppResult<i64> {
    let slug = channel_slug(name);
    conn.execute(
        "INSERT OR IGNORE INTO channels (slug, name) VALUES (?1, ?2)",
        params![slug, name],
    )?;
    let id = conn.query_row(
        "SELECT id FROM channels WHERE slug = ?1",
        [slug],
        |r| r.get(0),
    )?;
    Ok(id)
}

fn find_video(conn: &Connection, path: &Path) -> AppResult<Option<(i64, i64)>> {
    let norm = normalize_path(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned();
    if let Some(pair) = conn
        .query_row(
            "SELECT id, source_id FROM videos WHERE path = ?1",
            [&norm],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
    {
        return Ok(Some(pair));
    }
    let mut stmt = conn.prepare("SELECT id, source_id, path FROM videos")?;
    let rows = stmt.query_map([], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?))
    })?;
    for row in rows {
        let (id, sid, p) = row?;
        if paths_equal(Path::new(&p), path) {
            return Ok(Some((id, sid)));
        }
    }
    Ok(None)
}

pub fn upsert_video(
    conn: &Connection,
    source_id: i64,
    path: &Path,
    meta: &ResolvedMeta,
    parent_dir: &str,
    mtime: i64,
    size: i64,
) -> AppResult<UpsertOutcome> {
    let stored = normalize_path(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned();
    let channel_id = match &meta.channel_name {
        Some(n) if !n.is_empty() => Some(get_or_create_channel(conn, n)?),
        _ => None,
    };
    let thumb = meta
        .thumbnail_path
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned());
    if let Some((id, existing_source)) = find_video(conn, path)? {
        if existing_source != source_id {
            return Ok(UpsertOutcome { id, created: false });
        }
        conn.execute(
            "UPDATE videos SET title=?1, channel_id=?2, description=?3, duration_sec=?4,
             thumbnail_path=?5, upload_date=?6, parent_dir=?7, mtime=?8, size_bytes=?9
             WHERE id=?10",
            params![
                meta.title,
                channel_id,
                meta.description,
                meta.duration_sec,
                thumb,
                meta.upload_date,
                parent_dir,
                mtime,
                size,
                id
            ],
        )?;
        return Ok(UpsertOutcome { id, created: false });
    }
    conn.execute(
        "INSERT INTO videos (path, source_id, channel_id, title, description, duration_sec,
         thumbnail_path, upload_date, parent_dir, mtime, size_bytes, added_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![
            stored,
            source_id,
            channel_id,
            meta.title,
            meta.description,
            meta.duration_sec,
            thumb,
            meta.upload_date,
            parent_dir,
            mtime,
            size,
            now_iso()
        ],
    )?;
    Ok(UpsertOutcome {
        id: conn.last_insert_rowid(),
        created: true,
    })
}

pub fn delete_videos_not_in(
    conn: &Connection,
    source_id: i64,
    keep_paths: &[String],
) -> AppResult<i64> {
    if keep_paths.is_empty() {
        let n = conn.execute("DELETE FROM videos WHERE source_id = ?1", [source_id])?;
        prune_empty_channels(conn)?;
        return Ok(n as i64);
    }
    let mut keep = std::collections::HashSet::new();
    for p in keep_paths {
        keep.insert(p.clone());
    }
    let mut stmt = conn.prepare("SELECT id, path FROM videos WHERE source_id = ?1")?;
    let rows = stmt.query_map([source_id], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
    })?;
    let mut gone = Vec::new();
    for row in rows {
        let (id, path) = row?;
        let still = keep.iter().any(|k| paths_equal(Path::new(k), Path::new(&path)));
        if !still {
            gone.push(id);
        }
    }
    for id in &gone {
        conn.execute("DELETE FROM videos WHERE id = ?1", [*id])?;
    }
    prune_empty_channels(conn)?;
    Ok(gone.len() as i64)
}

const CARD_SELECT: &str = "SELECT v.id, v.title, c.name, c.slug, v.duration_sec, v.thumbnail_path, v.upload_date
    FROM videos v
    JOIN sources s ON s.id = v.source_id AND s.available = 1
    LEFT JOIN channels c ON c.id = v.channel_id";

const ORDER: &str = " ORDER BY (v.upload_date IS NULL) ASC, v.upload_date DESC, v.mtime DESC";

fn row_card(r: &rusqlite::Row) -> rusqlite::Result<VideoCard> {
    Ok(VideoCard {
        id: r.get(0)?,
        title: r.get(1)?,
        channel_name: r.get(2)?,
        channel_slug: r.get(3)?,
        duration_sec: r.get(4)?,
        thumbnail_path: r.get(5)?,
        upload_date: r.get(6)?,
    })
}

fn row_source(r: &rusqlite::Row) -> rusqlite::Result<Source> {
    Ok(Source {
        id: r.get(0)?,
        path: r.get(1)?,
        added_at: r.get(2)?,
        last_scanned_at: r.get(3)?,
        available: r.get::<_, i64>(4)? != 0,
    })
}

fn page_of<T: serde::Serialize>(
    items: Vec<T>,
    page: i64,
    total: i64,
    page_size: i64,
) -> Page<T> {
    Page {
        items,
        page,
        page_size,
        total,
    }
}

pub fn list_home(conn: &Connection, page: i64) -> AppResult<Page<VideoCard>> {
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM videos v JOIN sources s ON s.id = v.source_id AND s.available = 1",
        [],
        |r| r.get(0),
    )?;
    let sql = format!("{CARD_SELECT}{ORDER} LIMIT ?1 OFFSET ?2");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![PAGE_SIZE, page * PAGE_SIZE], row_card)?;
    Ok(page_of(rows.collect::<Result<Vec<_>, _>>()?, page, total, PAGE_SIZE))
}

fn like_pattern(q: &str) -> String {
    let mut s = String::from("%");
    for ch in q.chars() {
        match ch {
            '%' | '_' | '\\' => {
                s.push('\\');
                s.push(ch);
            }
            _ => s.push(ch),
        }
    }
    s.push('%');
    s
}

pub fn search(conn: &Connection, query: &str, page: i64) -> AppResult<Page<VideoCard>> {
    let pat = like_pattern(query.trim());
    let where_sql = " WHERE (v.title LIKE ?1 ESCAPE '\\' OR IFNULL(v.description,'') LIKE ?1 ESCAPE '\\' OR IFNULL(c.name,'') LIKE ?1 ESCAPE '\\')";
    let total: i64 = conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM videos v JOIN sources s ON s.id = v.source_id AND s.available = 1 LEFT JOIN channels c ON c.id = v.channel_id{where_sql}"
        ),
        [&pat],
        |r| r.get(0),
    )?;
    let sql = format!("{CARD_SELECT}{where_sql}{ORDER} LIMIT ?2 OFFSET ?3");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![pat, PAGE_SIZE, page * PAGE_SIZE], row_card)?;
    Ok(page_of(rows.collect::<Result<Vec<_>, _>>()?, page, total, PAGE_SIZE))
}

pub fn list_channels(conn: &Connection) -> AppResult<Vec<Channel>> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.slug, c.name, COUNT(v.id) FROM channels c
         JOIN videos v ON v.channel_id = c.id
         JOIN sources s ON s.id = v.source_id AND s.available = 1
         GROUP BY c.id
         ORDER BY c.name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(Channel {
            id: r.get(0)?,
            slug: r.get(1)?,
            name: r.get(2)?,
            video_count: r.get(3)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn get_channel(conn: &Connection, slug: &str, page: i64) -> AppResult<(Channel, Page<VideoCard>)> {
    let ch = conn
        .query_row(
            "SELECT c.id, c.slug, c.name,
                (SELECT COUNT(*) FROM videos v JOIN sources s ON s.id = v.source_id AND s.available = 1 WHERE v.channel_id = c.id)
             FROM channels c WHERE c.slug = ?1",
            [slug],
            |r| {
                Ok(Channel {
                    id: r.get(0)?,
                    slug: r.get(1)?,
                    name: r.get(2)?,
                    video_count: r.get(3)?,
                })
            },
        )
        .map_err(|_| AppError::NotFound(format!("channel {slug}")))?;
    let sql = format!("{CARD_SELECT} WHERE v.channel_id = ?1{ORDER} LIMIT ?2 OFFSET ?3");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![ch.id, PAGE_SIZE, page * PAGE_SIZE], row_card)?;
    let items = rows.collect::<Result<Vec<_>, _>>()?;
    Ok((ch.clone(), page_of(items, page, ch.video_count, PAGE_SIZE)))
}

pub fn get_video(conn: &Connection, id: i64) -> AppResult<VideoDetail> {
    conn.query_row(
        "SELECT v.id, v.title, c.name, c.slug, v.channel_id, v.source_id, v.path, v.parent_dir,
                v.description, v.duration_sec, v.thumbnail_path, v.upload_date, p.position_sec
         FROM videos v
         LEFT JOIN channels c ON c.id = v.channel_id
         LEFT JOIN watch_progress p ON p.video_id = v.id
         WHERE v.id = ?1",
        [id],
        |r| {
            Ok(VideoDetail {
                id: r.get(0)?,
                title: r.get(1)?,
                channel_name: r.get(2)?,
                channel_slug: r.get(3)?,
                channel_id: r.get(4)?,
                source_id: r.get(5)?,
                path: r.get(6)?,
                parent_dir: r.get(7)?,
                description: r.get(8)?,
                duration_sec: r.get(9)?,
                thumbnail_path: r.get(10)?,
                upload_date: r.get(11)?,
                progress_sec: r.get(12)?,
            })
        },
    )
    .map_err(|_| AppError::NotFound(format!("video {id}")))
}

pub fn list_more(conn: &Connection, video_id: i64) -> AppResult<Vec<VideoCard>> {
    let v = get_video(conn, video_id)?;
    if let Some(cid) = v.channel_id {
        let sql = format!(
            "{CARD_SELECT} WHERE v.id != ?1 AND v.channel_id = ?2{ORDER} LIMIT ?3"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![video_id, cid, MORE_SIZE], row_card)?;
        return Ok(rows.collect::<Result<Vec<_>, _>>()?);
    }
    let sql = format!(
        "{CARD_SELECT} WHERE v.id != ?1 AND v.parent_dir = ?2{ORDER} LIMIT ?3"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![video_id, v.parent_dir, MORE_SIZE], row_card)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn set_progress(conn: &Connection, video_id: i64, position_sec: f64) -> AppResult<()> {
    conn.execute(
        "INSERT INTO watch_progress (video_id, position_sec, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(video_id) DO UPDATE SET position_sec=excluded.position_sec, updated_at=excluded.updated_at",
        params![video_id, position_sec, now_iso()],
    )?;
    Ok(())
}

pub fn start_watch(conn: &Connection, video_id: i64) -> AppResult<()> {
    conn.execute(
        "INSERT INTO watch_history (video_id, watched_at) VALUES (?1, ?2)
         ON CONFLICT(video_id) DO UPDATE SET watched_at=excluded.watched_at",
        params![video_id, now_iso()],
    )?;
    Ok(())
}

pub fn list_history(conn: &Connection, page: i64) -> AppResult<Page<VideoCard>> {
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM watch_history h
         JOIN videos v ON v.id = h.video_id
         JOIN sources s ON s.id = v.source_id AND s.available = 1",
        [],
        |r| r.get(0),
    )?;
    let sql = format!(
        "{CARD_SELECT}
         JOIN watch_history h ON h.video_id = v.id
         ORDER BY h.watched_at DESC LIMIT ?1 OFFSET ?2"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![PAGE_SIZE, page * PAGE_SIZE], row_card)?;
    Ok(page_of(rows.collect::<Result<Vec<_>, _>>()?, page, total, PAGE_SIZE))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrate;
    use crate::error::AppError;
    use crate::metadata::ResolvedMeta;
    use rusqlite::Connection;
    use std::path::{Path, PathBuf};

    fn mem() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        migrate(&c).unwrap();
        c
    }

    fn tmpdir() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "mytube-cat-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn meta(title: &str, channel: Option<&str>, upload: Option<&str>) -> ResolvedMeta {
        ResolvedMeta {
            title: title.into(),
            channel_name: channel.map(|s| s.into()),
            description: Some("d".into()),
            duration_sec: Some(60.0),
            thumbnail_path: None,
            upload_date: upload.map(|s| s.into()),
        }
    }

    fn seed_one(conn: &Connection) -> (i64, i64) {
        let root = tmpdir();
        std::fs::create_dir_all(&root).unwrap();
        let src = add_source(conn, &root).unwrap();
        let file = root.join("a.mp4");
        let out = upsert_video(
            conn,
            src.id,
            &file,
            &meta("Alpha", Some("Veritasium"), Some("2024-01-01")),
            root.to_string_lossy().as_ref(),
            100,
            1,
        )
        .unwrap();
        (src.id, out.id)
    }

    #[test]
    fn rejects_overlapping_sources() {
        let conn = mem();
        let root = tmpdir();
        let a = root.join("A");
        let nested = a.join("N");
        std::fs::create_dir_all(&nested).unwrap();
        add_source(&conn, &a).unwrap();
        let err = add_source(&conn, &nested).unwrap_err();
        assert!(matches!(err, AppError::Overlap { .. }));
        let err = add_source(&conn, &a).unwrap_err();
        assert!(matches!(err, AppError::Overlap { .. }));
        assert_eq!(list_sources(&conn).unwrap().len(), 1);
    }

    #[test]
    fn upsert_updates_same_path() {
        let conn = mem();
        let root = tmpdir();
        std::fs::create_dir_all(&root).unwrap();
        let src = add_source(&conn, &root).unwrap();
        let file = root.join("a.mp4");
        upsert_video(&conn, src.id, &file, &meta("Old", None, None), "p", 1, 1).unwrap();
        upsert_video(&conn, src.id, &file, &meta("New", None, None), "p", 2, 2).unwrap();
        let home = list_home(&conn, 0).unwrap();
        assert_eq!(home.total, 1);
        assert_eq!(home.items[0].title, "New");
    }

    #[test]
    fn delete_videos_not_in_removes_missing() {
        let conn = mem();
        let (_sid, vid) = seed_one(&conn);
        let n = delete_videos_not_in(&conn, _sid, &[]).unwrap();
        assert_eq!(n, 1);
        assert!(get_video(&conn, vid).is_err());
    }

    #[test]
    fn remove_source_cascades_and_prunes_channel() {
        let conn = mem();
        let (sid, _vid) = seed_one(&conn);
        assert_eq!(list_channels(&conn).unwrap().len(), 1);
        remove_source(&conn, sid).unwrap();
        assert!(list_sources(&conn).unwrap().is_empty());
        assert_eq!(list_channels(&conn).unwrap().len(), 0);
    }

    #[test]
    fn first_source_wins_on_duplicate_path() {
        let conn = mem();
        let a = tmpdir();
        let b = tmpdir();
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        let sa = add_source(&conn, &a).unwrap();
        let sb = add_source(&conn, &b).unwrap();
        let file = a.join("x.mp4");
        let first = upsert_video(&conn, sa.id, &file, &meta("A", None, None), "p", 1, 1).unwrap();
        let second = upsert_video(&conn, sb.id, &file, &meta("B", None, None), "p", 1, 1).unwrap();
        assert_eq!(first.id, second.id);
        assert!(!second.created);
        assert_eq!(get_video(&conn, first.id).unwrap().title, "A");
    }

    #[test]
    fn unavailable_hides_but_keeps_rows() {
        let conn = mem();
        let (sid, vid) = seed_one(&conn);
        set_source_available(&conn, sid, false, None).unwrap();
        assert_eq!(list_home(&conn, 0).unwrap().total, 0);
        assert_eq!(search(&conn, "Alpha", 0).unwrap().total, 0);
        assert!(list_channels(&conn).unwrap().is_empty());
        assert!(get_video(&conn, vid).is_ok());
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM videos", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn home_orders_upload_date_then_mtime() {
        let conn = mem();
        let root = tmpdir();
        std::fs::create_dir_all(&root).unwrap();
        let src = add_source(&conn, &root).unwrap();
        upsert_video(
            &conn,
            src.id,
            &root.join("old.mp4"),
            &meta("DatedOld", None, Some("2020-01-01")),
            "p",
            999,
            1,
        )
        .unwrap();
        upsert_video(
            &conn,
            src.id,
            &root.join("new.mp4"),
            &meta("DatedNew", None, Some("2024-01-01")),
            "p",
            1,
            1,
        )
        .unwrap();
        upsert_video(
            &conn,
            src.id,
            &root.join("nodate.mp4"),
            &meta("NoDate", None, None),
            "p",
            5000,
            1,
        )
        .unwrap();
        let home = list_home(&conn, 0).unwrap();
        assert_eq!(home.items[0].title, "DatedNew");
        assert_eq!(home.items[1].title, "DatedOld");
        assert_eq!(home.items[2].title, "NoDate");
        assert_eq!(home.page_size, 24);
    }
}
