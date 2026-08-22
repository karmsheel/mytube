# Changelog

## 0.1.1 — 2026-08-22

Faster startup for the portable/installed app.

- Show the existing library immediately instead of waiting for a full rescan
- Skip metadata/ffmpeg work for files whose size and timestamp have not changed
- Do not hold the database lock while walking the disk
- SQLite WAL so the UI can read while a background scan runs

## 0.1.0 — 2026-08-19

First shippable build.

- Add local folders as library sources and scan video files (mp4, webm, mkv, m4v, mov)
- Use yt-dlp companion files (`.info.json`, sidecar thumbs) when present
- Home, search, channels, watch, resume, and history
- No ads, no recommendations, no network
