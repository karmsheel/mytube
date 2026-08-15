use std::path::Path;
use std::process::Command;

pub struct Ffmpeg {
    bin: String,
}

impl Ffmpeg {
    pub fn detect() -> Option<Self> {
        let bin = "ffmpeg";
        let ok = Command::new(bin).arg("-version").output().ok()?.status.success();
        if ok {
            Some(Self { bin: bin.into() })
        } else {
            None
        }
    }

    pub fn duration(&self, video: &Path) -> Option<f64> {
        let out = Command::new(&self.bin)
            .args(["-i"])
            .arg(video)
            .output()
            .ok()?;
        let err = String::from_utf8_lossy(&out.stderr);
        let marker = "Duration: ";
        let i = err.find(marker)?;
        let rest = &err[i + marker.len()..];
        let hms = rest.split(',').next()?;
        parse_hms(hms.trim())
    }

    pub fn extract_thumb(&self, video: &Path, dest: &Path) -> bool {
        if let Some(dir) = dest.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        Command::new(&self.bin)
            .args(["-y", "-ss", "1", "-i"])
            .arg(video)
            .args(["-frames:v", "1", "-q:v", "4"])
            .arg(dest)
            .output()
            .map(|o| o.status.success() && dest.exists())
            .unwrap_or(false)
    }
}

fn parse_hms(s: &str) -> Option<f64> {
    let mut parts = s.split(':');
    let h: f64 = parts.next()?.parse().ok()?;
    let m: f64 = parts.next()?.parse().ok()?;
    let sec: f64 = parts.next()?.parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + sec)
}
