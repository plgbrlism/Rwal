use std::path::Path;
use std::process::Command;
use crate::error::RwalError;

/// Set wallpaper using hyprpaper via hyprctl.
/// hyprpaper must be running; we preload the image then set it as the wallpaper.
pub fn set(path: &Path) -> Result<(), RwalError> {
    let path_str = path.to_string_lossy();

    // hyprpaper requires two steps: preload then set
    let preload = Command::new("hyprctl")
        .args(["hyprpaper", "preload", &path_str])
        .status()
        .map_err(|e| RwalError::WallpaperSetFailed(
            format!("hyprctl not found or failed to launch: {e}")
        ))?;

    if !preload.success() {
        return Err(RwalError::WallpaperSetFailed(
            format!("hyprctl preload exited with code {}", preload.code().unwrap_or(-1))
        ));
    }

    // Apply to all monitors using the wildcard syntax
    let wallpaper = Command::new("hyprctl")
        .args(["hyprpaper", "wallpaper", &format!(",{path_str}")])
        .status()
        .map_err(|e| RwalError::WallpaperSetFailed(
            format!("hyprctl wallpaper failed: {e}")
        ))?;

    if wallpaper.success() {
        Ok(())
    } else {
        Err(RwalError::WallpaperSetFailed(
            format!("hyprctl wallpaper exited with code {}", wallpaper.code().unwrap_or(-1))
        ))
    }
}
