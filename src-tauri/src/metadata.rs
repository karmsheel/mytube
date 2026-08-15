use crate::ffmpeg::Ffmpeg;
use crate::pathutil::first_segment_under;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub const VIDEO_EXTS: &[&str] = &["mp4", "webm", "mkv", "m4v", "mov"];

#[derive(Debug, Clone, Default)]
pub struct ResolvedMeta {
    pub title: String,
    pub channel_name: Option<String>,
    pub description: Option<String>,
    pub duration_sec: Option<f64>,
    pub thumbnail_path: Option<PathBuf>,
    pub upload_date: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct InfoJson {
    title: Option<String>,
    channel: Option<String>,
    uploader: Option<String>,
    description: Option<String>,
    duration: Option<f64>,
    upload_date: Option<String>,
    thumbnail: Option<String>,
    thumbnails: Option<Vec<ThumbEntry>>,
}

#[derive(Debug, Deserialize, Default)]
struct ThumbEntry {
    url: Option<String>,
}

pub fn is_video_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| VIDEO_EXTS.iter().any(|x| x.eq_ignore_ascii_case(e)))
        .unwrap_or(false)
}

pub fn resolve(video_path: &Path, source_root: &Path, thumbs_dir: &Path) -> ResolvedMeta {
    let stem = video_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "video".into());
    let dir = video_path.parent().unwrap_or(source_root);
    let mut meta = ResolvedMeta {
        title: stem.clone(),
        ..Default::default()
    };

    let info_path = dir.join(format!("{stem}.info.json"));
    if let Ok(bytes) = std::fs::read(&info_path) {
        if let Ok(info) = serde_json::from_slice::<InfoJson>(&bytes) {
            meta.thumbnail_path = local_thumb_hint(&info);
            if let Some(t) = info.title.filter(|s| !s.is_empty()) {
                meta.title = t;
            }
            meta.channel_name = info
                .channel
                .filter(|s| !s.is_empty())
                .or(info.uploader.filter(|s| !s.is_empty()));
            meta.description = info.description;
            meta.duration_sec = info.duration.filter(|d| d.is_finite() && *d > 0.0);
            meta.upload_date = info.upload_date.as_deref().and_then(parse_upload_date);
        }
    }

    if meta.thumbnail_path.is_none() {
        for ext in ["webp", "jpg", "png"] {
            let p = dir.join(format!("{stem}.{ext}"));
            if p.is_file() {
                meta.thumbnail_path = Some(p);
                break;
            }
        }
    }

    if meta.channel_name.is_none() {
        meta.channel_name = first_segment_under(source_root, video_path);
    }

    if meta.duration_sec.is_none() || meta.thumbnail_path.is_none() {
        if let Some(ff) = Ffmpeg::detect() {
            if meta.duration_sec.is_none() {
                meta.duration_sec = ff.duration(video_path);
            }
            if meta.thumbnail_path.is_none() {
                let dest = thumbs_dir.join(format!("{}.jpg", path_hash(video_path)));
                if ff.extract_thumb(video_path, &dest) {
                    meta.thumbnail_path = Some(dest);
                }
            }
        }
    }

    meta
}

fn parse_upload_date(raw: &str) -> Option<String> {
    let d = raw.trim();
    if d.len() == 8 && d.chars().all(|c| c.is_ascii_digit()) {
        return Some(format!("{}-{}-{}", &d[0..4], &d[4..6], &d[6..8]));
    }
    None
}

