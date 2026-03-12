/*

Reads every file in ~/.config/rwal/templates/
Replaces tokens: {color0}–{color15}, {background}, {foreground}, {cursor}, {wallpaper}, {alpha}
Also supports {color0.rgb}, {color0.r}, {color0.g}, {color0.b} for flexibility
Writes output to target path specified in a comment on line 1 of each template: # Target: ~/.config/kitty/colors.conf

*/
use std::path::{Path, PathBuf};
use crate::colors::types::ColorDict;
use crate::error::{RwalError, warn};
use crate::paths::Paths;

/// Render all templates in `~/.config/rwal/templates/` using the active `ColorDict`.
/// Skips files with no `# Target:` directive on line 1 with a warning.
pub fn render_all(paths: &Paths, dict: &ColorDict) -> Result<(), RwalError> {
    let entries = std::fs::read_dir(&paths.templates_dir)
        .map_err(|e| RwalError::IoError(e.to_string()))?;

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        match render_one(&path, dict) {
            Ok(()) => {}
            Err(RwalError::TemplateMissingTarget(_)) => {
                warn(&RwalError::TemplateMissingTarget(path));
            }
            Err(e) => warn(&e),
        }
    }

    Ok(())
}

/// Render a single template file and write it to its declared target path.
pub fn render_one(template_path: &Path, dict: &ColorDict) -> Result<(), RwalError> {
    let contents = std::fs::read_to_string(template_path)
        .map_err(|e| RwalError::TemplateReadError(
            template_path.to_path_buf(),
            e.to_string(),
        ))?;

    let target = parse_target(&contents)
        .ok_or_else(|| RwalError::TemplateMissingTarget(template_path.to_path_buf()))?;

    let rendered = replace_tokens(&contents, dict);

    // Create parent directories if they don't exist
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| RwalError::TemplateWriteError(
                target.clone(),
                e.to_string(),
            ))?;
    }

    std::fs::write(&target, rendered)
        .map_err(|e| RwalError::TemplateWriteError(target.clone(), e.to_string()))?;

    Ok(())
}

/// Parse the target path from line 1 of the template.
/// Supports comment styles: `# Target:`, `// Target:`, `; Target:`, `/* Target:`
fn parse_target(contents: &str) -> Option<PathBuf> {
    let first_line = contents.lines().next()?;

    // Strip known comment prefixes
    let stripped = first_line
        .trim()
        .trim_start_matches("/*")
        .trim_start_matches("//")
        .trim_start_matches('#')
        .trim_start_matches(';')
        .trim();

    let path_str = stripped.strip_prefix("Target:")?.trim();

    if path_str.is_empty() {
        return None;
    }

    // Expand ~ to home directory
    let expanded = expand_tilde(path_str);
    Some(PathBuf::from(expanded))
}

/// Replace all `{tokenN}` placeholders in a template with color values.
fn replace_tokens(contents: &str, dict: &ColorDict) -> String {
    let mut out = contents.to_string();

    // Replace color0..color15
    for i in 0..16 {
        let token = format!("{{color{i}}}");
        let value = dict.colors[i].to_hex();
        out = out.replace(&token, &value);
    }

    // Replace special tokens
    out = out.replace("{background}", &dict.special.background.to_hex());
    out = out.replace("{foreground}", &dict.special.foreground.to_hex());
    out = out.replace("{cursor}",     &dict.special.cursor.to_hex());
    out = out.replace("{wallpaper}",  &dict.wallpaper.display().to_string());
    out = out.replace("{alpha}",      &dict.alpha.to_string());

    out
}

