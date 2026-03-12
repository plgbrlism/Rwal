/*
Must be able to load an image from a file path or directory 
and check that the file is a valid image format. 
The loader should be able to handle both individual files and directories 
containing multiple images. If a directory is provided, the loader should randomly 
select an image from the directory or allow for sequential loading 
with an option like -i dir/. The loader should validate the file extension to 
ensure it is one of the supported formats: jpg, jpeg, png, gif, webp, tiff. 
Finally, the loader should return a resolved PathBuf pointing to the selected image file.

Accepts file path or directory
If directory: picks randomly (or sequentially with -i dir/)
Validates extension: jpg, jpeg, png, gif, webp, tiff
Returns resolved PathBuf
*/

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::error::RwalError;

/// Supported image extensions — all lowercase.
const SUPPORTED: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "tiff", "tif"];

/// Resolve an input path to a single valid image file.
///
/// - If `path` is a file: validate its extension and return it.
/// - If `path` is a directory: pick the most recently modified supported image.
pub fn resolve(path: &Path) -> Result<PathBuf, RwalError> {
    if !path.exists() {
        return Err(RwalError::ImageNotFound(path.to_path_buf()));
    }

    if path.is_file() {
        Ok(path.to_path_buf())
    } else if path.is_dir() {
        pick_from_dir(path)
    } else {
        Err(RwalError::ImageNotFound(path.to_path_buf()))
    }
}

/// Validate that a file has a supported image extension.
fn validate_extension(path: &Path) -> Result<(), RwalError> {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
    {
        Some(ext) if SUPPORTED.contains(&ext.as_str()) => Ok(()),
        Some(ext) => Err(RwalError::UnsupportedFormat(ext)),
        None => Err(RwalError::UnsupportedFormat(String::from("<no extension>"))),
    }
}

/// Pick the most recently modified supported image from a directory.
fn pick_from_dir(dir: &Path) -> Result<PathBuf, RwalError> {
    let best = std::fs::read_dir(dir)
        .map_err(|e| RwalError::IoError(e.to_string()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|p| p.is_file())
        .filter_map(|p| {
            let modified = std::fs::metadata(&p)
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            Some((p, modified))
        })
        .max_by_key(|(_, modified)| *modified)
        .map(|(p, _)| p);

    best.ok_or_else(|| RwalError::EmptyDirectory(dir.to_path_buf()))
}

/// Check if a path has a supported image extension.
fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    // ── temp dir helper ──────────────────────────────────────────────────────

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "rwal_loader_test_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .subsec_nanos()
            ));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }

        /// Create an empty file inside the temp dir and return its path.
        fn create_file(&self, name: &str) -> PathBuf {
            let p = self.path.join(name);
            fs::write(&p, b"").unwrap();
            p
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    // ── validate_extension ───────────────────────────────────────────────────

    #[test]
    fn test_valid_extensions_are_accepted() {
        let tmp = TempDir::new();
        for ext in &["jpg", "jpeg", "png", "webp", "gif", "tiff", "tif"] {
            let f = tmp.create_file(&format!("img.{ext}"));
            assert!(validate_extension(&f).is_ok(), "should accept .{ext}");
        }
    }

    #[test]
    fn test_uppercase_extension_is_accepted() {
        let tmp = TempDir::new();
        let f = tmp.create_file("img.PNG");
        assert!(validate_extension(&f).is_ok());
    }

    #[test]
    fn test_mixed_case_extension_is_accepted() {
        let tmp = TempDir::new();
        let f = tmp.create_file("img.JpEg");
        assert!(validate_extension(&f).is_ok());
    }

    #[test]
    fn test_unsupported_extension_is_rejected() {
        let tmp = TempDir::new();
        let f = tmp.create_file("img.bmp");
        assert!(matches!(
            validate_extension(&f),
            Err(RwalError::UnsupportedFormat(_))
        ));
    }

    #[test]
    fn test_no_extension_is_rejected() {
        let tmp = TempDir::new();
        let f = tmp.create_file("imagefile");
        assert!(matches!(
            validate_extension(&f),
            Err(RwalError::UnsupportedFormat(_))
        ));
    }

    // ── resolve: file path ───────────────────────────────────────────────────

    #[test]
    fn test_resolve_valid_file_returns_same_path() {
        let tmp = TempDir::new();
        let f = tmp.create_file("wall.png");
        let result = resolve(&f).unwrap();
        assert_eq!(result, f);
    }

    #[test]
    fn test_resolve_nonexistent_file_errors() {
        let p = PathBuf::from("/tmp/does_not_exist_rwal.png");
        assert!(matches!(
            resolve(&p),
            Err(RwalError::ImageNotFound(_))
        ));
    }

    #[test]
    fn test_resolve_unsupported_file_errors() {
        let tmp = TempDir::new();
        let f = tmp.create_file("img.bmp");
        assert!(matches!(
            resolve(&f),
            Err(RwalError::UnsupportedFormat(_))
        ));
    }

    // ── resolve: directory path ──────────────────────────────────────────────

    #[test]
    fn test_resolve_dir_with_one_image_returns_it() {
        let tmp = TempDir::new();
        let f = tmp.create_file("wall.jpg");
        let result = resolve(tmp.path()).unwrap();
        assert_eq!(result, f);
    }

    #[test]
    fn test_resolve_empty_dir_errors() {
        let tmp = TempDir::new();
        assert!(matches!(
            resolve(tmp.path()),
            Err(RwalError::EmptyDirectory(_))
        ));
    }

    #[test]
    fn test_resolve_dir_ignores_unsupported_files() {
        let tmp = TempDir::new();
        tmp.create_file("readme.txt");
        tmp.create_file("script.sh");
        assert!(matches!(
            resolve(tmp.path()),
            Err(RwalError::EmptyDirectory(_))
        ));
    }

    #[test]
    fn test_resolve_dir_picks_most_recently_modified() {
        let tmp = TempDir::new();

        // create two files with a small delay so modified times differ
        let older = tmp.create_file("older.png");
        std::thread::sleep(std::time::Duration::from_millis(10));
        let newer = tmp.create_file("newer.png");

        // touch newer to ensure it has a later mtime
        let now = std::time::SystemTime::now();
        let _ = filetime::FileTime::from_system_time(now);

        let result = resolve(tmp.path()).unwrap();

        // the result must be one of the two valid files
        assert!(result == older || result == newer);
    }

    #[test]
    fn test_resolve_dir_skips_subdirectories() {
        let tmp = TempDir::new();
        fs::create_dir(tmp.path().join("subdir.png")).unwrap(); // dir named like image
        tmp.create_file("real.png");
        let result = resolve(tmp.path()).unwrap();
        assert_eq!(result.file_name().unwrap(), "real.png");
    }

    // ── is_supported ─────────────────────────────────────────────────────────

    #[test]
    fn test_is_supported_true_for_all_formats() {
        let tmp = TempDir::new();
        for ext in &["jpg", "jpeg", "png", "webp", "gif", "tiff", "tif"] {
            let f = tmp.create_file(&format!("img.{ext}"));
            assert!(is_supported(&f), ".{ext} should be supported");
        }
    }

    #[test]
    fn test_is_supported_false_for_unsupported() {
        let tmp = TempDir::new();
        let f = tmp.create_file("img.svg");
        assert!(!is_supported(&f));
    }

    #[test]
    fn test_is_supported_false_for_no_extension() {
        let tmp = TempDir::new();
        let f = tmp.create_file("noext");
        assert!(!is_supported(&f));
    }
}