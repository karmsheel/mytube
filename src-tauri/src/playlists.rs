use crate::catalog;
use crate::db::{now_iso, PAGE_SIZE};
use crate::error::{AppError, AppResult};
use crate::models::{Page, Playlist, VideoCard};
use rusqlite::{params, Connection};

fn row_playlist(r: &rusqlite::Row) -> rusqlite::Result<Playlist> {
    Ok(Playlist {
        id: r.get(0)?,
        name: r.get(1)?,
        created_at: r.get(2)?,
        video_count: r.get(3)?,
    })
}

pub fn list_playlists(conn: &Connection) -> AppResult<Vec<Playlist>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.created_at,
                (SELECT COUNT(*) FROM playlist_items i WHERE i.playlist_id = p.id) AS n
         FROM playlists p
         ORDER BY p.created_at DESC",
    )?;
    let rows = stmt.query_map([], row_playlist)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn create_playlist(conn: &Connection, name: &str) -> AppResult<Playlist> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::Invalid("Playlist name is required".into()));
    }
    let created = now_iso();
    conn.execute(
        "INSERT INTO playlists (name, created_at) VALUES (?1, ?2)",
        params![name, created],
    )?;
    let id = conn.last_insert_rowid();
    get_playlist_meta(conn, id)
}

pub fn delete_playlist(conn: &Connection, id: i64) -> AppResult<()> {
    let n = conn.execute("DELETE FROM playlists WHERE id = ?1", [id])?;
    if n == 0 {
        return Err(AppError::NotFound(format!("playlist {id}")));
    }
    Ok(())
}

fn get_playlist_meta(conn: &Connection, id: i64) -> AppResult<Playlist> {
    conn.query_row(
        "SELECT p.id, p.name, p.created_at,
                (SELECT COUNT(*) FROM playlist_items i WHERE i.playlist_id = p.id) AS n
         FROM playlists p WHERE p.id = ?1",
        [id],
        row_playlist,
    )
    .map_err(|_| AppError::NotFound(format!("playlist {id}")))
}

pub fn get_playlist(
    conn: &Connection,
    id: i64,
    page: i64,
) -> AppResult<(Playlist, Page<VideoCard>)> {
    let pl = get_playlist_meta(conn, id)?;
    let sql = format!(
        "SELECT v.id, v.title, c.name, c.slug, v.duration_sec, v.thumbnail_path, v.upload_date
         FROM playlist_items i
         JOIN videos v ON v.id = i.video_id
         JOIN sources s ON s.id = v.source_id AND s.available = 1
         LEFT JOIN channels c ON c.id = v.channel_id
         WHERE i.playlist_id = ?1
         ORDER BY i.position ASC, i.added_at ASC
         LIMIT ?2 OFFSET ?3"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![id, PAGE_SIZE, page * PAGE_SIZE], |r| {
        Ok(VideoCard {
            id: r.get(0)?,
            title: r.get(1)?,
            channel_name: r.get(2)?,
            channel_slug: r.get(3)?,
            duration_sec: r.get(4)?,
            thumbnail_path: r.get(5)?,
            upload_date: r.get(6)?,
        })
    })?;
    let items = rows.collect::<Result<Vec<_>, _>>()?;
    Ok((
        pl.clone(),
        Page {
            items,
            page,
            page_size: PAGE_SIZE,
            total: pl.video_count,
        },
    ))
}

pub fn add_to_playlist(conn: &Connection, playlist_id: i64, video_id: i64) -> AppResult<Playlist> {
    let _ = get_playlist_meta(conn, playlist_id)?;
    catalog::get_video(conn, video_id)?;
    let next: i64 = conn.query_row(
        "SELECT COALESCE(MAX(position), -1) + 1 FROM playlist_items WHERE playlist_id = ?1",
        [playlist_id],
        |r| r.get(0),
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO playlist_items (playlist_id, video_id, position, added_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![playlist_id, video_id, next, now_iso()],
    )?;
    get_playlist_meta(conn, playlist_id)
}

pub fn remove_from_playlist(
    conn: &Connection,
    playlist_id: i64,
    video_id: i64,
) -> AppResult<Playlist> {
    let n = conn.execute(
        "DELETE FROM playlist_items WHERE playlist_id = ?1 AND video_id = ?2",
        params![playlist_id, video_id],
    )?;
    if n == 0 {
        return Err(AppError::NotFound("video not in playlist".into()));
    }
    get_playlist_meta(conn, playlist_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrate;
    use rusqlite::Connection;

    fn mem() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        migrate(&c).unwrap();
        c.execute(
            "INSERT INTO sources (path, added_at) VALUES ('C:\\v', 't')",
            [],
        )
        .unwrap();
        c.execute(
            "INSERT INTO videos (path, source_id, title, parent_dir, mtime, size_bytes, added_at)
             VALUES ('C:\\v\\a.mp4', 1, 'A', 'C:\\v', 1, 1, 't')",
            [],
        )
        .unwrap();
        c.execute(
            "INSERT INTO videos (path, source_id, title, parent_dir, mtime, size_bytes, added_at)
             VALUES ('C:\\v\\b.mp4', 1, 'B', 'C:\\v', 1, 1, 't')",
            [],
        )
        .unwrap();
        c
    }

    #[test]
    fn create_and_add() {
        let conn = mem();
        let p = create_playlist(&conn, "  Mix  ").unwrap();
        assert_eq!(p.name, "Mix");
        assert_eq!(p.video_count, 0);
        add_to_playlist(&conn, p.id, 1).unwrap();
        add_to_playlist(&conn, p.id, 2).unwrap();
        add_to_playlist(&conn, p.id, 1).unwrap();
        let (pl, page) = get_playlist(&conn, p.id, 0).unwrap();
        assert_eq!(pl.video_count, 2);
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.items[0].title, "A");
        assert_eq!(page.items[1].title, "B");
    }

    #[test]
    fn empty_name_rejected() {
        let conn = mem();
        let err = create_playlist(&conn, "   ").unwrap_err();
        assert!(matches!(err, AppError::Invalid(_)));
    }

    #[test]
    fn remove_and_delete() {
        let conn = mem();
        let p = create_playlist(&conn, "Mix").unwrap();
        add_to_playlist(&conn, p.id, 1).unwrap();
        remove_from_playlist(&conn, p.id, 1).unwrap();
        let (pl, page) = get_playlist(&conn, p.id, 0).unwrap();
        assert_eq!(pl.video_count, 0);
        assert!(page.items.is_empty());
        delete_playlist(&conn, p.id).unwrap();
        assert!(list_playlists(&conn).unwrap().is_empty());
    }
}
