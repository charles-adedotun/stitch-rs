//! FIFE (Google Frontend Image Format Engine) dynamic image sizing utilities.

use regex::Regex;
use std::sync::OnceLock;

static FIFE_REGEX: OnceLock<Regex> = OnceLock::new();

fn get_fife_regex() -> &'static Regex {
    FIFE_REGEX.get_or_init(|| Regex::new(r"=[wWhHsSpP0-9-]+$").expect("valid fife regex"))
}

/// Builds a FIFE URL sizing suffix string.
///
/// Returns:
/// - `=w1280-h800` when both width and height are provided.
/// - `=w800` when only width is provided.
/// - `=h600` when only height is provided.
/// - `""` when neither is provided.
pub fn build_fife_suffix(width: Option<u32>, height: Option<u32>) -> String {
    let mut parts = Vec::with_capacity(2);
    if let Some(w) = width {
        if w > 0 {
            parts.push(format!("w{w}"));
        }
    }
    if let Some(h) = height {
        if h > 0 {
            parts.push(format!("h{h}"));
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("={}", parts.join("-"))
    }
}

/// Applies dynamic FIFE sizing parameters to an image URL.
///
/// If the URL already ends with a FIFE size parameter, it is replaced with the new one.
pub fn apply_fife_sizing(url: &str, width: Option<u32>, height: Option<u32>) -> String {
    if url.is_empty() {
        return String::new();
    }
    let suffix = build_fife_suffix(width, height);
    if suffix.is_empty() {
        return url.to_string();
    }

    let re = get_fife_regex();
    if re.is_match(url) {
        re.replace(url, &suffix).into_owned()
    } else {
        format!("{url}{suffix}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_suffix() {
        assert_eq!(build_fife_suffix(Some(1280), Some(800)), "=w1280-h800");
        assert_eq!(build_fife_suffix(Some(1024), None), "=w1024");
        assert_eq!(build_fife_suffix(None, Some(768)), "=h768");
        assert_eq!(build_fife_suffix(None, None), "");
    }

    #[test]
    fn test_apply_sizing_new() {
        let base = "https://lh3.googleusercontent.com/abc";
        let sized = apply_fife_sizing(base, Some(800), Some(600));
        assert_eq!(sized, "https://lh3.googleusercontent.com/abc=w800-h600");
    }

    #[test]
    fn test_apply_sizing_replace() {
        let base = "https://lh3.googleusercontent.com/abc=s1600";
        let sized = apply_fife_sizing(base, Some(640), Some(480));
        assert_eq!(sized, "https://lh3.googleusercontent.com/abc=w640-h480");
    }
}
