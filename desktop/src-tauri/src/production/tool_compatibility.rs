//! Downloader tool compatibility: the version snapshot check before any spawn.
//!
//! Split out of `production.rs` without changing behaviour. The rule that matters:
//! when the captured snapshot carries arguments, the resolved executable's
//! reported version must equal the captured compatibility value, and the check
//! runs *before* any download process starts. A mismatch is refused rather than
//! retried, so an unreviewed option can never be executed speculatively.

use std::process::Command;

use crate::archive::SidecarDownloadRequest;

pub(crate) fn revalidate_tool_compatibility(
    request: &SidecarDownloadRequest,
    aria2_program: Option<&str>,
    use_aria2: bool,
) -> Result<(), String> {
    crate::executor::validate_downloader_argument_snapshot(&request.downloader_arguments)
        .map_err(|error| format!("downloader argument snapshot rejected: {error}"))?;
    if use_aria2 {
        if !request.downloader_arguments.gallery_dl.is_empty() {
            return Err("gallery-dl arguments do not match the selected backend".into());
        }
        if !request.downloader_arguments.aria2.is_empty() {
            let program = aria2_program.ok_or_else(|| "ARIA2_NOT_CONFIGURED".to_owned())?;
            let current = executable_version(program, &["--version"])?;
            if request
                .downloader_arguments
                .tool_compatibility
                .aria2
                .as_deref()
                != Some(current.as_str())
            {
                return Err("aria2 tool version does not match the task snapshot".into());
            }
        }
    } else {
        if !request.downloader_arguments.aria2.is_empty() {
            return Err("aria2 arguments do not match the selected backend".into());
        }
        if !request.downloader_arguments.gallery_dl.is_empty() {
            let current = request
                .gallery_dl_program
                .as_deref()
                .ok_or_else(|| "gallery-dl executable is unavailable".to_owned())?;
            let current = executable_version(current, &["--version"])?;
            if request
                .downloader_arguments
                .tool_compatibility
                .gallery_dl
                .as_deref()
                != Some(current.as_str())
            {
                return Err("gallery-dl tool version does not match the task snapshot".into());
            }
        }
    }
    Ok(())
}

pub(crate) fn resolved_tool_version(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|_| "downloader version probe failed".to_owned())?;
    if !output.status.success() {
        return Err("downloader version probe failed".to_owned());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let version = text
        .split_whitespace()
        .find(|part| part.chars().next().is_some_and(|ch| ch.is_ascii_digit()))
        .map(str::to_owned)
        .ok_or_else(|| "downloader version probe returned no version".to_owned())?;
    let normalized = version.trim_start_matches('v');
    if normalized.split('.').count() < 2
        || normalized
            .split('.')
            .any(|part| part.parse::<u64>().is_err())
    {
        return Err("downloader version probe returned an unsupported version".to_owned());
    }
    Ok(normalized.to_owned())
}

pub(crate) fn executable_version(program: &str, args: &[&str]) -> Result<String, String> {
    resolved_tool_version(program, args)
}
