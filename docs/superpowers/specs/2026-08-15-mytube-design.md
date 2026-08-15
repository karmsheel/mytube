# Mytube v1 Design

Local desktop app for watching video files the user already has. It looks like YouTube (sidebar, Home grid, watch page) without an algorithm, ads, accounts, or network.

**Stack:** Tauri 2 + React + TypeScript + Vite + SQLite (Rust, `rusqlite`).  
**Platform:** Windows first (WebView2).  
**Date:** 2026-08-15

## Problem

Opening a folder of downloads in Explorer and double-clicking files in VLC works, but it is not a library. Downloaded YouTube videos often already have titles, channels, descriptions, and thumbnails sitting next to the file. Mytube turns those folders into a personal archive you can browse and resume.

## Goals

- Point the app at one or more local folders (“sources”) and catalog every supported video.
- Browse Home, search, open channel pages, watch, resume, and see history.
- Prefer yt-dlp companion files (`.info.json`, sidecar images) when present.
- Stay entirely offline. No recommendations, no ads, no telemetry.

## Non-goals (v1)

- Downloading from YouTube or anywhere else (explicit follow-up).
- Playlists, Liked, Watch later, up-next queue.
- Live folder watching (rescan on launch + manual button only).
- Bundling ffmpeg.
- Playing every container/codec. MP4 (H.264) and WebM are the supported path; MKV/HEVC may fail with a visible error.
- Multi-user, cloud sync, comments, subscriptions, miniplayer.
- Matching a file across move/rename (identity is the path).

## Key decisions

| Decision | Choice | Why |
|---|---|---|
| Product | Personal archive + Explorer/VLC replacement | User jobs A + C |
| Download | Not in v1 | Second product; library/player first |
| Shell | Tauri 2 desktop window, web UI inside | Local app + YouTube-shaped UI, light vs Electron |
| Catalog | SQLite in app data | Search, history, resume; UI never walks disk |
| Grouping | Hybrid: Home is everything; channels when we have a name | Works for sidecar-rich downloads and bare folders |
| UI | YouTube bones, Mytube skin | Muscle memory without cloning youtube.com |
| Metadata | Sidecars first, then filename/folder, then optional ffmpeg | Archive case is why those files exist |
| Playback | WebView2 `<video>` | Simple; document codec limits instead of shipping a decoder |
| Identity | Normalized absolute path | Predictable; move/rename is a new video |
| Network | None in v1 | Privacy; this is a local library |

## Architecture

Two processes, one window.

```
┌─────────────────────────────────────────┐
│  WebView2  (React + TypeScript)         │
│  Shell, routes, player, read-only views │
└─────────────────┬───────────────────────┘
                  │ Tauri commands
┌─────────────────▼───────────────────────┐
│  Rust                                   │
│  Library scan · metadata · SQLite       │
└─────────────────────────────────────────┘
                  │
         source folders on disk
```

The UI never reads the filesystem. It only calls commands and renders catalog rows. Playback uses a local-file URL the Rust side issues for a `video_id`.

### Units

1. **Library** — native folder picker, source CRUD, recursive scan, extension filter.
2. **Metadata resolver** — sidecars → folder/filename → optional ffmpeg → placeholder.
3. **Catalog** — SQLite at Tauri app data for identifier `com.mytube.app` (`library.db`). Generated thumbs in `thumbs/<sha256(path)>.jpg` under the same directory. On Windows this lives under `%APPDATA%`; do not hardcode a `mytube` folder name outside Tauri’s app-data API.
4. **Player** — `<video>` in the watch route; progress upserts every ~5s and on pause/unmount.

### Lifecycle

1. First launch: empty Library, CTA to add a folder.
2. Add source → scan → upsert catalog → Home fills.
3. Launch: rescan all sources (unavailable sources stay listed, their videos hidden).
4. Manual **Rescan** on the Library screen.

No file watcher in v1.

## Screens

Dark theme. Left sidebar + main pane. Search bar at the top of the main pane.

| Route | Sidebar | Main pane |
|---|---|---|
| `/` | Home | Thumbnail grid, newest first |
| `/history` | History | Same cards, most recently played first |
| `/channels` | Channels | List of named channels + video counts |
| `/channel/:slug` | Channels | That channel’s grid |
| `/watch/:id` | (current) | Player + metadata + more-from-channel/folder |
| `/library` | Library (bottom) | Source folders, add/remove, Rescan |

**Home cards:** thumbnail, duration badge, title, channel name if any. Sort: `upload_date` descending, NULLS last, then `mtime` descending.

**Watch:** player on top (right-hand “more” column on wide windows). Title, channel link (only if `channel_id` is set), upload/file date, expandable description. Below/side: **More from this channel** if the video has a channel, else **More from this folder** (same parent directory, excluding the current video). Never a global recommended row.

**Empty state:** “Add a folder of videos” + native folder picker. No splash feed.