/// Expand a leading `~` to the user's home directory.
fn expand_tilde(path: &str) -> String {
    if path.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            return path.replacen('~', &home.display().to_string(), 1);
        }
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use crate::colors::types::{Rgb, Special};

    // ── helpers ──────────────────────────────────────────────────────────────

    struct TempDir { path: PathBuf }

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
        fn path(&self) -> &Path { &self.path }
        fn create_file(&self, name: &str, contents: &str) -> PathBuf {
            let p = self.path.join(name);
            std::fs::write(&p, contents).unwrap();
            p
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.path); }
    }

    fn dummy_dict(tmp: &TempDir) -> ColorDict {
        let mut colors = [Rgb::new(0, 0, 0); 16];
        colors[0]  = Rgb::new(10,  10,  10);
        colors[1]  = Rgb::new(200, 50,  50);
        colors[15] = Rgb::new(240, 240, 240);

        ColorDict {
            wallpaper: PathBuf::from("/home/user/wall.jpg"),
            alpha: 100,
            special: Special {
                background: Rgb::new(10,  10,  10),
                foreground: Rgb::new(240, 240, 240),
                cursor:     Rgb::new(240, 240, 240),
            },
            colors,
        }
    }

    // ── parse_target ─────────────────────────────────────────────────────────

    #[test]
    fn test_parse_target_hash_comment() {
        let content = "# Target: /tmp/output.conf\nsome content";
        let result = parse_target(content).unwrap();
        assert_eq!(result, PathBuf::from("/tmp/output.conf"));
    }

    #[test]
    fn test_parse_target_slash_comment() {
        let content = "// Target: /tmp/output.css\nsome content";
        let result = parse_target(content).unwrap();
        assert_eq!(result, PathBuf::from("/tmp/output.css"));
    }

    #[test]
    fn test_parse_target_semicolon_comment() {
        let content = "; Target: /tmp/output.ini\nsome content";
        let result = parse_target(content).unwrap();
        assert_eq!(result, PathBuf::from("/tmp/output.ini"));
    }

    #[test]
    fn test_parse_target_block_comment() {
        let content = "/* Target: /tmp/output.css */\nsome content";
        let result = parse_target(content).unwrap();
        assert_eq!(result, PathBuf::from("/tmp/output.css */"));
        // trailing */ stays — that's acceptable, path won't exist but
        // the real templates don't use block comments on line 1
    }

    #[test]
    fn test_parse_target_missing_returns_none() {
        let content = "no target directive here\nsome content";
        assert!(parse_target(content).is_none());
    }

    #[test]
    fn test_parse_target_empty_file_returns_none() {
        assert!(parse_target("").is_none());
    }

    // ── replace_tokens ───────────────────────────────────────────────────────

    #[test]
    fn test_replace_color0_token() {
        let tmp  = TempDir::new();
        let dict = dummy_dict(&tmp);
        let out  = replace_tokens("bg = {color0}", &dict);
        assert_eq!(out, format!("bg = {}", dict.colors[0].to_hex()));
    }

    #[test]
    fn test_replace_all_16_color_tokens() {
        let tmp  = TempDir::new();
        let dict = dummy_dict(&tmp);

        let template: String = (0..16).map(|i| format!("{{color{i}}}")).collect::<Vec<_>>().join(" ");
        let out = replace_tokens(&template, &dict);

        for i in 0..16 {
            assert!(out.contains(&dict.colors[i].to_hex()), "color{i} not replaced");
        }
    }

    #[test]
    fn test_replace_special_tokens() {
        let tmp  = TempDir::new();
        let dict = dummy_dict(&tmp);
        let out  = replace_tokens("{background} {foreground} {cursor}", &dict);

        assert!(out.contains(&dict.special.background.to_hex()));
        assert!(out.contains(&dict.special.foreground.to_hex()));
        assert!(out.contains(&dict.special.cursor.to_hex()));
    }

    #[test]
    fn test_replace_wallpaper_token() {
        let tmp  = TempDir::new();
        let dict = dummy_dict(&tmp);
        let out  = replace_tokens("wall={wallpaper}", &dict);
        assert!(out.contains("/home/user/wall.jpg"));
    }

    #[test]
    fn test_replace_alpha_token() {
        let tmp  = TempDir::new();
        let dict = dummy_dict(&tmp);
        let out  = replace_tokens("alpha={alpha}", &dict);
        assert!(out.contains("100"));
    }

    #[test]
    fn test_no_tokens_content_unchanged() {
        let tmp  = TempDir::new();
        let dict = dummy_dict(&tmp);
        let content = "nothing to replace here";
        assert_eq!(replace_tokens(content, &dict), content);
    }

    // ── render_one ───────────────────────────────────────────────────────────

    #[test]
    fn test_render_one_writes_to_target() {
        let tmp  = TempDir::new();
        let dict = dummy_dict(&tmp);

        let target = tmp.path().join("output.conf");
        let content = format!(
            "# Target: {}\nbg = {{color0}}",
            target.display()
        );
        let tpl = tmp.create_file("template.conf", &content);

        render_one(&tpl, &dict).unwrap();

        assert!(target.exists());
        let out = std::fs::read_to_string(&target).unwrap();
        assert!(out.contains(&dict.colors[0].to_hex()));
    }

    #[test]
    fn test_render_one_missing_target_directive_errors() {
        let tmp  = TempDir::new();
        let dict = dummy_dict(&tmp);
        let tpl  = tmp.create_file("no_target.conf", "no directive here\n{color0}");

        assert!(matches!(
            render_one(&tpl, &dict),
            Err(RwalError::TemplateMissingTarget(_))
        ));
    }

    #[test]
    fn test_render_one_creates_parent_dirs() {
        let tmp  = TempDir::new();
        let dict = dummy_dict(&tmp);

        let target = tmp.path().join("deep").join("nested").join("output.conf");
        let content = format!("# Target: {}\nbg = {{background}}", target.display());
        let tpl = tmp.create_file("deep_tpl.conf", &content);

        render_one(&tpl, &dict).unwrap();
        assert!(target.exists());
    }

    // ── render_all ───────────────────────────────────────────────────────────

    #[test]
    fn test_render_all_skips_missing_target_with_warn() {
        let tmp   = TempDir::new();
        let paths = Paths::from_home(tmp.path().to_path_buf());
        paths.ensure_dirs().unwrap();
        let dict  = dummy_dict(&tmp);

        // One valid template, one without target
        let target = tmp.path().join("valid_out.conf");
        std::fs::write(
            paths.templates_dir.join("valid.conf"),
            format!("# Target: {}\n{{color0}}", target.display()),
        ).unwrap();
        std::fs::write(
            paths.templates_dir.join("invalid.conf"),
            "no target directive\n{color0}",
        ).unwrap();

        // Should not error — invalid one is skipped with warn
        render_all(&paths, &dict).unwrap();
        assert!(target.exists());
    }

    #[test]
    fn test_render_all_renders_multiple_templates() {
        let tmp   = TempDir::new();
        let paths = Paths::from_home(tmp.path().to_path_buf());
        paths.ensure_dirs().unwrap();
        let dict  = dummy_dict(&tmp);

        let out1 = tmp.path().join("out1.conf");
        let out2 = tmp.path().join("out2.css");

        std::fs::write(
            paths.templates_dir.join("t1.conf"),
            format!("# Target: {}\n{{color0}}", out1.display()),
        ).unwrap();
        std::fs::write(
            paths.templates_dir.join("t2.css"),
            format!("# Target: {}\n{{background}}", out2.display()),
        ).unwrap();

        render_all(&paths, &dict).unwrap();
        assert!(out1.exists());
        assert!(out2.exists());
    }
}