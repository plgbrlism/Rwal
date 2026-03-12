/*

Opens image with image crate
Resizes to 200×200 (speed — pywal does the same)
Collects all pixels as Vec<Rgb>
Calls the chosen backend's generate()
Returns Vec<Rgb> of raw dominant colors

*/
use std::path::Path;
use image::DynamicImage;
use crate::colors::types::Rgb;
use crate::error::RwalError;

const SAMPLE_SIZE: u32 = 200;

/// Open an image file from disk, detecting format from content not extension.
pub fn open(path: &Path) -> Result<DynamicImage, RwalError> {
    image::io::Reader::open(path)
        .map_err(|e| RwalError::ImageDecodeError(e.to_string()))?
        .with_guessed_format()
        .map_err(|e| RwalError::ImageDecodeError(e.to_string()))?
        .decode()
        .map_err(|e| RwalError::ImageDecodeError(e.to_string()))
}

/// Resize image to 200×200 and collect all pixels as Vec<Rgb>.
/// This is the same downsampling strategy pywal uses for performance.
pub fn sample_pixels(img: &DynamicImage) -> Vec<Rgb> {
    let resized = img.resize(
        SAMPLE_SIZE,
        SAMPLE_SIZE,
        image::imageops::FilterType::Nearest,
    );

    resized
        .to_rgb8()
        .pixels()
        .map(|p| Rgb::new(p[0], p[1], p[2]))
        .collect()
}

/// Full pipeline: open → sample → backend generate → Vec<Rgb>.
/// This is the convenience function main.rs calls.
pub fn extract(
    path: &Path,
    backend: &dyn crate::backends::Backend,
    count: usize,
    iterations: u8,
) -> Result<Vec<Rgb>, RwalError> {
    let img    = open(path)?;
    let pixels = sample_pixels(&img);
    backend.generate(&pixels, count, iterations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb as ImageRgb};
    use std::path::PathBuf;

    // ── helpers ──────────────────────────────────────────────────────────────

    struct TempDir { path: PathBuf }

    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "rwal_extractor_test_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .subsec_nanos()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.path); }
    }

    /// Create a solid-color PNG image of given size and save it to disk.
    fn solid_image(tmp: &TempDir, name: &str, r: u8, g: u8, b: u8, w: u32, h: u32) -> PathBuf {
        let img: ImageBuffer<ImageRgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(w, h, |_, _| ImageRgb([r, g, b]));
        let path = tmp.path.join(name);
        img.save(&path).unwrap();
        path
    }

    // ── open ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_open_valid_image() {
        let tmp  = TempDir::new();
        let path = solid_image(&tmp, "test.png", 255, 0, 0, 100, 100);
        assert!(open(&path).is_ok());
    }

    #[test]
    fn test_open_missing_file_errors() {
        let result = open(Path::new("/tmp/rwal_does_not_exist.png"));
        assert!(matches!(result, Err(RwalError::ImageDecodeError(_))));
    }

    // ── sample_pixels ────────────────────────────────────────────────────────

    #[test]
    fn test_sample_pixels_returns_nonempty() {
        let tmp  = TempDir::new();
        let path = solid_image(&tmp, "big.png", 100, 150, 200, 800, 600);
        let img  = open(&path).unwrap();
        let pixels = sample_pixels(&img);
        assert!(!pixels.is_empty());
    }

    #[test]
    fn test_sample_pixels_count_matches_resized_dimensions() {
        let tmp    = TempDir::new();
        let path   = solid_image(&tmp, "any.png", 10, 20, 30, 800, 600);
        let img    = open(&path).unwrap();
        let pixels = sample_pixels(&img);
        // After resize(200, 200, Nearest) with aspect ratio kept,
        // total pixels should be <= 200*200
        assert!(pixels.len() <= (SAMPLE_SIZE * SAMPLE_SIZE) as usize);
        assert!(!pixels.is_empty());
    }

    #[test]
    fn test_sample_pixels_solid_red_all_red() {
        let tmp    = TempDir::new();
        let path   = solid_image(&tmp, "red.png", 255, 0, 0, 400, 400);
        let img    = open(&path).unwrap();
        let pixels = sample_pixels(&img);
        assert!(pixels.iter().all(|p| p.r > 200 && p.g < 10 && p.b < 10));
    }

    #[test]
    fn test_sample_pixels_preserves_color_values() {
        let tmp    = TempDir::new();
        let path   = solid_image(&tmp, "color.png", 123, 45, 67, 300, 300);
        let img    = open(&path).unwrap();
        let pixels = sample_pixels(&img);
        // All pixels should be approximately (123, 45, 67)
        for p in &pixels {
            assert!((p.r as i16 - 123).abs() <= 2);
            assert!((p.g as i16 - 45).abs()  <= 2);
            assert!((p.b as i16 - 67).abs()  <= 2);
        }
    }

    #[test]
    fn test_sample_pixels_small_image_does_not_panic() {
        let tmp    = TempDir::new();
        let path   = solid_image(&tmp, "tiny.png", 0, 0, 0, 1, 1);
        let img    = open(&path).unwrap();
        let pixels = sample_pixels(&img);
        assert!(!pixels.is_empty());
    }

    // ── extract (full pipeline) ───────────────────────────────────────────────

    #[test]
    fn test_extract_returns_correct_count() {
        let tmp = TempDir::new();

        // gradient image with enough color variety for 16 clusters
        let img: ImageBuffer<ImageRgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(400, 400, |x, y| {
                ImageRgb([
                    (x * 255 / 400) as u8,
                    (y * 255 / 400) as u8,
                    ((x + y) * 255 / 800) as u8,
                ])
            });
        let path = tmp.path.join("gradient.png");
        img.save(&path).unwrap();

        let backend = crate::backends::kmeans::KMeans;
        let result  = extract(&path, &backend, 16, 10).unwrap();
        assert_eq!(result.len(), 16);
    }

    #[test]
    fn test_extract_missing_file_errors() {
        let backend = crate::backends::kmeans::KMeans;
        let result  = extract(Path::new("/tmp/rwal_no_file.png"), &backend, 16, 10);
        assert!(matches!(result, Err(RwalError::ImageDecodeError(_))));
    }
}