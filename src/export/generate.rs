use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use crate::error::RwalError;
use crate::paths::Paths;
use crate::colors::semantic::SemanticDict;
use crate::colors::types::Rgb;

/// The top-level mapping file (~/.config/rwal/theme-map.toml)
#[derive(Deserialize, Debug)]
pub struct ThemeMap {
    /// Global role fallbacks: if role X is missing, try Y
    #[serde(default)]
    pub fallbacks: HashMap<String, String>,
    /// Application entries
    #[serde(flatten)]
    pub apps: HashMap<String, AppEntry>,
}

/// A single application's config mapping entry
#[derive(Deserialize, Debug)]
pub struct AppEntry {
    /// Where to write the generated config (supports ~/)
    pub output: String,
    /// Format: toml | css | ini (fallback: key = value)
    pub format: Option<String>,
    /// Map of [app-specific-key] = "semantic-role"
    pub map: HashMap<String, String>,
}

/// Render ALL apps defined in theme-map.toml using the active semantic palette.
pub fn render_all(paths: &Paths, semantic: &SemanticDict) -> Result<(), RwalError> {
    let theme_map = load_theme_map(paths)?;
    
    for (name, app) in theme_map.apps {
        if let Err(e) = render_app(&name, &app, semantic, &theme_map.fallbacks) {
            crate::error::warn(&e);
        }
    }
    
    Ok(())
}

/// Render a single named app from the mapping file.
pub fn render_one(paths: &Paths, semantic: &SemanticDict, name: &str) -> Result<(), RwalError> {
    let theme_map = load_theme_map(paths)?;
    let app = theme_map.apps.get(name)
        .ok_or_else(|| RwalError::Custom(format!("app '{}' not found in theme-map.toml", name)))?;
    
    render_app(name, app, semantic, &theme_map.fallbacks)
}

/// Show color previews. 
/// - If `base16` is None, just show semantic roles (Generation flow).
/// - If `base16` is Some, show both (Preview flow).
pub fn preview(semantic: &SemanticDict, base16: Option<&crate::colors::types::ColorDict>) {
    println!("\n  \x1b[1mSemantic Role Palette\x1b[0m\n");

    let roles = [
        ("background",      &semantic.colors.background),
        ("surface",         &semantic.colors.surface),
        ("foreground",      &semantic.colors.foreground),
        ("cursor",          &semantic.colors.cursor),
        ("primary",         &semantic.colors.primary),
        ("secondary",       &semantic.colors.secondary),
        ("tertiary",        &semantic.colors.tertiary),
        ("accent",          &semantic.colors.accent),
        ("error",           &semantic.colors.error),
        ("success",         &semantic.colors.success),
        ("warning",         &semantic.colors.warning),
        ("info",            &semantic.colors.info),
        ("neutral",         &semantic.colors.neutral),
        ("neutral_variant", &semantic.colors.neutral_variant),
    ];

    for (name, c) in roles {
        let fg = if crate::colors::adjust::relative_luminance(c) > 0.5 {
            "\x1b[38;2;0;0;0m" // Black text on bright blocks
        } else {
            "\x1b[38;2;255;255;255m" // White text on dark blocks
        };
        
        // Vertical blocks with overlaid names
        println!("  \x1b[48;2;{};{};{}m{} {:<18} \x1b[0m {}", c.r, c.g, c.b, fg, name, c.to_hex());
    }

    if let Some(dict) = base16 {
        println!("\n  \x1b[1mBase16 Palette Slots\x1b[0m\n");
        for (i, c) in dict.colors.iter().enumerate() {
            // Vertical empty blocks with hex beside them + slot index
            println!("  \x1b[48;2;{};{};{}m          \x1b[0m  {}  (color{})", c.r, c.g, c.b, c.to_hex(), i);
        }
    }
    
    println!();
}


