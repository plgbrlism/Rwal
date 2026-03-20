use std::path::Path;
use crate::error::RwalError;

/// Set wallpaper on macOS using osascript (AppleScript).
/// Works on all modern macOS versions without any extra tools.
pub fn set(path: &Path) -> Result<(), RwalError> {
    let path_str = path.to_string_lossy();

    // Tell Finder to set the desktop picture of every display
    let script = format!(
        r#"tell application "Finder"
    set desktop picture to POSIX file "{path_str}"
end tell"#
    );

    let status = std::process::Command::new("osascript")
        .args(["-e", &script])
        .status()
        .map_err(|e| RwalError::WallpaperSetFailed(
            format!("osascript not found or failed to launch: {e}")
        ))?;

    if status.success() {
        Ok(())
    } else {
        Err(RwalError::WallpaperSetFailed(
            format!("osascript exited with code {}", status.code().unwrap_or(-1))
        ))
    }
}
