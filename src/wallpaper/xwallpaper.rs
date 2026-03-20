use std::path::Path;
use std::process::Command;
use crate::error::RwalError;

/// Set wallpaper using xwallpaper (X11 only).
pub fn set(path: &Path) -> Result<(), RwalError> {
    let status = Command::new("xwallpaper")
        .args(["--zoom"])
        .arg(path)
        .status()
        .map_err(|e| RwalError::WallpaperSetFailed(
            format!("xwallpaper not found or failed to launch: {e}")
        ))?;

    if status.success() {
        Ok(())
    } else {
        Err(RwalError::WallpaperSetFailed(
            format!("xwallpaper exited with code {}", status.code().unwrap_or(-1))
        ))
    }
}
