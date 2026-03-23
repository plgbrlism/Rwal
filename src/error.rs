/*
One for all RwalError for catching all following errors in one place.

ImageLoad, 
NoColors, 
CacheRead, 
CacheWrite, 
TemplateRender, 
WallpaperSet, 
UnsupportedBackend

*/

use std::path::PathBuf;
use std::fmt;

#[derive(Debug)]
pub enum RwalError {
    // Image loading
    ImageNotFound(PathBuf),

    EmptyDirectory(PathBuf),
    ImageDecodeError(String),

    // Color extraction
    NoColorsExtracted,
    BackendFailed(String),
    UnsupportedBackend(String),

    // Cache
    CacheReadError(PathBuf, String),
    CacheWriteError(PathBuf, String),
    CacheCorrupted(PathBuf),

    // Export
    TemplateReadError(PathBuf, String),
    TemplateWriteError(PathBuf, String),
    ColorsJsonWriteError(PathBuf, String),
    SequenceWriteError(String),
    SymlinkFailed(PathBuf, PathBuf, String),

    // Wallpaper
    WallpaperSetFailed(String),
    NoCompositorDetected,

    // Paths / IO
    HomeDirNotFound,
    CreateDirFailed(PathBuf, String),
    IoError(String),
    Custom(String),
}

impl fmt::Display for RwalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Image loading
            RwalError::ImageNotFound(p) =>
                write!(f, "Image not found: {}", p.display()),

            RwalError::EmptyDirectory(p) =>
                write!(f, "No valid images found in directory: {}", p.display()),
            RwalError::ImageDecodeError(msg) =>
                write!(f, "Failed to decode image: {msg}"),

            // Color extraction
            RwalError::NoColorsExtracted =>
                write!(f, "Backend returned no colors — image may be too uniform"),
            RwalError::BackendFailed(msg) =>
                write!(f, "Color backend failed: {msg}"),
            RwalError::UnsupportedBackend(name) =>
                write!(f, "Unknown backend '{name}' — available: kmeans, median_cut"),

            // Cache
            RwalError::CacheReadError(p, msg) =>
                write!(f, "Could not read cache at {}: {msg}", p.display()),
            RwalError::CacheWriteError(p, msg) =>
                write!(f, "Could not write cache to {}: {msg}", p.display()),
            RwalError::CacheCorrupted(p) =>
                write!(f, "Cache file is corrupted, will regenerate: {}", p.display()),

            // Export
            RwalError::TemplateReadError(p, msg) =>
                write!(f, "Could not read template {}: {msg}", p.display()),
            RwalError::TemplateWriteError(p, msg) =>
                write!(f, "Could not write rendered template to {}: {msg}", p.display()),
            RwalError::ColorsJsonWriteError(p, msg) =>
                write!(f, "Could not write colors.json to {}: {msg}", p.display()),
            RwalError::SequenceWriteError(msg) =>
                write!(f, "Could not write terminal sequences: {msg}"),
            RwalError::SymlinkFailed(src, dst, msg) =>
                write!(f, "Could not symlink {} to {}: {msg}", src.display(), dst.display()),

            // Wallpaper
            RwalError::WallpaperSetFailed(msg) =>
                write!(f, "Failed to set wallpaper: {msg}"),
            RwalError::NoCompositorDetected =>
                write!(f, "Could not detect a supported compositor (sway, feh, nitrogen)"),

            // Paths / IO
            RwalError::HomeDirNotFound =>
                write!(f, "Could not resolve home directory"),
            RwalError::CreateDirFailed(p, msg) =>
                write!(f, "Could not create directory {}: {msg}", p.display()),
            RwalError::IoError(msg) =>
                write!(f, "IO error: {msg}"),
            RwalError::Custom(msg) =>
                write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for RwalError {}

/// Print a warning to stderr and continue — used for non-fatal errors.
pub fn warn(err: &RwalError) {
    eprintln!("\x1b[33mwarn\x1b[0m: {err}");
}

/// Print an error to stderr — used before giving up on a pipeline step.
pub fn error(err: &RwalError) {
    eprintln!("\x1b[31merror\x1b[0m: {err}");
}

