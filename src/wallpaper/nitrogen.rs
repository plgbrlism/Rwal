use std::path::Path;
use std::process::Command;
use crate::error::RwalError;

/// Set wallpaper using nitrogen with --set-scaled (keep aspect ratio).
pub fn set(path: &Path) -> Result<(), RwalError> {
    let status = Command::new("nitrogen")
        .args(["--set-scaled"])
        .arg(path)
        .status()
        .map_err(|e| RwalError::WallpaperSetFailed(
            format!("nitrogen not found or failed to launch: {e}")
        ))?;

    if status.success() {
        Ok(())
    } else {
        Err(RwalError::WallpaperSetFailed(
            format!("nitrogen exited with code {}", status.code().unwrap_or(-1))
        ))
    }
}