fn local_thumb_hint(info: &InfoJson) -> Option<PathBuf> {
    if let Some(t) = &info.thumbnail {
        let p = PathBuf::from(t);
        if p.is_file() {
            return Some(p);
        }
    }
    if let Some(list) = &info.thumbnails {
        for e in list {
            if let Some(u) = &e.url {
                let p = PathBuf::from(u);
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }
    None
}

fn path_hash(path: &Path) -> String {
    let h = Sha256::digest(path.to_string_lossy().as_bytes());
    hex::encode(&h[..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn tmp() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "mytube-meta-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn full_sidecar_wins() {
        let root = tmp();
        let vid = root.join("Lesson [abc].mp4");
        fs::write(&vid, b"x").unwrap();
        let thumb = root.join("Lesson [abc].webp");
        fs::write(&thumb, b"img").unwrap();
        fs::write(
            root.join("Lesson [abc].info.json"),
            r#"{
              "title": "Real Title",
              "channel": "Veritasium",
              "description": "Desc",
              "duration": 123.5,
              "upload_date": "20240115",
              "thumbnail": "ignore-remote"
            }"#,
        )
        .unwrap();
        let thumbs = root.join("thumbs");
        fs::create_dir_all(&thumbs).unwrap();
        let m = resolve(&vid, &root, &thumbs);
        assert_eq!(m.title, "Real Title");
        assert_eq!(m.channel_name.as_deref(), Some("Veritasium"));
        assert_eq!(m.description.as_deref(), Some("Desc"));
        assert_eq!(m.duration_sec, Some(123.5));
        assert_eq!(m.upload_date.as_deref(), Some("2024-01-15"));
        assert_eq!(m.thumbnail_path.as_deref(), Some(thumb.as_path()));
    }

    #[test]
    fn broken_json_falls_back_to_filename_and_folder() {
        let root = tmp();
        let vid = root.join("Veritasium").join("My Video.mp4");
        fs::create_dir_all(vid.parent().unwrap()).unwrap();
        fs::write(&vid, b"x").unwrap();
        fs::write(root.join("Veritasium").join("My Video.info.json"), b"{not json").unwrap();
        let m = resolve(&vid, &root, &root.join("t"));
        assert_eq!(m.title, "My Video");
        assert_eq!(m.channel_name.as_deref(), Some("Veritasium"));
    }

    #[test]
    fn root_file_has_no_folder_channel() {
        let root = tmp();
        let vid = root.join("lone.webm");
        fs::write(&vid, b"x").unwrap();
        let m = resolve(&vid, &root, &root.join("t"));
        assert_eq!(m.title, "lone");
        assert_eq!(m.channel_name, None);
    }

    #[test]
    fn nested_year_uses_first_segment() {
        let root = tmp();
        let vid = root.join("Channel").join("2024").join("x.mkv");
        fs::create_dir_all(vid.parent().unwrap()).unwrap();
        fs::write(&vid, b"x").unwrap();
        let m = resolve(&vid, &root, &root.join("t"));
        assert_eq!(m.channel_name.as_deref(), Some("Channel"));
    }

    #[test]
    fn image_prefers_webp_then_jpg_then_png() {
        let root = tmp();
        let vid = root.join("a.mp4");
        fs::write(&vid, b"x").unwrap();
        fs::write(root.join("a.png"), b"p").unwrap();
        fs::write(root.join("a.jpg"), b"j").unwrap();
        fs::write(root.join("a.webp"), b"w").unwrap();
        let m = resolve(&vid, &root, &root.join("t"));
        assert_eq!(
            m.thumbnail_path.unwrap().file_name().unwrap(),
            "a.webp"
        );
    }

    #[test]
    fn prefers_uploader_if_no_channel() {
        let root = tmp();
        let vid = root.join("a.m4v");
        fs::write(&vid, b"x").unwrap();
        fs::write(
            root.join("a.info.json"),
            r#"{"title":"T","uploader":"U"}"#,
        )
        .unwrap();
        let m = resolve(&vid, &root, &root.join("t"));
        assert_eq!(m.channel_name.as_deref(), Some("U"));
    }

    #[test]
    fn is_video_filters_extensions() {
        assert!(is_video_file(Path::new("x.MP4")));
        assert!(is_video_file(Path::new("x.mov")));
        assert!(!is_video_file(Path::new("x.info.json")));
        assert!(!is_video_file(Path::new("x.txt")));
    }
}
