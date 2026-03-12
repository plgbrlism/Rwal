/*

Serializes ColorDict to ~/.cache/rwal/colors.json 
— this is what other tools (waybar scripts, rofi themes, etc.) read

*/
use serde::Serialize;
use std::collections::HashMap;
use crate::colors::types::ColorDict;
use crate::error::RwalError;
use crate::paths::Paths;

/// The exact JSON structure written to ~/.cache/rwal/colors.json.
/// Mirrors pywal's output format so existing tooling stays compatible.
#[derive(Serialize)]
struct ColorsFile<'a> {
    wallpaper: String,
    alpha:     u8,
    special:   SpecialJson<'a>,
    colors:    HashMap<String, &'a str>,
}

#[derive(Serialize)]
struct SpecialJson<'a> {
    background: &'a str,
    foreground: &'a str,
    cursor:     &'a str,
}

/// Write the active `ColorDict` to `~/.cache/rwal/colors.json`.
pub fn write(paths: &Paths, dict: &ColorDict) -> Result<(), RwalError> {
    // Pre-format all hex strings so we can borrow them
    let hex: Vec<String> = dict.colors.iter().map(|c| c.to_hex()).collect();
    let bg  = dict.special.background.to_hex();
    let fg  = dict.special.foreground.to_hex();
    let cur = dict.special.cursor.to_hex();

    let mut colors_map: HashMap<String, &str> = HashMap::new();
    for (i, h) in hex.iter().enumerate() {
        colors_map.insert(format!("color{i}"), h.as_str());
    }

    let file = ColorsFile {
        wallpaper: dict.wallpaper.display().to_string(),
        alpha:     dict.alpha,
        special: SpecialJson {
            background: bg.as_str(),
            foreground: fg.as_str(),
            cursor:     cur.as_str(),
        },
        colors: colors_map,
    };

    let json = serde_json::to_string_pretty(&file)
        .map_err(|e| RwalError::ColorsJsonWriteError(
            paths.colors_json.clone(),
            e.to_string(),
        ))?;

    std::fs::write(&paths.colors_json, json)
        .map_err(|e| RwalError::ColorsJsonWriteError(
            paths.colors_json.clone(),
            e.to_string(),
        ))?;

    Ok(())
}

