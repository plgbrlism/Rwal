use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use crate::error::RwalError;
use crate::paths::Paths;
use crate::colors::semantic::SemanticDict;

/// The top-level mapping file (~/.config/rwal/config-map.toml)
#[derive(Deserialize, Debug)]
pub struct ConfigMap {
    /// Mapping of arbitrary keys (e.g., "btop", "kitty") to a template and output path
    #[serde(flatten)]
    pub templates: HashMap<String, TemplateEntry>,
}

#[derive(Deserialize, Debug)]
pub struct TemplateEntry {
    /// The name of the template file in ~/.config/rwal/templates/ (e.g., "btop.theme")
    pub template: String,
    /// The destination path where the generated template should be symlinked (supports ~/)
    pub output: String,
}

pub fn render_all(paths: &Paths, _semantic: &SemanticDict) -> Result<(), RwalError> {
    let config_map = load_config_map(paths)?;
    
    for (name, entry) in config_map.templates {
        if let Err(e) = render_and_symlink(&name, &entry, paths) {
            crate::error::warn(&e);
        }
    }
    
    Ok(())
}

pub fn render_one(paths: &Paths, _semantic: &SemanticDict, name: &str) -> Result<(), RwalError> {
    let config_map = load_config_map(paths)?;
    let entry = config_map.templates.get(name)
        .ok_or_else(|| RwalError::Custom(format!("entry '{}' not found in config-map.toml", name)))?;
    
    render_and_symlink(name, entry, paths)
}

fn render_and_symlink(_name: &str, entry: &TemplateEntry, paths: &Paths) -> Result<(), RwalError> {
    let source_cache_path = paths.cache_dir.join(&entry.template);
    
    if !source_cache_path.exists() {
        return Err(RwalError::Custom(format!(
            "Template '{}' not found in cache. Ensure it exists in ~/.config/rwal/templates/ and the --template flag was run.",
            entry.template
        )));
    }

    let output_path = resolve_path(&entry.output);
    
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| RwalError::CreateDirFailed(parent.to_path_buf(), e.to_string()))?;
        }
    }

    // Remove existing symlink or file if it exists
    if output_path.exists() || output_path.is_symlink() {
        std::fs::remove_file(&output_path)
            .map_err(|e| RwalError::Custom(format!("Failed to remove existing file at {}: {}", output_path.display(), e)))?;
    }

    // Create the symlink
    std::os::unix::fs::symlink(&source_cache_path, &output_path)
        .map_err(|e| RwalError::SymlinkFailed(source_cache_path.clone(), output_path.clone(), e.to_string()))?;

    Ok(())
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


/// Print debug information about the config-map.toml.
pub fn debug(paths: &Paths, _semantic: &SemanticDict) -> Result<(), RwalError> {
    let config_map = load_config_map(paths)?;
    println!("\n  \x1b[1mDebugging config-map.toml\x1b[0m\n");

    for (app_name, entry) in &config_map.templates {
        println!("  \x1b[1m[{}]\x1b[0m", app_name);
        println!("    template: {}", entry.template);
        println!("    output:   {}", entry.output);
        let cache_path = paths.cache_dir.join(&entry.template);
        if cache_path.exists() {
            println!("    \x1b[32mstatus\x1b[0m: cached");
        } else {
            println!("    \x1b[31;1mstatus\x1b[0m: MISSING in cache (~/.cache/rwal/{})", entry.template);
        }
        println!();
    }

    Ok(())
}

fn load_config_map(paths: &Paths) -> Result<ConfigMap, RwalError> {
    if !paths.config_map.exists() {
        return Err(RwalError::Custom("config-map.toml not found. Run rwal once to seed it.".into()));
    }

    let content = std::fs::read_to_string(&paths.config_map)
        .map_err(|e| RwalError::CacheReadError(paths.config_map.clone(), e.to_string()))?;

    let map: ConfigMap = toml::from_str(&content)
        .map_err(|e| RwalError::Custom(format!("failed to parse config-map.toml: {}", e)))?;

    Ok(map)
}

fn resolve_path(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}