**Can’t play / missing file:** stay on the watch page; show a short message and the path. Do not crash the window.

**Not in v1 UI:** playlists, likes, comments, subscriptions, theater-only chrome beyond native `<video>` fullscreen.

## Data

### Schema

**sources**

- `id` INTEGER PK
- `path` TEXT UNIQUE NOT NULL — normalized absolute path
- `added_at` TEXT NOT NULL — ISO-8601
- `last_scanned_at` TEXT NULL
- `available` INTEGER NOT NULL DEFAULT 1 — 0 if the folder was missing on last scan

**channels**

- `id` INTEGER PK
- `slug` TEXT UNIQUE NOT NULL
- `name` TEXT NOT NULL

**videos**

- `id` INTEGER PK
- `path` TEXT UNIQUE NOT NULL — normalized absolute path; this is the identity
- `source_id` INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE
- `channel_id` INTEGER NULL REFERENCES channels(id)
- `title` TEXT NOT NULL
- `description` TEXT NULL
- `duration_sec` REAL NULL
- `thumbnail_path` TEXT NULL — sidecar on disk or generated file under app data
- `upload_date` TEXT NULL — `YYYY-MM-DD`
- `parent_dir` TEXT NOT NULL — directory containing the file (for “more from this folder”)
- `mtime` INTEGER NOT NULL — file mtime as Unix seconds
- `size_bytes` INTEGER NOT NULL
- `added_at` TEXT NOT NULL

**watch_progress**

- `video_id` INTEGER PK REFERENCES videos(id) ON DELETE CASCADE
- `position_sec` REAL NOT NULL
- `updated_at` TEXT NOT NULL

**watch_history**

- `video_id` INTEGER PK REFERENCES videos(id) ON DELETE CASCADE
- `watched_at` TEXT NOT NULL

Indexes: `videos(source_id)`, `videos(channel_id)`, `videos(upload_date, mtime)`, `videos(title)`, `watch_history(watched_at)`.

### Rules

- **Path identity:** store `std::fs::canonicalize` output (Windows extended-length path is fine). Treat two paths as the same source or video if they canonicalize equal, or if they are equal under a case-insensitive UTF-8 compare. This is the only identity; no content hash in v1.
- **Add source:** reject if the path is an existing source, or if it contains / is contained by an existing source. Return a clear error; do not scan.
- **Remove source:** delete the row; cascade removes its videos, progress, and history. Files on disk are not touched. Then delete any channel with zero remaining videos.
- **Duplicate path in two sources:** should not happen if overlap is rejected; if a file still resolves to an existing `videos.path`, keep the first row and skip the insert.
- **Paging:** `list_home`, `search`, `get_channel` videos, and `list_history` use page size 24. `list_more` returns 12.
- **Rescan:** upsert by `path`. Path gone from disk → delete video (cascade progress/history). New file → insert. Same path → refresh metadata fields. Source folder missing → `available = 0`; do not delete its video rows; Home/search/channel hide videos whose source is unavailable.
- **Resume:** if `5 <= position_sec <= duration_sec - 10` (or `position_sec >= 5` when duration is unknown), start there; otherwise start at 0. Ignore negative, NaN, or `position_sec > duration_sec`.
- **History:** when playback actually starts, upsert `watch_history` (`watched_at = now`). Opening the page without playing does not write history.
- **Progress writes:** every ~5 seconds while playing, and on pause/unmount. Single-row upsert.
- **Search:** SQL `LIKE %query%` on `videos.title`, `videos.description`, and `channels.name`. Case-insensitive. Personal library scale; no extra search engine.
- **Home listing:** only videos whose source `available = 1`.

### Channel slug

Unicode lowercase, trim, replace each run of characters other than letters/numbers with a single `-`, trim `-`. If the result is empty, use `channel` plus a short hash of the original name (e.g. `channel-a1b2c3d4`). Same resolved name → same slug → one channel (e.g. two folders both “Veritasium”).

## Metadata resolver

Run once per discovered video file during scan.

**Supported extensions:** `.mp4`, `.webm`, `.mkv`, `.m4v`, `.mov` (case-insensitive). Everything else is ignored.

**Companion stem:** the video filename without extension. Look in the same directory for `<stem>.info.json` and `<stem>.webp` / `<stem>.jpg` / `<stem>.png`.

**Steps, first match wins per field:**

1. **Sidecar JSON** — if `<stem>.info.json` parses as an object:
   - `title` ← `title`
   - `channel name` ← `channel`, else `uploader`
   - `description` ← `description`
   - `duration_sec` ← `duration` (seconds, number)
   - `upload_date` ← `upload_date` (`YYYYMMDD` → `YYYY-MM-DD`)
   - thumbnail hint ← `thumbnail` if it is an existing local path, else first existing local path in `thumbnails[].url`
