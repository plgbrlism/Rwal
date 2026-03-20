use std::path::Path;
use std::process::Command;
use crate::error::RwalError;

/// Set wallpaper using swww — a Wayland wallpaper daemon.
/// swww-daemon must already be running. rwal calls `swww img` to apply.
pub fn set(path: &Path) -> Result<(), RwalError> {
    let status = Command::new("swww")
        .arg("img")
        .arg(path)
        .status()
        .map_err(|e| RwalError::WallpaperSetFailed(
            format!("swww not found or failed to launch: {e}")
        ))?;

    if status.success() {
        Ok(())
    } else {
        Err(RwalError::WallpaperSetFailed(
            format!("swww exited with code {}", status.code().unwrap_or(-1))
        ))
    }
}
