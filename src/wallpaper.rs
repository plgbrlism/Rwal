use std::process::Command;

pub fn set_wallpaper(backend: &str, path: &std::path::Path) {
    log::info!("Setting wallpaper with backend '{}' to path '{}'", backend, path.display());
    let mut cmd = Command::new(backend);
    match backend {
        "feh" => {
            cmd.arg("--bg-fill");
        },
        "swaybg" => {
            cmd.arg("-i");
        },
        "xwallpaper" => {
            cmd.arg("--zoom");
        }
        _ => {
            log::warn!("Unsupported wallpaper backend: {}", backend);
            return;
        }
    }
    cmd.arg(path);
    
    match cmd.status() {
        Ok(status) => {
            if !status.success() {
                log::error!("Failed to set wallpaper with {}: {}", backend, status);
            }
        }
        Err(e) => {
            log::error!("Failed to execute wallpaper command: {}", e);
        }
    }
}
