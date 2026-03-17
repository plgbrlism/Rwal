/*
Colorthief median cut implementation.
Will function as a fallback when the image is too small for the MMCQ implementation.
Alternative algorithm, tends toward less saturated palettes 
(like pywal's colorthief backend). Good for lighter images.

*/
use crate::colors::types::Rgb;
use crate::error::RwalError;
use super::Backend;

pub struct MedianCut;

impl Backend for MedianCut {
    fn name(&self) -> &str {
        "median_cut"
    }

    /// `iterations` is unused in median cut (it's deterministic) but kept
    /// in the trait signature for API consistency.
    fn generate(&self, pixels: &[Rgb], count: usize, _iterations: u8) -> Result<Vec<Rgb>, RwalError> {
        if pixels.is_empty() {
            return Err(RwalError::NoColorsExtracted);
        }

        // Number of splits needed: 2^splits >= count
        let splits = (count as f32).log2().ceil() as usize;
        
        let mut pixels_mut = pixels.to_vec();
        let mut result = Vec::with_capacity(count);
        
        median_cut(&mut pixels_mut, splits, &mut result);
        
        result.truncate(count);

        if result.is_empty() {
            return Err(RwalError::NoColorsExtracted);
        }

        Ok(result)
    }
}

/// Recursively split buckets along the widest color channel.
fn median_cut(pixels: &mut [Rgb], depth: usize, out: &mut Vec<Rgb>) {
    if depth == 0 || pixels.len() <= 1 {
        if !pixels.is_empty() {
            out.push(average(pixels));
        }
        return;
    }

    let (r_range, g_range, b_range) = channel_ranges(pixels);

    // Split along the channel with the widest range
    if r_range >= g_range && r_range >= b_range {
        pixels.sort_unstable_by_key(|p| p.r);
    } else if g_range >= r_range && g_range >= b_range {
        pixels.sort_unstable_by_key(|p| p.g);
    } else {
        pixels.sort_unstable_by_key(|p| p.b);
    }

    let mid = pixels.len() / 2;
    let (lo, hi) = pixels.split_at_mut(mid);

    median_cut(lo, depth - 1, out);
    median_cut(hi, depth - 1, out);
}

/// Returns (r_range, g_range, b_range) for a slice of pixels.
fn channel_ranges(pixels: &[Rgb]) -> (u8, u8, u8) {
    let (mut r_min, mut g_min, mut b_min) = (255u8, 255u8, 255u8);
    let (mut r_max, mut g_max, mut b_max) = (0u8, 0u8, 0u8);

    for p in pixels {
        r_min = r_min.min(p.r); r_max = r_max.max(p.r);
        g_min = g_min.min(p.g); g_max = g_max.max(p.g);
        b_min = b_min.min(p.b); b_max = b_max.max(p.b);
    }

    (r_max - r_min, g_max - g_min, b_max - b_min)
}

/// Average all pixels in a bucket into a single representative color.
fn average(pixels: &[Rgb]) -> Rgb {
    let n = pixels.len() as u64;
    let r = pixels.iter().map(|p| p.r as u64).sum::<u64>() / n;
    let g = pixels.iter().map(|p| p.g as u64).sum::<u64>() / n;
    let b = pixels.iter().map(|p| p.b as u64).sum::<u64>() / n;
    Rgb::new(r as u8, g as u8, b as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── channel_ranges ───────────────────────────────────────────────────────

    #[test]
    fn test_channel_ranges_uniform() {
        let pixels = vec![Rgb::new(100, 100, 100); 5];
        assert_eq!(channel_ranges(&pixels), (0, 0, 0));
    }

    #[test]
    fn test_channel_ranges_detects_widest() {
        let pixels = vec![
            Rgb::new(0, 0, 0),
            Rgb::new(0, 0, 255), // blue has widest range
        ];
        let (r, g, b) = channel_ranges(&pixels);
        assert!(b > r && b > g);
    }

    // ── average ──────────────────────────────────────────────────────────────

    #[test]
    fn test_average_single_pixel() {
        let pixels = vec![Rgb::new(100, 150, 200)];
        assert_eq!(average(&pixels), Rgb::new(100, 150, 200));
    }

    #[test]
    fn test_average_two_pixels() {
        let pixels = vec![Rgb::new(0, 0, 0), Rgb::new(200, 200, 200)];
        assert_eq!(average(&pixels), Rgb::new(100, 100, 100));
    }

    // ── full generate ────────────────────────────────────────────────────────

    #[test]
    fn test_generate_empty_errors() {
        assert!(matches!(
            MedianCut.generate(&[], 8, 0),
            Err(RwalError::NoColorsExtracted)
        ));
    }

    #[test]
    fn test_generate_solid_color() {
        let pixels = vec![Rgb::new(200, 100, 50); 200];
        let result = MedianCut.generate(&pixels, 4, 0).unwrap();
        assert!(result.iter().all(|c| c.r > 190 && c.g > 90 && c.b > 40));
    }

    #[test]
    fn test_generate_two_distinct_colors() {
        let mut pixels = vec![Rgb::new(20, 20, 20); 200];
        pixels.extend(vec![Rgb::new(220, 220, 220); 200]);

        let result = MedianCut.generate(&pixels, 2, 0).unwrap();

        let has_dark   = result.iter().any(|c| c.r < 100);
        let has_bright = result.iter().any(|c| c.r > 150);
        assert!(has_dark,   "expected a dark color");
        assert!(has_bright, "expected a bright color");
    }

    #[test]
    fn test_generate_count_does_not_exceed_request() {
        let pixels: Vec<Rgb> = (0..200).map(|i| Rgb::new(i as u8, i as u8, i as u8)).collect();
        let result = MedianCut.generate(&pixels, 8, 0).unwrap();
        assert!(result.len() <= 8);
    }

    // ── backend trait ────────────────────────────────────────────────────────

    #[test]
    fn test_name_is_median_cut() {
        assert_eq!(MedianCut.name(), "median_cut");
    }
}