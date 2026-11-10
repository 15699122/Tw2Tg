use super::*;

#[test]
fn configured_sidecar_args_passes_portable_gallery_dl_path_to_worker() {
    // Build the expected path via PathBuf::join so the assertion is
    // separator-agnostic and valid on Windows (backslash) as well as Unix.
    // Use a relative fixture so the root itself is valid on both Unix and
    // Windows; only the path construction is under test here.
    let root = std::path::Path::new("xarchive with spaces");
    let expected_gallery_dl = std::path::PathBuf::from(root)
        .join("sidecar")
        .join("gallery-dl")
        .join("gallery-dl.exe")
        .display()
        .to_string();
    let config = AppConfig::default();
    assert_eq!(
        configured_sidecar_args(root, &config),
        vec!["--gallery-dl".to_owned(), expected_gallery_dl]
    );
}

#[test]
fn parse_sidecar_args_accepts_explicit_json_array() {
    assert_eq!(
        crate::commands::parse_sidecar_args(r#"["-m","xarchive_downloader"]"#).unwrap(),
        vec!["-m", "xarchive_downloader"]
    );
}