/// Print debug information about the theme-map.toml.
pub fn debug(paths: &Paths, semantic: &SemanticDict) -> Result<(), RwalError> {
    let theme_map = load_theme_map(paths)?;
    println!("\n  \x1b[1mDebugging theme-map.toml\x1b[0m\n");

    let mut issues = 0;
    
    for (app_name, app) in &theme_map.apps {
        println!("  \x1b[1m[{}]\x1b[0m  (output: {})", app_name, app.output);
        
        let mut sorted_keys: Vec<_> = app.map.keys().collect();
        sorted_keys.sort();

        for key in sorted_keys {
            let role = &app.map[key];
            if let Some(color) = resolve_role(role, semantic, &theme_map.fallbacks) {
                // OK
                println!("    {}  →  {}  ({})", key, role, color);
            } else {
                println!("    \x1b[31merror\x1b[0m: {} uses unknown role '{}'", key, role);
                issues += 1;
            }
        }
        println!();
    }

    if issues == 0 {
        println!("  \x1b[32mOK\x1b[0m: No issues found in theme-map.toml");
    } else {
        println!("  \x1b[31mFound {} issues.\x1b[0m", issues);
    }
    
    Ok(())
}

fn load_theme_map(paths: &Paths) -> Result<ThemeMap, RwalError> {
    if !paths.theme_map.exists() {
        return Err(RwalError::Custom("theme-map.toml not found. Run rwal once to seed it.".into()));
    }

    let content = std::fs::read_to_string(&paths.theme_map)
        .map_err(|e| RwalError::CacheReadError(paths.theme_map.clone(), e.to_string()))?;

    let map: ThemeMap = toml::from_str(&content)
        .map_err(|e| RwalError::Custom(format!("failed to parse theme-map.toml: {}", e)))?;

    Ok(map)
}

fn render_app(name: &str, app: &AppEntry, semantic: &SemanticDict, fallbacks: &HashMap<String, String>) -> Result<(), RwalError> {
    let output_path = resolve_path(&app.output);
    
    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| RwalError::CreateDirFailed(parent.to_path_buf(), e.to_string()))?;
        }
    }

    let format = app.format.as_deref().unwrap_or("conf");
    let mut content = format!("# Generated by rwal for {} from {}\n", name, semantic.wallpaper);
    
    // Sort keys for deterministic output
    let mut keys: Vec<_> = app.map.keys().collect();
    keys.sort();

    for key in keys {
        let role = &app.map[key];
        
        let color = match resolve_role(role, semantic, fallbacks) {
            Some(c) => c,
            None => {
                crate::error::warn(&RwalError::Custom(format!("role '{}' (mapping to '{}') not found", role, key)));
                continue;
            }
        };

        match format {
            "toml" => {
                content.push_str(&format!("{} = \"{}\"\n", key, color));
            }
            "css" => {
                let css_key = key.replace('.', "-");
                content.push_str(&format!("  --{}: {};\n", css_key, color));
            }
            "ini" | "conf" => {
                content.push_str(&format!("{} = {}\n", key, color));
            }
            _ => {
                content.push_str(&format!("{} = {}\n", key, color));
            }
        }
    }

    std::fs::write(&output_path, content)
        .map_err(|e| RwalError::Custom(format!("failed to write config to {}: {}", output_path.display(), e)))?;

    Ok(())
}

fn resolve_role<'a>(role: &str, semantic: &'a SemanticDict, fallbacks: &HashMap<String, String>) -> Option<&'a Rgb> {
    // 1. Direct lookup
    if let Some(c) = lookup_direct(role, semantic) {
        return Some(c);
    }

    // 2. Fallback lookup (recursive-ish)
    if let Some(target) = fallbacks.get(role) {
        return resolve_role(target, semantic, fallbacks);
    }

    None
}

fn lookup_direct<'a>(role: &str, semantic: &'a SemanticDict) -> Option<&'a Rgb> {
    match role {
        "background" => Some(&semantic.colors.background),
        "surface"    => Some(&semantic.colors.surface),
        "foreground" => Some(&semantic.colors.foreground),
        "cursor"     => Some(&semantic.colors.cursor),
        "primary"    => Some(&semantic.colors.primary),
        "secondary"  => Some(&semantic.colors.secondary),
        "tertiary"   => Some(&semantic.colors.tertiary),
        "accent"     => Some(&semantic.colors.accent),
        "error"      => Some(&semantic.colors.error),
        "success"    => Some(&semantic.colors.success),
        "warning"    => Some(&semantic.colors.warning),
        "info"       => Some(&semantic.colors.info),
        "neutral"    => Some(&semantic.colors.neutral),
        "neutral_variant" => Some(&semantic.colors.neutral_variant),
        _ => None,
    }
}

fn resolve_path(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}
