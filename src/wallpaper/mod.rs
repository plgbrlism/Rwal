/*
Wallpaper setter with Linux-only compositor detection.

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

mod feh;
mod nitrogen;
mod sway;
mod hyprpaper;
mod xwallpaper;

/// Linux: detect active compositor/setter via environment variables, then try
/// X11 tools in order. Only mature, daemon-free backends are included.
pub fn set(path: &Path) -> Result<(), RwalError> {
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