/// UNIT TESTS FOR RwalError
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_not_found_display() {
        let err = RwalError::ImageNotFound(PathBuf::from("/tmp/ghost.png"));
        assert_eq!(err.to_string(), "Image not found: /tmp/ghost.png");
    }



    #[test]
    fn test_empty_directory_display() {
        let err = RwalError::EmptyDirectory(PathBuf::from("/tmp/pics"));
        assert!(err.to_string().contains("/tmp/pics"));
    }

    #[test]
    fn test_no_colors_extracted_display() {
        let err = RwalError::NoColorsExtracted;
        assert!(err.to_string().contains("no colors"));
    }

    #[test]
    fn test_unsupported_backend_display() {
        let err = RwalError::UnsupportedBackend("magic".into());
        let msg = err.to_string();
        assert!(msg.contains("magic"));
        assert!(msg.contains("kmeans"));
    }

    #[test]
    fn test_cache_read_error_display() {
        let err = RwalError::CacheReadError(
            PathBuf::from("/home/user/.cache/rwal/schemes/abc.json"),
            "permission denied".into(),
        );
        let msg = err.to_string();
        assert!(msg.contains("abc.json"));
        assert!(msg.contains("permission denied"));
    }

    #[test]
    fn test_cache_write_error_display() {
        let err = RwalError::CacheWriteError(
            PathBuf::from("/home/user/.cache/rwal/schemes/abc.json"),
            "disk full".into(),
        );
        assert!(err.to_string().contains("disk full"));
    }

    #[test]
    fn test_cache_corrupted_display() {
        let err = RwalError::CacheCorrupted(PathBuf::from("/tmp/bad.json"));
        assert!(err.to_string().contains("corrupted"));
    }

    #[test]
    fn test_wallpaper_set_failed_display() {
        let err = RwalError::WallpaperSetFailed("feh exited with code 1".into());
        assert!(err.to_string().contains("feh exited with code 1"));
    }

    #[test]
    fn test_no_compositor_detected_display() {
        let err = RwalError::NoCompositorDetected;
        let msg = err.to_string();
        assert!(msg.contains("sway"));
        assert!(msg.contains("feh"));
        assert!(msg.contains("nitrogen"));
    }

    #[test]
    fn test_home_dir_not_found_display() {
        let err = RwalError::HomeDirNotFound;
        assert!(err.to_string().contains("home directory"));
    }

    #[test]
    fn test_create_dir_failed_display() {
        let err = RwalError::CreateDirFailed(
            PathBuf::from("/root/forbidden"),
            "permission denied".into(),
        );
        let msg = err.to_string();
        assert!(msg.contains("/root/forbidden"));
        assert!(msg.contains("permission denied"));
    }

    #[test]
    fn test_io_error_display() {
        let err = RwalError::IoError("broken pipe".into());
        assert!(err.to_string().contains("broken pipe"));
    }

    #[test]
    fn test_sequence_write_error_display() {
        let err = RwalError::SequenceWriteError("no pts found".into());
        assert!(err.to_string().contains("no pts found"));
    }

    #[test]
    fn test_implements_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(RwalError::NoColorsExtracted);
        let _ = err.to_string();
    }

    #[test]
    fn test_all_variants_are_debug() {
        let variants: Vec<Box<dyn std::fmt::Debug>> = vec![
            Box::new(RwalError::ImageNotFound(PathBuf::from("/a"))),

            Box::new(RwalError::EmptyDirectory(PathBuf::from("/b"))),
            Box::new(RwalError::ImageDecodeError("bad".into())),
            Box::new(RwalError::NoColorsExtracted),
            Box::new(RwalError::BackendFailed("x".into())),
            Box::new(RwalError::UnsupportedBackend("x".into())),
            Box::new(RwalError::CacheReadError(PathBuf::from("/c"), "x".into())),
            Box::new(RwalError::CacheWriteError(PathBuf::from("/d"), "x".into())),
            Box::new(RwalError::CacheCorrupted(PathBuf::from("/e"))),
            Box::new(RwalError::TemplateReadError(PathBuf::from("/f"), "x".into())),
            Box::new(RwalError::TemplateWriteError(PathBuf::from("/g"), "x".into())),
            Box::new(RwalError::ColorsJsonWriteError(PathBuf::from("/i"), "x".into())),
            Box::new(RwalError::SequenceWriteError("x".into())),
            Box::new(RwalError::SymlinkFailed(PathBuf::from("/a"), PathBuf::from("/b"), "x".into())),
            Box::new(RwalError::WallpaperSetFailed("x".into())),
            Box::new(RwalError::NoCompositorDetected),
            Box::new(RwalError::HomeDirNotFound),
            Box::new(RwalError::CreateDirFailed(PathBuf::from("/j"), "x".into())),
            Box::new(RwalError::IoError("x".into())),
        ];
        for v in &variants {
            let _ = format!("{v:?}");
        }
    }
}