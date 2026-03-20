use std::path::Path;
use crate::error::RwalError;

/// Set wallpaper on Windows using the SystemParametersInfoW Win32 API via PowerShell.
/// This avoids requiring a C FFI dependency and works on any modern Windows installation.
pub fn set(path: &Path) -> Result<(), RwalError> {
    let path_str = path.to_string_lossy();

    // SPI_SETDESKWALLPAPER = 0x0014, SPIF_UPDATEINIFILE | SPIF_SENDCHANGE = 0x3
    let script = format!(
        r#"Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class Wallpaper {{
    [DllImport("user32.dll", CharSet = CharSet.Auto)]
    public static extern int SystemParametersInfo(int uAction, int uParam, string lpvParam, int fuWinIni);
}}
"@
[Wallpaper]::SystemParametersInfo(0x0014, 0, "{path_str}", 0x3)"#
    );

    let status = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .status()
        .map_err(|e| RwalError::WallpaperSetFailed(
            format!("powershell not found or failed to launch: {e}")
        ))?;

    if status.success() {
        Ok(())
    } else {
        Err(RwalError::WallpaperSetFailed(
            format!("powershell exited with code {}", status.code().unwrap_or(-1))
        ))
    }
}
