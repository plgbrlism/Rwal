use std::path::Path;
use std::process::Command;
use crate::error::RwalError;

/// Set wallpaper using feh with --bg-scale (keep aspect ratio).
pub fn set(path: &Path) -> Result<(), RwalError> {
    let status = Command::new("feh")
        .args(["--no-fehbg", "--bg-scale"])
        .arg(path)
        .status()
        .map_err(|e| RwalError::WallpaperSetFailed(
            format!("feh not found or failed to launch: {e}")
        ))?;

    if status.success() {
        Ok(())
    } else {
        Err(RwalError::WallpaperSetFailed(
            format!("feh exited with code {}", status.code().unwrap_or(-1))
        ))
    }
}