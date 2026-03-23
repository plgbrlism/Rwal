/*
Cache key is a hash of: image_path + backend_name + light_flag + saturate_value + file_size_bytes
Stored as: ~/.cache/rwal/schemes/<hash>.json
On hit: deserialize and return ColorDict immediately, skip all extraction.
On miss: run pipeline, serialize ColorDict, save.

*/
use std::path::{Path, PathBuf};
use sha1::{Sha1, Digest};
use crate::colors::types::ColorDict;
use crate::error::{RwalError, warn};
use crate::paths::Paths;

/// Build a cache key from all inputs that affect the output palette.
/// Any change to these inputs produces a different hash → new cache entry.
pub fn cache_key(
    image_path: &Path,
    backend:    &str,
    mode:       &str,
    light_mode: bool,
    file_size:  u64,
) -> String {
    // Content-based hash: first 64KB + metadata
    // This allows renaming/moving files without losing cache.
    let mut hasher = Sha1::new();
    
    // 1. Metadata that affects palette
    hasher.update(backend.as_bytes());
    hasher.update(mode.as_bytes());
    hasher.update(if light_mode { b"light" as &[u8] } else { b"dark" as &[u8] });
    hasher.update(&file_size.to_le_bytes());

    // 2. Partial file content (first 64KB)
    if let Ok(mut file) = std::fs::File::open(image_path) {
        use std::io::Read;
        let mut buffer = [0u8; 65536];
        if let Ok(n) = file.read(&mut buffer) {
            hasher.update(&buffer[..n]);
        }
    }

    let hash = hasher.finalize();
    format!("{hash:x}")
}

/// Try to load a cached `ColorDict` for the given key.
///
/// Returns:
/// - `Ok(Some(dict))` — cache hit, valid data
/// - `Ok(None)`       — cache miss, file does not exist
/// - On corrupted file: deletes it, warns, returns `Ok(None)` to trigger regeneration
pub fn load(paths: &Paths, key: &str) -> Result<Option<ColorDict>, RwalError> {
    let path = paths.scheme_cache(key);

    if !path.exists() {
        return Ok(None);
    }

    match read_cache(&path) {
        Ok(dict) => Ok(Some(dict)),
        Err(e) => {
            // Corrupted — delete and let caller regenerate
            warn(&RwalError::CacheCorrupted(path.clone()));
            let _ = std::fs::remove_file(&path);
            Err(e)
        }
    }
}

/// Save a `ColorDict` to the cache under the given key.
pub fn save(paths: &Paths, key: &str, dict: &ColorDict) -> Result<(), RwalError> {
    let path = paths.scheme_cache(key);

    let json = serde_json::to_string_pretty(dict)
        .map_err(|e| RwalError::CacheWriteError(path.clone(), e.to_string()))?;

    std::fs::write(&path, json)
        .map_err(|e| RwalError::CacheWriteError(path.clone(), e.to_string()))?;

    Ok(())
}

/// Read and deserialize a cache file.
fn read_cache(path: &PathBuf) -> Result<ColorDict, RwalError> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| RwalError::CacheReadError(path.clone(), e.to_string()))?;

    serde_json::from_str(&contents)
        .map_err(|_| RwalError::CacheCorrupted(path.clone()))
}

