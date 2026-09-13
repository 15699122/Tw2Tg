use reqwest::blocking::Client;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize)]
pub struct Aria2Release {
    pub version: &'static str,
    pub asset_name: &'static str,
    pub sha256: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct Aria2Installation {
    pub found: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub source: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Aria2DownloadResult {
    pub version: String,
    pub path: String,
    pub sha256: String,
}

const ARIA2_RELEASES: &[Aria2Release] = &[
    Aria2Release {
        version: "1.37.0",
        asset_name: "aria2-1.37.0-win-64bit-build1.zip",
        sha256: "67D015301EEF0B612191212D564C5BB0A14B5B9C4796B76454276A4D28D9B288",
    },
    Aria2Release {
        version: "1.36.0",
        asset_name: "aria2-1.36.0-win-64bit-build1.zip",
        sha256: "C82DF5415125B438D72443923FEA7F5F9FDA1F326D0DBFAC6AAB16D58DBB7BF0",
    },
];

fn aria2_executable_names() -> &'static [&'static str] {
    if cfg!(target_os = "windows") {
        &["aria2c.exe", "aria2c"]
    } else {
        &["aria2c"]
    }
}

fn executable_version(path: &Path) -> Result<String, String> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|error| format!("failed to run aria2c: {error}"))?;
    if !output.status.success() {
        return Err(format!("aria2c exited with status {}", output.status));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .find_map(|line| {
            line.split_whitespace()
                .find(|part| {
                    part.chars()
                        .next()
                        .is_some_and(|character| character.is_ascii_digit())
                })
                .map(str::to_owned)
        })
        .ok_or_else(|| "aria2c version was not found in --version output".to_owned())
}

fn candidate_aria2_paths(extra_directories: &[PathBuf]) -> Vec<(PathBuf, &'static str)> {
    let mut candidates = Vec::new();
    if let Ok(current_exe) = std::env::current_exe()
        && let Some(directory) = current_exe.parent()
    {
        for name in aria2_executable_names() {
            candidates.push((directory.join(name), "program_directory"));
            candidates.push((directory.join("bin").join(name), "program_bin"));
        }
    }
    for directory in extra_directories {
        for name in aria2_executable_names() {
            candidates.push((directory.join(name), "program_data"));
            candidates.push((directory.join("bin").join(name), "program_data_bin"));
        }
        if let Ok(versions) = fs::read_dir(directory) {
            for version in versions.flatten() {
                let version_directory = version.path();
                if !version_directory.is_dir() {
                    continue;
                }
                for name in aria2_executable_names() {
                    candidates.push((version_directory.join(name), "program_data_version"));
                    candidates.push((
                        version_directory.join("bin").join(name),
                        "program_data_version_bin",
                    ));
                }
                if let Ok(build_directories) = fs::read_dir(&version_directory) {
                    for build in build_directories.flatten() {
                        if !build.path().is_dir() {
                            continue;
                        }
                        for name in aria2_executable_names() {
                            candidates.push((build.path().join(name), "program_data_build"));
                        }
                    }
                }
            }
        }
    }
    if let Some(path) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&path) {
            for name in aria2_executable_names() {
                candidates.push((directory.join(name), "path"));
            }
        }
    }
    candidates
}

fn detect_aria2_installation(extra_directories: &[PathBuf]) -> Aria2Installation {
    for (path, source) in candidate_aria2_paths(extra_directories) {
        if !path.is_file() {
            continue;
        }
        match executable_version(&path) {
            Ok(version) => {
                return Aria2Installation {
                    found: true,
                    version: Some(version),
                    path: Some(path.display().to_string()),
                    source: Some(source.to_owned()),
                    error: None,
                };
            }
            Err(error) => {
                return Aria2Installation {
                    found: false,
                    version: None,
                    path: Some(path.display().to_string()),
                    source: Some(source.to_owned()),
                    error: Some(error),
                };
            }
        }
    }
    Aria2Installation {
        found: false,
        version: None,
        path: None,
        source: None,
        error: None,
    }
}

#[tauri::command]
pub(crate) fn detect_aria2(app: AppHandle) -> Aria2Installation {
    let app_data = app.path().app_data_dir().ok();
    let extra_directories = app_data
        .into_iter()
        .map(|path| path.join("tools").join("aria2"))
        .collect::<Vec<_>>();
    detect_aria2_installation(&extra_directories)
}

#[tauri::command]
pub(crate) fn list_aria2_releases() -> Vec<Aria2Release> {
    ARIA2_RELEASES.to_vec()
}

pub(crate) fn selected_aria2_release(version: &str) -> Result<&'static Aria2Release, String> {
    ARIA2_RELEASES
        .iter()
        .find(|release| release.version == version)
        .ok_or_else(|| "unsupported aria2 version".to_owned())
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

#[tauri::command]
pub(crate) fn download_aria2(
    app: AppHandle,
    version: String,
) -> Result<Aria2DownloadResult, String> {
    if !cfg!(target_os = "windows") {
        return Err("aria2 Windows x64 downloads are only supported on Windows".to_owned());
    }
    let release = selected_aria2_release(&version)?;
    let url = format!(
        "https://github.com/aria2/aria2/releases/download/release-{}/{}",
        release.version, release.asset_name
    );
    let response = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|_| "failed to create download client".to_owned())?
        .get(url)
        .send()
        .map_err(|_| "failed to download aria2 release".to_owned())?;
    if !response.status().is_success() {
        return Err(format!(
            "aria2 release download returned HTTP {}",
            response.status()
        ));
    }
    let bytes = response
        .bytes()
        .map_err(|_| "failed to read aria2 release".to_owned())?;
    let actual_sha256 = sha256_hex(&bytes);
    if !actual_sha256.eq_ignore_ascii_case(release.sha256) {
        return Err("aria2 release SHA-256 mismatch".to_owned());
    }

    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to locate application data directory: {error}"))?;
    let install_root = app_data.join("tools").join("aria2").join(release.version);
    fs::create_dir_all(&install_root)
        .map_err(|error| format!("failed to create aria2 install directory: {error}"))?;
    let archive_path = install_root.join(release.asset_name);
    fs::write(&archive_path, &bytes)
        .map_err(|error| format!("failed to save aria2 release: {error}"))?;

    let status = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Expand-Archive -LiteralPath $args[0] -DestinationPath $args[1] -Force",
            &archive_path.to_string_lossy(),
            &install_root.to_string_lossy(),
        ])
        .status()
        .map_err(|error| format!("failed to extract aria2 release: {error}"))?;
    if !status.success() {
        return Err(format!(
            "aria2 release extraction failed with status {status}"
        ));
    }
    let executable = install_root
        .join(format!("aria2-{}-win-64bit-build1", release.version))
        .join("aria2c.exe");
    if !executable.is_file() {
        return Err("aria2 archive did not contain the expected aria2c.exe".to_owned());
    }
    let _ = fs::remove_file(&archive_path);
    Ok(Aria2DownloadResult {
        version: release.version.to_owned(),
        path: executable.display().to_string(),
        sha256: actual_sha256,
    })
}
