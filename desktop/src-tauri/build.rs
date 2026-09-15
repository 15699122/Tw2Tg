fn main() {
    let capabilities = if std::env::var_os("CARGO_FEATURE_WDIO_E2E").is_some() {
        "./capabilities/*.json"
    } else {
        "./capabilities/default.json"
    };
    let attributes = tauri_build::Attributes::new().capabilities_path_pattern(capabilities);
    tauri_build::try_build(attributes).expect("failed to build Tauri application");
}