/// Resolve the image file size in bytes — part of the cache key
/// so stale cache is invalidated when the file changes.
pub fn file_size(path: &Path) -> u64 {
    std::fs::metadata(path)
        .map(|m| m.len())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use crate::colors::types::{Rgb, Special};

    // ── temp dir helper ──────────────────────────────────────────────────────

    struct TempDir { path: PathBuf }

    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "rwal_cache_test_{}",
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
        ColorDict {
            wallpaper: PathBuf::from("/tmp/wall.jpg"),
            alpha: 100,
            special: Special {
                background: Rgb::new(0, 0, 0),
                foreground: Rgb::new(255, 255, 255),
                cursor:     Rgb::new(255, 255, 255),
            },
            colors: [Rgb::new(10, 20, 30); 16],
        }
    }

    // ── cache_key ────────────────────────────────────────────────────────────

    #[test]
    fn test_cache_key_is_deterministic() {
        let p = Path::new("/home/user/wall.jpg");
        let k1 = cache_key(p, "kmeans", "classic", false, 12345);
        let k2 = cache_key(p, "kmeans", "classic", false, 12345);
        assert_eq!(k1, k2);
    }

    #[test]
    fn test_cache_key_differs_by_backend() {
        let p = Path::new("/home/user/wall.jpg");
        let k1 = cache_key(p, "kmeans",     "classic", false, 100);
        let k2 = cache_key(p, "median_cut", "classic", false, 100);
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_cache_key_differs_by_light_mode() {
        let p = Path::new("/home/user/wall.jpg");
        let k1 = cache_key(p, "kmeans", "classic", false, 100);
        let k2 = cache_key(p, "kmeans", "classic", true,  100);
        assert_ne!(k1, k2);
    }



    #[test]
    fn test_cache_key_differs_by_file_size() {
        let p = Path::new("/home/user/wall.jpg");
        let k1 = cache_key(p, "kmeans", "classic", false, 100);
        let k2 = cache_key(p, "kmeans", "classic", false, 999);
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_cache_key_differs_by_path_if_content_differs() {
        let tmp = TempDir::new();
        let p1 = tmp.path().join("a.jpg");
        let p2 = tmp.path().join("b.jpg");
        std::fs::write(&p1, b"content a").unwrap();
        std::fs::write(&p2, b"content b").unwrap();
        
        let k1 = cache_key(&p1, "kmeans", "classic", false, 100);
        let k2 = cache_key(&p2, "kmeans", "classic", false, 100);
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_cache_key_same_if_content_identical_despite_path() {
        let tmp = TempDir::new();
        let p1 = tmp.path().join("a.jpg");
        let p2 = tmp.path().join("b.jpg");
        let content = b"shared content";
        std::fs::write(&p1, content).unwrap();
        std::fs::write(&p2, content).unwrap();
        
        let k1 = cache_key(&p1, "kmeans", "classic", false, content.len() as u64);
        let k2 = cache_key(&p2, "kmeans", "classic", false, content.len() as u64);
        assert_eq!(k1, k2, "Cache key should be identical for same content despite different paths");
    }

    #[test]
    fn test_cache_key_is_hex_string() {
        let key = cache_key(Path::new("/wall.jpg"), "kmeans", "classic", false, 0);
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // ── save + load roundtrip ────────────────────────────────────────────────

    #[test]
    fn test_save_and_load_roundtrip() {
        let tmp   = TempDir::new();
        let paths = fake_paths(&tmp);
        let dict  = dummy_dict();
        let key   = "testroundtrip";

        save(&paths, key, &dict).unwrap();
        let loaded = load(&paths, key).unwrap().unwrap();
        assert_eq!(dict, loaded);
    }

    #[test]
    fn test_load_returns_none_on_miss() {
        let tmp   = TempDir::new();
        let paths = fake_paths(&tmp);
        let result = load(&paths, "nonexistent_key").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_corrupted_cache_is_deleted_and_returns_err() {
        let tmp   = TempDir::new();
        let paths = fake_paths(&tmp);
        let key   = "corruptedkey";

        // Write garbage into the cache file
        let path = paths.scheme_cache(key);
        std::fs::write(&path, b"not valid json {{{{").unwrap();

        // Should return an error and delete the file
        let result = load(&paths, key);
        assert!(result.is_err());
        assert!(!path.exists(), "corrupted cache file should be deleted");
    }

    #[test]
    fn test_save_overwrites_existing_cache() {
        let tmp   = TempDir::new();
        let paths = fake_paths(&tmp);
        let key   = "overwrite_test";

        let dict1 = dummy_dict();
        save(&paths, key, &dict1).unwrap();

        let mut dict2 = dummy_dict();
        dict2.alpha = 50;
        save(&paths, key, &dict2).unwrap();

        let loaded = load(&paths, key).unwrap().unwrap();
        assert_eq!(loaded.alpha, 50);
    }

    // ── file_size ────────────────────────────────────────────────────────────

    #[test]
    fn test_file_size_returns_correct_size() {
        let tmp  = TempDir::new();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, b"hello world").unwrap();
        assert_eq!(file_size(&path), 11);
    }

    #[test]
    fn test_file_size_returns_zero_for_missing_file() {
        assert_eq!(file_size(Path::new("/tmp/does_not_exist_rwal")), 0);
    }
}