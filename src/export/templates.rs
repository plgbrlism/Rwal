use std::collections::HashMap;

use crate::colors::types::ColorDict;
use crate::error::{warn, RwalError};
use crate::paths::Paths;

/// Render all templates and write rendered output to `~/.cache/rwal/`.
///
/// Only user templates from `~/.config/rwal/templates/` are loaded.
/// Drop any template file there to have it rendered on every run.
pub fn render_all(paths: &Paths, dict: &ColorDict) -> Result<(), RwalError> {
    let templates = collect_templates(paths)?;

    for (filename, contents) in templates {
        let rendered = replace_tokens(&contents, dict);
        let out_path = paths.cache_dir.join(&filename);

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| RwalError::TemplateWriteError(out_path.clone(), e.to_string()))?;
        }

        match std::fs::write(&out_path, rendered) {
            Ok(()) => {}
            Err(e) => warn(&RwalError::TemplateWriteError(out_path, e.to_string())),
        }
    }

    Ok(())
}

/// Collect all user templates from `~/.config/rwal/templates/`.
fn collect_templates(paths: &Paths) -> Result<HashMap<String, String>, RwalError> {
    let mut map: HashMap<String, String> = HashMap::new();

    if paths.templates_dir.is_dir() {
        let entries = std::fs::read_dir(&paths.templates_dir)
            .map_err(|e| RwalError::IoError(e.to_string()))?;

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let filename = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };

            match std::fs::read_to_string(&path) {
                Ok(contents) => {
                    map.insert(filename, contents);
                }
                Err(e) => warn(&RwalError::TemplateReadError(path, e.to_string())),
            }
        }
    }

    Ok(map)
}


/// Replace all `{token}` placeholders in a template with color values from the dict.
///
/// Supported tokens:
///   {color0} … {color15}  — palette colors as #rrggbb hex
///   {background}          — background color as #rrggbb hex
///   {foreground}          — foreground color as #rrggbb hex
///   {cursor}              — cursor color as #rrggbb hex
///   {wallpaper}           — absolute path to the source wallpaper
///   {alpha}               — alpha/opacity value (0–100)
fn replace_tokens(contents: &str, dict: &ColorDict) -> String {
    let mut out = contents.to_string();

    for i in 0..16 {
        let token = format!("{{color{i}}}");
        let value = dict.colors[i].to_hex();
        out = out.replace(&token, &value);
    }

    out = out.replace("{background}", &dict.special.background.to_hex());
    out = out.replace("{foreground}", &dict.special.foreground.to_hex());
    out = out.replace("{cursor}", &dict.special.cursor.to_hex());
    out = out.replace("{wallpaper}", &dict.wallpaper.display().to_string());
    out = out.replace("{alpha}", &dict.alpha.to_string());

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::types::{Rgb, Special};
    use std::path::{Path, PathBuf};

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "rwal_tpl_test_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .subsec_nanos()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
        }
        fn path(&self) -> &Path{
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn dummy_dict() -> ColorDict {
        let mut colors = [Rgb::new(0, 0, 0); 16];
        colors[0] = Rgb::new(10, 10, 10);
        colors[1] = Rgb::new(200, 50, 50);
        colors[15] = Rgb::new(240, 240, 240);

        ColorDict {
            wallpaper: PathBuf::from("/home/user/wall.jpg"),
            alpha: 90,
            special: Special {
                background: Rgb::new(10, 10, 10),
                foreground: Rgb::new(240, 240, 240),
                cursor: Rgb::new(240, 240, 240),
            },
            colors,
        }
    }

    #[test]
    fn test_replace_color_tokens() {
        let dict = dummy_dict();
        let out = replace_tokens("{color0} {color1} {color15}", &dict);
        assert!(out.contains(&dict.colors[0].to_hex()));
        assert!(out.contains(&dict.colors[1].to_hex()));
        assert!(out.contains(&dict.colors[15].to_hex()));
    }

    #[test]
    fn test_replace_special_tokens() {
        let dict = dummy_dict();
        let out = replace_tokens("{background} {foreground} {cursor}", &dict);
        assert!(out.contains(&dict.special.background.to_hex()));
        assert!(out.contains(&dict.special.foreground.to_hex()));
        assert!(out.contains(&dict.special.cursor.to_hex()));
    }

    #[test]
    fn test_replace_wallpaper_and_alpha() {
        let dict = dummy_dict();
        let out = replace_tokens("{wallpaper} {alpha}", &dict);
        assert!(out.contains("/home/user/wall.jpg"));
        assert!(out.contains("90"));
    }

    #[test]
    fn test_no_tokens_unchanged() {
        let dict = dummy_dict();
        let content = "nothing to replace here";
        assert_eq!(replace_tokens(content, &dict), content);
    }

    #[test]
    fn test_user_template_is_loaded() {
        let tmp = TempDir::new();
        let paths = Paths::from_home(tmp.path().to_path_buf());
        paths.ensure_dirs().unwrap();

        std::fs::write(
            paths.templates_dir.join("colors.css"),
            "/* user template */\n:root { --bg: {background}; }",
        )
        .unwrap();

        let templates = collect_templates(&paths).unwrap();
        let css = templates.get("colors.css").unwrap();
        assert!(css.contains("user template"));
    }

    #[test]
    fn test_render_all_writes_to_cache() {
        let tmp = TempDir::new();
        let paths = Paths::from_home(tmp.path().to_path_buf());
        paths.ensure_dirs().unwrap();
        let dict = dummy_dict();

        std::fs::write(
            paths.templates_dir.join("colors.txt"),
            "{background}",
        )
        .unwrap();

        render_all(&paths, &dict).unwrap();

        let out = std::fs::read_to_string(paths.cache_dir.join("colors.txt")).unwrap();
        assert!(out.contains(&dict.special.background.to_hex()));
    }
}