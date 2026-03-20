/*
Wallpaper setter with cross-platform support and automatic compositor detection.

Platform routing (compile-time):
  - Windows   → PowerShell + SystemParametersInfo Win32 API
  - macOS     → osascript (AppleScript via Finder)
  - Linux     → runtime auto-detection (see set_linux below)

Linux detection order:
  1. $SWAYSOCK                    → sway
  2. $HYPRLAND_INSTANCE_SIGNATURE → hyprpaper (via hyprctl)
  3. X11 try-in-order fallback    → feh → nitrogen → xwallpaper

No generic Wayland fallback is included. Tools like swww require a running
daemon that rwal cannot guarantee, making silent failures likely. Users on
non-sway/non-hyprland Wayland compositors should use XWayland tools above,
or route through a user template that calls their preferred setter directly.

Adding a new backend: create a .rs file in this directory and add it to the
appropriate detection block. No changes to core logic are required.
*/

use std::path::Path;
use crate::error::{RwalError, warn};

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
mod feh;
#[cfg(target_os = "linux")]
mod nitrogen;
#[cfg(target_os = "linux")]
mod sway;
#[cfg(target_os = "linux")]
mod hyprpaper;
#[cfg(target_os = "linux")]
mod xwallpaper;

/// Set the wallpaper using the best available method for the current platform.
pub fn set(path: &Path) -> Result<(), RwalError> {
    #[cfg(target_os = "windows")]
    return windows::set(path);

    #[cfg(target_os = "macos")]
    return macos::set(path);

    #[cfg(target_os = "linux")]
    return set_linux(path);

    #[allow(unreachable_code)]
    Err(RwalError::WallpaperSetFailed(
        "unsupported platform".to_string()
    ))
}

/// Linux: detect active compositor/setter via environment variables, then try
/// X11 tools in order. Only mature, daemon-free backends are included.
#[cfg(target_os = "linux")]
fn set_linux(path: &Path) -> Result<(), RwalError> {
    // Wayland: Sway
    if std::env::var("SWAYSOCK").is_ok() {
        return sway::set(path);
    }

    // Wayland: Hyprland
    if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
        return hyprpaper::set(path);
    }

    // X11 fallback: try each in order, warn on individual failures
    let x11_setters: &[(&str, fn(&Path) -> Result<(), RwalError>)] = &[
        ("feh",         feh::set),
        ("nitrogen",    nitrogen::set),
        ("xwallpaper",  xwallpaper::set),
    ];

    for (name, setter) in x11_setters {
        match setter(path) {
            Ok(()) => return Ok(()),
            Err(e) => warn(&RwalError::WallpaperSetFailed(
                format!("{name} failed: {e}")
            )),
        }
    }

    Err(RwalError::NoCompositorDetected)
}