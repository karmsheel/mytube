use sha2::{Digest, Sha256};

pub fn channel_slug(name: &str) -> String {
    let lower = name.trim().to_lowercase();
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in lower.chars() {
        if ch.is_alphanumeric() {
            out.push(ch);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        let hash = Sha256::digest(name.as_bytes());
        format!("channel-{:x}{:x}{:x}{:x}", hash[0], hash[1], hash[2], hash[3])
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::channel_slug;

    #[test]
    fn veritasium_is_stable() {
        assert_eq!(channel_slug("Veritasium"), "veritasium");
        assert_eq!(channel_slug("Veritasium"), channel_slug("veritasium"));
    }

    #[test]
    fn punctuation_becomes_single_hyphen() {
        assert_eq!(channel_slug("  Hello, World!!  "), "hello-world");
    }

    #[test]
    fn empty_after_strip_uses_hash_prefix() {
        let s = channel_slug("---");
        assert!(s.starts_with("channel-"), "{s}");
        assert_eq!(s, channel_slug("---"));
    }
}
