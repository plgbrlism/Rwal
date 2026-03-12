/*

Auto-detects compositor from env vars:

$SWAYSOCK → sway
Falls back to feh, then nitrogen

*/
use std::path::Path;
use crate::error::{RwalError, warn};

mod feh;
mod nitrogen;
mod sway;

/// Set the wallpaper by auto-detecting the active compositor/setter.
///
/// Detection order:
/// 1. $SWAYSOCK env var → sway
/// 2. Try feh (most common on i3/openbox/etc.)
/// 3. Try nitrogen
/// 4. Warn if nothing works
pub fn set(path: &Path) -> Result<(), RwalError> {
    if let Some(setter) = detect() {
        return setter(path);
    }

    // Fallback: try each in order
    let setters: &[(&str, fn(&Path) -> Result<(), RwalError>)] = &[
        ("feh",      feh::set),
        ("nitrogen", nitrogen::set),
        ("sway",     sway::set),
    ];

    for (name, setter) in setters {
        match setter(path) {
            Ok(()) => return Ok(()),
            Err(e) => warn(&RwalError::WallpaperSetFailed(
                format!("{name} failed: {e}")
            )),
        }
    }

    Err(RwalError::NoCompositorDetected)
}

/// Detect the active compositor from environment variables.
fn detect() -> Option<fn(&Path) -> Result<(), RwalError>> {
    if std::env::var("SWAYSOCK").is_ok() {
        return Some(sway::set);
    }
    None
    // feh and nitrogen have no env var — they fall through to the
    // try-each fallback in set()
}