/*
Paths used by rwal:

~/.cache/rwal/colors.json       // active color dict
~/.cache/rwal/sequences         // OSC sequence file (sourced on login)
~/.cache/rwal/schemes/          // per-image cached palettes
~/.config/rwal/templates/       // user templates
~/.config/rwal/themes/          // saved named themes

*/

use std::path::PathBuf;
use crate::error::RwalError;

/// All canonical paths used by rwal, resolved at runtime from $HOME.
pub struct Paths {
    // ~/.cache/rwal/
    pub cache_dir:     PathBuf,
    pub colors_json:   PathBuf,
    pub sequences:     PathBuf,
    pub schemes_dir:   PathBuf,

    // ~/.config/rwal/
    pub config_dir:    PathBuf,
    pub templates_dir: PathBuf,
    pub themes_dir:    PathBuf,
    pub colorschemes_dir: PathBuf,
}

impl Paths {
    /// Resolve all paths from the user's home directory.
    pub fn resolve() -> Result<Self, RwalError> {
        let home = dirs::home_dir().ok_or(RwalError::HomeDirNotFound)?;
        Ok(Self::from_home(home))
    }

    /// Build all paths from an explicit home directory.
    /// Used by tests to inject a temp dir instead of real $HOME.
    pub fn from_home(home: PathBuf) -> Self {
        let cache_dir  = home.join(".cache").join("rwal");
        let config_dir = home.join(".config").join("rwal");

        Self {
            colors_json:   cache_dir.join("colors.json"),
            sequences:     cache_dir.join("sequences"),
            schemes_dir:   cache_dir.join("schemes"),
            cache_dir,

            templates_dir: config_dir.join("templates"),
            themes_dir:    config_dir.join("themes"),
            colorschemes_dir: config_dir.join("default_colorschemes"),
            config_dir,
        }
    }

    /// Ensure all required directories exist, creating them if needed.
    pub fn ensure_dirs(&self) -> Result<(), RwalError> {
        let dirs = [
            &self.cache_dir,
            &self.schemes_dir,
            &self.config_dir,
            &self.templates_dir,
            &self.themes_dir,
            &self.colorschemes_dir,
        ];

        for dir in dirs {
            if !dir.exists() {
                std::fs::create_dir_all(dir).map_err(|e| {
                    RwalError::CreateDirFailed(dir.to_path_buf(), e.to_string())
                })?;
            }
        }

        Ok(())
    }

    /// Path for a cached color scheme, keyed by a pre-computed hash string.
    pub fn scheme_cache(&self, hash: &str) -> PathBuf {
        self.schemes_dir.join(format!("{hash}.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn fake_home() -> PathBuf {
        PathBuf::from("/home/testuser")
    }

    fn paths() -> Paths {
        Paths::from_home(fake_home())
    }

    #[test]
    fn test_cache_dir_is_under_home() {
        let p = paths();
        assert!(p.cache_dir.starts_with(fake_home()));
        assert!(p.cache_dir.ends_with(".cache/rwal"));
    }

    #[test]
    fn test_config_dir_is_under_home() {
        let p = paths();
        assert!(p.config_dir.starts_with(fake_home()));
        assert!(p.config_dir.ends_with(".config/rwal"));
    }

    #[test]
    fn test_colors_json_is_under_cache_dir() {
        let p = paths();
        assert!(p.colors_json.starts_with(&p.cache_dir));
        assert_eq!(p.colors_json.file_name().unwrap(), "colors.json");
    }

    #[test]
    fn test_sequences_is_under_cache_dir() {
        let p = paths();
        assert!(p.sequences.starts_with(&p.cache_dir));
        assert_eq!(p.sequences.file_name().unwrap(), "sequences");
    }

    #[test]
    fn test_schemes_dir_is_under_cache_dir() {
        let p = paths();
        assert!(p.schemes_dir.starts_with(&p.cache_dir));
        assert!(p.schemes_dir.ends_with("schemes"));
    }

    #[test]
    fn test_templates_dir_is_under_config_dir() {
        let p = paths();
        assert!(p.templates_dir.starts_with(&p.config_dir));
        assert!(p.templates_dir.ends_with("templates"));
    }

    #[test]
    fn test_themes_dir_is_under_config_dir() {
        let p = paths();
        assert!(p.themes_dir.starts_with(&p.config_dir));
        assert!(p.themes_dir.ends_with("themes"));
    }

    #[test]
    fn test_different_homes_produce_different_paths() {
        let p1 = Paths::from_home(PathBuf::from("/home/alice"));
        let p2 = Paths::from_home(PathBuf::from("/home/bob"));
        assert_ne!(p1.cache_dir, p2.cache_dir);
        assert_ne!(p1.colors_json, p2.colors_json);
    }

    #[test]
    fn test_scheme_cache_has_json_extension() {
        let p = paths();
        assert_eq!(p.scheme_cache("abc123").extension().unwrap(), "json");
    }

    #[test]
    fn test_scheme_cache_is_under_schemes_dir() {
        let p = paths();
        assert!(p.scheme_cache("abc123").starts_with(&p.schemes_dir));
    }

    #[test]
    fn test_scheme_cache_filename_contains_hash() {
        let p = paths();
        let hash = "deadbeef1234";
        let name = p.scheme_cache(hash)
            .file_name().unwrap()
            .to_str().unwrap()
            .to_string();
        assert!(name.contains(hash));
    }

    #[test]
    fn test_scheme_cache_different_hashes_differ() {
        let p = paths();
        assert_ne!(p.scheme_cache("hash_a"), p.scheme_cache("hash_b"));
    }

    #[test]
    fn test_scheme_cache_same_hash_is_deterministic() {
        let p = paths();
        assert_eq!(p.scheme_cache("stable"), p.scheme_cache("stable"));
    }

    struct TempDir { path: PathBuf }

    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "rwal_test_{}",
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

    #[test]
    fn test_ensure_dirs_creates_all_directories() {
        let tmp = TempDir::new();
        let p = Paths::from_home(tmp.path().to_path_buf());
        p.ensure_dirs().expect("ensure_dirs should not fail");

        assert!(p.cache_dir.exists(),     "cache_dir missing");
        assert!(p.schemes_dir.exists(),   "schemes_dir missing");
        assert!(p.config_dir.exists(),    "config_dir missing");
        assert!(p.templates_dir.exists(), "templates_dir missing");
        assert!(p.themes_dir.exists(),    "themes_dir missing");
    }

    #[test]
    fn test_ensure_dirs_is_idempotent() {
        let tmp = TempDir::new();
        let p = Paths::from_home(tmp.path().to_path_buf());
        p.ensure_dirs().expect("first call failed");
        p.ensure_dirs().expect("second call should not fail");
    }
}