2. **Sidecar image** — if no usable thumbnail yet: `<stem>.webp`, then `.jpg`, then `.png`.
3. **Folder channel** — if still no channel name: take the first path segment under the source root. File directly in the source root → no folder channel.  
   Example: source `D:\Videos`, file `D:\Videos\Veritasium\2024\foo.mp4` → channel `Veritasium`. File `D:\Videos\foo.mp4` → no folder channel.
4. **Filename title** — if still no title: stem of the filename.
5. **ffmpeg (optional)** — if `ffmpeg` is on `PATH` and duration or thumbnail is still missing: probe duration; extract one JPEG frame into the app-data `thumbs/<sha256(path)>.jpg`. If ffmpeg is missing or fails, skip; do not fail the scan.
6. **Placeholder** — UI shows a letter tile (first character of title) when `thumbnail_path` is null.

Malformed JSON, unreadable files, or ffmpeg errors fall back to the next step. They never abort the scan.

**Scan errors:** unreadable subfolders are skipped and counted (`skipped_dirs` on the scan result). The rest of the source still imports.

## Playback and file access

- Watch route loads catalog metadata by integer `id`, then asks Rust for a playable local URL (`convertFileSrc` or a custom asset protocol).
- Protocol/CSP scope must allow user-selected source roots, updated when sources change. The app bundle path alone is not enough.
- Volume/mute are the native `<video>` controls. Not persisted in v1.
- MKV/HEVC or any file WebView2 refuses: show “Can’t play this format” and leave title/description/more-from in place.
- File missing at watch time: “File not found” + path + hint to rescan.

## Commands (Rust ↔ UI)

| Command | Role |
|---|---|
| `pick_folder` | Native directory dialog; returns a path or null |
| `add_source` | Persist source, scan it, return source + scan stats |
| `remove_source` | Delete source and cascaded catalog rows |
| `list_sources` | Library screen |
| `rescan` | Rescan all sources; return stats |
| `list_home` | Paged Home rows |
| `search` | Paged search rows |
| `list_channels` | Name, slug, video count (available videos only) |
| `get_channel` | Channel + paged videos |
| `get_video` | One video + channel name/slug + progress |
| `list_more` | More from channel or same `parent_dir` |
| `video_url` | Playable local URL for `<video src>` |
| `set_progress` | Upsert position |
| `start_watch` | Upsert history (`watched_at = now`) |
| `list_history` | Paged history joined to videos |

All list commands omit videos from unavailable sources.

## Error handling

| Situation | Behavior |
|---|---|
| Overlapping source | Do not add; show why (same as, inside, or contains an existing source) |
| Source path gone | `available = 0`; Library shows “Unavailable”; videos hidden until a later scan finds the folder |
| Unreadable subdirectory | Skip + count; continue |
| Bad `.info.json` | Ignore sidecar; continue resolver |
| Duplicate file via two sources | Keep the first catalog row |
| ffmpeg missing | Placeholders / missing duration; scan succeeds |
| Play fails | In-page message; no crash |
| Corrupt progress | Treat as no progress |
| Empty library | First-run CTA, not an error |

No network calls. No crash reporting.

## Testing

In-repo fixture directory with tiny valid MP4s (or stub files plus metadata-only tests where a real container is not required) and sidecar combinations.

**Resolver (unit):**

- Full yt-dlp sidecar (title, channel, description, duration, upload_date, local thumb).
- Broken JSON → filename / folder fallback.
- No sidecar, file in `source/Channel/file.mp4` → channel from first segment.
- No sidecar, file in source root → no channel.
- Nested `source/Channel/2024/file.mp4` → channel `Channel`, not `2024`.
- Slug: same display name collapses to one channel.
- Image preference: webp over jpg over png.

**Catalog (unit):**

- Upsert on rescan; delete when file disappears.
- Remove-source cascade.
- Two sources containing the same path → one row.
- Add source that equals / contains / is inside an existing source → error, no scan.
- Unavailable source hides videos from Home/search; rows remain.
- Resume window: 4s → 0; mid-file → resume; last 5s → 0; position > duration → 0.

**App (manual / WebView, before calling v1 done):**

- First run → add folder → Home cards.
- Search by title and channel.
- Channel page, watch, resume, history.
- Empty description, missing thumb, can’t-play file, missing file, unavailable drive.

## Follow-ups (not this spec)

1. Download: paste a YouTube URL, save into a library folder, import.
2. Broader codecs (bundled player / ffmpeg decode).
3. Playlists, queue, liked, watch later.
4. Live folder watcher.
5. Persist volume; richer keyboard shortcuts.
6. Path-independent identity (hash) so renames keep history.

## Implementation sketch

Single app repo at the workspace root (this is greenfield; no existing code).

- `src-tauri/` — Rust commands, rusqlite schema/migrations, scan, resolver, ffmpeg spawn.
- `src/` — React shell, routes, cards, watch player.
- `fixtures/library/` — scan/resolver test library.
- App data created on first launch; no installer customisation required for v1 beyond `tauri dev` / `tauri build`.
