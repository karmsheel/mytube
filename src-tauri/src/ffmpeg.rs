use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

fn command(bin: &str) -> Command {
    let mut cmd = Command::new(bin);
    // Avoid a console flash when spawning ffmpeg on Windows.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

pub struct Ffmpeg {
    bin: String,
}

impl Ffmpeg {
    pub fn detect() -> Option<&'static Self> {
        static DETECTED: OnceLock<Option<Ffmpeg>> = OnceLock::new();
        DETECTED
            .get_or_init(|| {
                let bin = "ffmpeg";
                let ok = command(bin).arg("-version").output().ok()?.status.success();
                ok.then(|| Self { bin: bin.into() })
            })
            .as_ref()
    }

    pub fn duration(&self, video: &Path) -> Option<f64> {
        let out = command(&self.bin)
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
        command(&self.bin)
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
