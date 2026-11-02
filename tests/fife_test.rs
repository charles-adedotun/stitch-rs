use stitch_rs::{apply_fife_sizing, build_fife_suffix};

#[test]
fn test_fife_suffixes() {
    assert_eq!(build_fife_suffix(Some(1920), Some(1080)), "=w1920-h1080");
    assert_eq!(build_fife_suffix(Some(1280), None), "=w1280");
    assert_eq!(build_fife_suffix(None, Some(720)), "=h720");
    assert_eq!(build_fife_suffix(None, None), "");
    assert_eq!(build_fife_suffix(Some(0), Some(0)), "");
}

#[test]
fn test_fife_url_transformations() {
    let url = "https://lh3.googleusercontent.com/a/ACg8ocId";
    assert_eq!(
        apply_fife_sizing(url, Some(800), Some(600)),
        "https://lh3.googleusercontent.com/a/ACg8ocId=w800-h600"
    );

    // Existing suffix replacement
    let existing_url = "https://lh3.googleusercontent.com/a/ACg8ocId=s256";
    assert_eq!(
        apply_fife_sizing(existing_url, Some(1024), Some(768)),
        "https://lh3.googleusercontent.com/a/ACg8ocId=w1024-h768"
    );

    let existing_wh = "https://lh3.googleusercontent.com/a/ACg8ocId=w500-h300";
    assert_eq!(
        apply_fife_sizing(existing_wh, Some(1280), None),
        "https://lh3.googleusercontent.com/a/ACg8ocId=w1280"
    );
}

#[test]
fn test_fife_noop_on_empty() {
    assert_eq!(apply_fife_sizing("", Some(800), Some(600)), "");
    let url = "https://example.com/image.png";
    assert_eq!(apply_fife_sizing(url, None, None), url);
}
