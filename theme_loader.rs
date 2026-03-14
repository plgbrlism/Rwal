/// Load a named theme from `colorschemes/dark/` or `colorschemes/light/`
/// and deserialize it into a `ColorDict`.
///
/// Resolution order:
///   1. `~/.config/rwal/colorschemes/dark/<name>.json`
///   2. `~/.config/rwal/colorschemes/light/<name>.json`
///   3. Bundled colorschemes embedded at compile time (same subfolder logic)
///
/// Returns `RwalError::ThemeNotFound` if the name doesn't match anything.
pub fn load_theme(paths: &Paths, name: &str) -> Result<ColorDict, RwalError> {
    use crate::colors::types::{Rgb, Special};

    // Normalise name — strip .json suffix if the user typed it
    let name = name.trim_end_matches(".json");

    // 1. Check user colorscheme dirs first (dark then light)
    for subdir in &["dark", "light"] {
        let path = paths
            .colorschemes_dir
            .join(subdir)
            .join(format!("{name}.json"));

        if path.is_file() {
            return read_theme_file(&path);
        }
    }

    // 2. Fall back to bundled (embedded) colorschemes
    for subdir in &["dark", "light"] {
        let key = format!("{subdir}/{name}.json");
        if let Some(file) = BundledColorschemes::get(&key) {
            let contents = std::str::from_utf8(file.data.as_ref())
                .map_err(|_| RwalError::ThemeNotFound(name.to_string()))?;
            return parse_theme(contents, name);
        }
    }

    Err(RwalError::ThemeNotFound(name.to_string()))
}

/// Deserialize a theme JSON file on disk into a `ColorDict`.
fn read_theme_file(path: &std::path::Path) -> Result<ColorDict, RwalError> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| RwalError::CacheReadError(path.to_path_buf(), e.to_string()))?;
    parse_theme(&contents, &path.display().to_string())
}

/// Parse a theme JSON string into a `ColorDict`.
/// Reuses the same deserialization structs as `colors_json::read`.
fn parse_theme(contents: &str, label: &str) -> Result<ColorDict, RwalError> {
    use crate::colors::types::{Rgb, Special};

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