/*
Wallpaper setter with macOS support.

Platform routing:
  - macOS     → osascript (AppleScript via Finder)
*/

use std::path::Path;
use crate::error::RwalError;

mod macos;

/// Set the wallpaper on macOS.
pub fn set(path: &Path) -> Result<(), RwalError> {
    macos::set(path)
}