/// Read colors.json back into a ColorDict.
pub fn read(paths: &Paths) -> Result<ColorDict, RwalError> {
    use crate::colors::types::{Rgb, Special};
    use std::collections::HashMap;

    #[derive(serde::Deserialize)]
    struct ColorsFileRead {
        wallpaper: String,
        alpha:     u8,
        special:   SpecialRead,
        colors:    HashMap<String, String>,
    }

    #[derive(serde::Deserialize)]
    struct SpecialRead {
        background: String,
        foreground: String,
        cursor:     String,
    }

    let contents = std::fs::read_to_string(&paths.colors_json)
        .map_err(|e| RwalError::CacheReadError(
            paths.colors_json.clone(),
            e.to_string(),
        ))?;

    let file: ColorsFileRead = serde_json::from_str(&contents)
        .map_err(|_| RwalError::CacheCorrupted(paths.colors_json.clone()))?;

    let mut colors = [Rgb::new(0, 0, 0); 16];
    for i in 0..16 {
        let key = format!("color{i}");
        let hex = file.colors.get(&key)
            .ok_or_else(|| RwalError::CacheCorrupted(paths.colors_json.clone()))?;
        colors[i] = Rgb::from_hex(hex)
            .ok_or_else(|| RwalError::CacheCorrupted(paths.colors_json.clone()))?;
    }

    Ok(ColorDict {
        wallpaper: std::path::PathBuf::from(file.wallpaper),
        alpha:     file.alpha,
        special:   Special {
            background: Rgb::from_hex(&file.special.background)
                .ok_or_else(|| RwalError::CacheCorrupted(paths.colors_json.clone()))?,
            foreground: Rgb::from_hex(&file.special.foreground)
                .ok_or_else(|| RwalError::CacheCorrupted(paths.colors_json.clone()))?,
            cursor:     Rgb::from_hex(&file.special.cursor)
                .ok_or_else(|| RwalError::CacheCorrupted(paths.colors_json.clone()))?,
        },
        colors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use crate::colors::types::{Rgb, Special};

    struct TempDir { path: PathBuf }

    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "rwal_colorsjson_test_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .subsec_nanos()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
        }
        fn path(&self) -> &Path { &self.path }
    }

    impl Drop for TempDir {
        fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.path); }
    }

    fn fake_paths(tmp: &TempDir) -> Paths {
        let p = Paths::from_home(tmp.path().to_path_buf());
        p.ensure_dirs().unwrap();
        p
    }

    fn dummy_dict() -> ColorDict {
        let mut colors = [Rgb::new(0, 0, 0); 16];
        colors[0]  = Rgb::new(10,  10,  10);
        colors[7]  = Rgb::new(180, 180, 180);
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

    // ── write creates the file ───────────────────────────────────────────────

    #[test]
    fn test_write_creates_colors_json() {
        let tmp   = TempDir::new();
        let paths = fake_paths(&tmp);
        write(&paths, &dummy_dict()).unwrap();
        assert!(paths.colors_json.exists());
    }

    // ── JSON structure ───────────────────────────────────────────────────────

    #[test]
    fn test_written_json_is_valid() {
        let tmp   = TempDir::new();
        let paths = fake_paths(&tmp);
        write(&paths, &dummy_dict()).unwrap();

        let contents = std::fs::read_to_string(&paths.colors_json).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
        assert!(parsed.is_object());
    }

    #[test]
    fn test_written_json_has_all_16_colors() {
        let tmp   = TempDir::new();
        let paths = fake_paths(&tmp);
        write(&paths, &dummy_dict()).unwrap();

        let contents = std::fs::read_to_string(&paths.colors_json).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();

        for i in 0..16 {
            let key = format!("color{i}");
            assert!(
                parsed["colors"][&key].is_string(),
                "missing {key} in colors"
            );
        }
    }

    #[test]
    fn test_written_json_has_special_keys() {
        let tmp   = TempDir::new();
        let paths = fake_paths(&tmp);
        write(&paths, &dummy_dict()).unwrap();

        let contents = std::fs::read_to_string(&paths.colors_json).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();

        assert!(parsed["special"]["background"].is_string());
        assert!(parsed["special"]["foreground"].is_string());
        assert!(parsed["special"]["cursor"].is_string());
    }

    #[test]
    fn test_written_json_has_wallpaper_and_alpha() {
        let tmp   = TempDir::new();
        let paths = fake_paths(&tmp);
        write(&paths, &dummy_dict()).unwrap();

        let contents = std::fs::read_to_string(&paths.colors_json).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();

        assert_eq!(parsed["wallpaper"], "/home/user/wall.jpg");
        assert_eq!(parsed["alpha"], 100);
    }

    // ── hex format ───────────────────────────────────────────────────────────

    #[test]
    fn test_colors_are_hex_format() {
        let tmp   = TempDir::new();
        let paths = fake_paths(&tmp);
        write(&paths, &dummy_dict()).unwrap();

        let contents = std::fs::read_to_string(&paths.colors_json).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();

        for i in 0..16 {
            let key = format!("color{i}");
            let val = parsed["colors"][&key].as_str().unwrap();
            assert!(val.starts_with('#'), "color{i} should start with #");
            assert_eq!(val.len(), 7, "color{i} should be #rrggbb");
        }
    }

    #[test]
    fn test_color0_matches_dict() {
        let tmp   = TempDir::new();
        let paths = fake_paths(&tmp);
        let dict  = dummy_dict();
        write(&paths, &dict).unwrap();

        let contents = std::fs::read_to_string(&paths.colors_json).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();

        assert_eq!(
            parsed["colors"]["color0"].as_str().unwrap(),
            dict.colors[0].to_hex()
        );
    }

    // ── overwrite ────────────────────────────────────────────────────────────

    #[test]
    fn test_write_overwrites_existing_file() {
        let tmp   = TempDir::new();
        let paths = fake_paths(&tmp);

        write(&paths, &dummy_dict()).unwrap();

        let mut dict2 = dummy_dict();
        dict2.alpha = 75;
        write(&paths, &dict2).unwrap();

        let contents = std::fs::read_to_string(&paths.colors_json).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
        assert_eq!(parsed["alpha"], 75);
    }

    // ── read testcases ────────────────────────────────────────────────────────────────
    #[test]
    fn test_read_roundtrip_matches_written_dict() {
        let tmp   = TempDir::new();
        let paths = fake_paths(&tmp);
        let dict  = dummy_dict();

        write(&paths, &dict).unwrap();
        let loaded = read(&paths).unwrap();

        assert_eq!(loaded.wallpaper, dict.wallpaper);
        assert_eq!(loaded.alpha,     dict.alpha);
        assert_eq!(loaded.special.background, dict.special.background);
        assert_eq!(loaded.special.foreground, dict.special.foreground);
        assert_eq!(loaded.special.cursor,     dict.special.cursor);
        assert_eq!(loaded.colors, dict.colors);
    }

    #[test]
    fn test_read_missing_file_errors() {
        let tmp   = TempDir::new();
        let paths = fake_paths(&tmp);
        assert!(matches!(
            read(&paths),
            Err(RwalError::CacheReadError(_, _))
        ));
    }

    #[test]
    fn test_read_corrupted_file_errors() {
        let tmp   = TempDir::new();
        let paths = fake_paths(&tmp);
        std::fs::write(&paths.colors_json, b"not valid json {{{{").unwrap();
        assert!(matches!(
            read(&paths),
            Err(RwalError::CacheCorrupted(_))
        ));
    }

    #[test]
    fn test_read_all_16_colors_correct() {
        let tmp   = TempDir::new();
        let paths = fake_paths(&tmp);
        let mut dict = dummy_dict();

        // give each slot a unique color
        for i in 0..16 {
            dict.colors[i] = Rgb::new(i as u8 * 15, i as u8 * 10, i as u8 * 5);
        }

        write(&paths, &dict).unwrap();
        let loaded = read(&paths).unwrap();

        for i in 0..16 {
            assert_eq!(
                loaded.colors[i], dict.colors[i],
                "color{i} mismatch after roundtrip"
            );
        }
    }
    
}





