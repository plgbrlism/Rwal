use std::path::Path;
use std::process::Command;
use crate::error::RwalError;

/// Set wallpaper on sway using swaymsg output * bg.
pub fn set(path: &Path) -> Result<(), RwalError> {
    let path_str = path
        .to_str()
        .ok_or_else(|| RwalError::WallpaperSetFailed(
            "wallpaper path contains invalid UTF-8".into()
        ))?;

    let status = Command::new("swaymsg")
        .args(["output", "*", "bg", path_str, "fill"])
        .status()
        .map_err(|e| RwalError::WallpaperSetFailed(
            format!("swaymsg not found or failed to launch: {e}")
        ))?;

    if status.success() {
        Ok(())
    } else {
        Err(RwalError::WallpaperSetFailed(
            format!("swaymsg exited with code {}", status.code().unwrap_or(-1))
        ))
    }
}