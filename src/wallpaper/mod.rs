/*
Wallpaper setter with Windows support.

Platform routing:
  - Windows   → PowerShell + SystemParametersInfo Win32 API
*/

use std::path::Path;
use crate::error::{RwalError, warn};

mod windows;

/// Set the wallpaper on Windows.
pub fn set(path: &Path) -> Result<(), RwalError> {
    windows::set(path)
}