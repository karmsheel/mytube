use crate::error::AppResult;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlapKind {
    Same,
    CandidateInsideExisting,
    ExistingInsideCandidate,
}

impl OverlapKind {
    pub fn reason(self) -> String {
        match self {
            OverlapKind::Same => "This folder is already a library source".into(),
            OverlapKind::CandidateInsideExisting => {
                "This folder is inside an existing library source".into()
            }
            OverlapKind::ExistingInsideCandidate => {
                "This folder contains an existing library source".into()
            }
        }
    }
}

pub fn normalize_path(path: &Path) -> AppResult<PathBuf> {
    if path.exists() {
        return Ok(path.canonicalize()?);
    }
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    Ok(lex_normalize(&abs))
}

fn lex_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

pub fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{rest}"))
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        path.to_path_buf()
    }
}

pub fn display_path(path: &Path) -> String {
    strip_verbatim_prefix(path).to_string_lossy().into_owned()
}

fn key(path: &Path) -> String {
    let n = normalize_path(path).unwrap_or_else(|_| path.to_path_buf());
    strip_verbatim_prefix(&n).to_string_lossy().to_lowercase()
}

pub fn paths_equal(a: &Path, b: &Path) -> bool {
    if let (Ok(ca), Ok(cb)) = (a.canonicalize(), b.canonicalize()) {
        return ca == cb || ca.to_string_lossy().eq_ignore_ascii_case(&cb.to_string_lossy());
    }
    key(a) == key(b)
}

pub fn is_path_prefix(prefix: &Path, path: &Path) -> bool {
    let p = key(prefix);
    let c = key(path);
    if c == p {
        return true;
    }
    let sep = if p.ends_with('\\') || p.ends_with('/') {
        p.clone()
    } else {
        format!("{p}\\")
    };
    let sep_fwd = sep.replace('\\', "/");
    let c_fwd = c.replace('\\', "/");
    c.starts_with(&sep) || c_fwd.starts_with(&sep_fwd.replace('\\', "/"))
}

pub fn source_overlap(existing: &Path, candidate: &Path) -> Option<OverlapKind> {
    if paths_equal(existing, candidate) {
        return Some(OverlapKind::Same);
    }
    if is_path_prefix(existing, candidate) {
        return Some(OverlapKind::CandidateInsideExisting);
    }
    if is_path_prefix(candidate, existing) {
        return Some(OverlapKind::ExistingInsideCandidate);
    }
    None
}

pub fn first_segment_under(source_root: &Path, file_path: &Path) -> Option<String> {
    let root = strip_verbatim_prefix(&normalize_path(source_root).ok()?);
    let parent = file_path.parent()?;
    let parent_n = strip_verbatim_prefix(&normalize_path(parent).ok()?);
    if paths_equal(&root, &parent_n) {
        return None;
    }
    let rel = parent_n.strip_prefix(&root).ok()?;
    rel.components()
        .next()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn tmp() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "mytube-path-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn overlap_same_and_nested() {
        let root = tmp();
        let a = root.join("Videos");
        let b = a.join("Nested");
        fs::create_dir_all(&b).unwrap();
        assert!(matches!(source_overlap(&a, &a), Some(OverlapKind::Same)));
        assert!(matches!(
            source_overlap(&a, &b),
            Some(OverlapKind::CandidateInsideExisting)
        ));
        assert!(matches!(
            source_overlap(&b, &a),
            Some(OverlapKind::ExistingInsideCandidate)
        ));
        let other = root.join("Other");
        fs::create_dir_all(&other).unwrap();
        assert!(source_overlap(&a, &other).is_none());
    }

    #[test]
    fn first_segment_skips_year_folder() {
        let root = tmp().join("Videos");
        let file = root.join("Veritasium").join("2024").join("foo.mp4");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, b"x").unwrap();
        assert_eq!(
            first_segment_under(&root, &file).as_deref(),
            Some("Veritasium")
        );
        let flat = root.join("foo.mp4");
        fs::write(&flat, b"x").unwrap();
        assert_eq!(first_segment_under(&root, &flat), None);
    }

    #[test]
    fn paths_equal_is_case_insensitive() {
        let a = std::path::Path::new(r"C:\Videos\Foo.mp4");
        let b = std::path::Path::new(r"c:\videos\foo.mp4");
        assert!(paths_equal(a, b));
    }

    #[test]
    fn key_and_prefix_strip_verbatim() {
        let prefix = Path::new(r"\\?\C:\Videos");
        let nested = Path::new(r"C:\Videos\foo.mp4");
        assert_eq!(key(prefix), r"c:\videos");
        assert!(is_path_prefix(prefix, nested));
        let unc = Path::new(r"\\?\UNC\server\share\Videos");
        assert_eq!(key(unc), r"\\server\share\videos");
        assert_eq!(
            display_path(Path::new(r"\\?\C:\Videos\a.mp4")),
            r"C:\Videos\a.mp4"
        );
    }
}
