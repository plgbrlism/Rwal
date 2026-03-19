/// Load a named theme from `themes/dark/` or `themes/light/`
/// and deserialize it into a `ColorDict`.
///
/// Resolution order:
///   1. `~/.config/rwal/themes/dark/<name>.json`
///   2. `~/.config/rwal/themes/light/<name>.json`
///   3. Bundled themes embedded at compile time (same subfolder logic)
///
/// Returns `RwalError::ThemeNotFound` if the name doesn't match anything.

use rust_embed::RustEmbed;

use crate::colors::types::{ColorDict, Rgb, Special};
use crate::error::RwalError;
use crate::paths::Paths;

#[derive(RustEmbed)]
#[folder = "themes/"]
#[exclude = "*.md"]
struct BundledThemes;

/// Load a named theme from `themes/`.
///
/// Resolution order:
///   1. `~/.config/rwal/themes/<name>.json`  (user wins)
///   2. Bundled themes embedded at compile time
///
/// Returns `RwalError::ThemeNotFound` if the name matches nothing.
pub fn load(paths: &Paths, name: &str) -> Result<ColorDict, RwalError> {
    let name = name.trim_end_matches(".json");

    // 1. User themes dir
    let user_path = paths.themes_dir.join(format!("{name}.json"));
    if user_path.is_file() {
        return read_theme_file(&user_path);
    }

    // 2. Bundled (embedded) themes
    let key = format!("{name}.json");
    if let Some(file) = BundledThemes::get(&key) {
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

/// List all available themes — bundled first, then user themes.
/// Bundled themes are marked with (default), user themes with (user).
/// If a user theme overrides a bundled one, only (user) is shown.
pub fn list_all(paths: &Paths) {
    use std::collections::HashSet;

    let mut seen: HashSet<String> = HashSet::new();

    // Collect user themes first so we know which bundled ones are overridden
    let mut user_themes: Vec<String> = Vec::new();
    if paths.themes_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&paths.themes_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        user_themes.push(stem.to_string());
                        seen.insert(stem.to_string());
                    }
                }
            }
        }
    }

    // Print bundled themes (skip if overridden by user)
    let mut bundled: Vec<String> = BundledThemes::iter()
        .filter_map(|f| {
            let name = f.trim_end_matches(".json").to_string();
            if seen.contains(&name) { None } else { Some(name) }
        })
        .collect();
    bundled.sort();

    for name in &bundled {
        println!("{name} (default)");
    }

    // Print user themes
    user_themes.sort();
    for name in &user_themes {
        println!("{name} (user)");
    }

    if bundled.is_empty() && user_themes.is_empty() {
        println!("no themes found");
    }
}