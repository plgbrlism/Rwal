/// Load a named theme from `colorschemes/dark/` or `colorschemes/light/`
/// and deserialize it into a `ColorDict`.
///
/// Resolution order:
///   1. `~/.config/rwal/default_colorschemes/dark/<name>.json`
///   2. `~/.config/rwal/default_colorschemes/light/<name>.json`
///   3. Bundled colorschemes embedded at compile time (same subfolder logic)
///
/// Returns `RwalError::ThemeNotFound` if the name doesn't match anything.

use rust_embed::RustEmbed;

use crate::colors::types::{ColorDict, Rgb, Special};
use crate::error::RwalError;
use crate::paths::Paths;

#[derive(RustEmbed)]
#[folder = "default_colorschemes/"]
#[exclude = "*.md"]
struct BundledColorschemes;

/// Load a named theme from `default_colorschemes/`.
///
/// Resolution order:
///   1. `~/.config/rwal/default_colorschemes/<name>.json`  (user wins)
///   2. Bundled colorschemes embedded at compile time
///
/// Returns `RwalError::ThemeNotFound` if the name matches nothing.
pub fn load(paths: &Paths, name: &str) -> Result<ColorDict, RwalError> {
    let name = name.trim_end_matches(".json");

    // 1. User colorschemes dir
    let user_path = paths.colorschemes_dir.join(format!("{name}.json"));
    if user_path.is_file() {
        return read_theme_file(&user_path);
    }

    // 2. Bundled (embedded) colorschemes
    let key = format!("{name}.json");
    if let Some(file) = BundledColorschemes::get(&key) {
        let contents = std::str::from_utf8(file.data.as_ref())
            .map_err(|_| RwalError::ThemeNotFound(name.to_string()))?;
        return parse_theme(contents, name);
    }

    Err(RwalError::ThemeNotFound(name.to_string()))
}

fn read_theme_file(path: &std::path::Path) -> Result<ColorDict, RwalError> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| RwalError::CacheReadError(path.to_path_buf(), e.to_string()))?;
    parse_theme(&contents, &path.display().to_string())
}

fn parse_theme(contents: &str, label: &str) -> Result<ColorDict, RwalError> {
    #[derive(serde::Deserialize)]
    struct ThemeFile {
        wallpaper: String,
        alpha: u8,
        special: SpecialRead,
        colors: std::collections::HashMap<String, String>,
    }

    #[derive(serde::Deserialize)]
    struct SpecialRead {
        background: String,
        foreground: String,
        cursor: String,
    }

    let corrupt = || RwalError::ThemeCorrupted(label.to_string());

    let file: ThemeFile = serde_json::from_str(contents).map_err(|_| corrupt())?;

    let mut colors = [Rgb::new(0, 0, 0); 16];
    for i in 0..16 {
        let key = format!("color{i}");
        let hex = file.colors.get(&key).ok_or_else(corrupt)?;
        colors[i] = Rgb::from_hex(hex).ok_or_else(corrupt)?;
    }

    Ok(ColorDict {
        wallpaper: std::path::PathBuf::from(file.wallpaper),
        alpha: file.alpha,
        special: Special {
            background: Rgb::from_hex(&file.special.background).ok_or_else(corrupt)?,
            foreground: Rgb::from_hex(&file.special.foreground).ok_or_else(corrupt)?,
            cursor: Rgb::from_hex(&file.special.cursor).ok_or_else(corrupt)?,
        },
        colors,
    })
}