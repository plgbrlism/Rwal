/*
This is the heart — pywal's adjust() logic:

Sort input colors by YIQ luminance (0.299R + 0.587G + 0.114B)
Assign slots:

color0 = darkest (background)
color1–6 = the 6 mid-range accent colors
color7 = near-lightest (foreground)
color8 = color0 darkened by ~40% (bright black)
color9–14 = color1–6 each lightened ~20%
color15 = lightest (bright white)

special.background = color0, special.foreground = color15, special.cursor = color15

*/
use crate::colors::types::{ColorDict, Rgb, Special};
use crate::colors::adjust;
use crate::error::RwalError;
use std::path::PathBuf;

/// Build a full `ColorDict` from raw backend colors.
///
/// Slot assignments (mirrors pywal's adjust() logic):
///   color0       = darkest (background)
///   color1–6     = 6 accent colors
///   color7       = near-lightest (foreground)
///   color8       = color0 darkened 20% (bright black)
///   color9–14    = color1–6 lightened 20% (bright accents)
///   color15      = lightest (bright white)
///
/// special.background = color0
/// special.foreground = color15
/// special.cursor     = color15
pub fn build(
    raw: Vec<Rgb>,
    wallpaper: PathBuf,
    alpha: u8,
    light_mode: bool,
    mode_name: &str,
) -> Result<ColorDict, RwalError> {
    if raw.is_empty() {
        return Err(RwalError::NoColorsExtracted);
    }

    // Sort by YIQ luminance dark → light
    let mut sorted = raw;
    sorted.sort_by(|a, b| {
        a.luminance()
            .partial_cmp(&b.luminance())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let sorted = pad_to(&sorted, 8);

    // Base slots common to all strategies
    let color0 = sorted[0];                          // darkest
    let color7 = sorted[sorted.len() - 2];           // near-lightest
    let color15 = sorted[sorted.len() - 1];          // lightest
    let color8 = adjust::darken(&color0, 0.20);      // bright black

    let mode: Box<dyn ColorMode> = match mode_name {
        "dynamic" => Box::new(AdaptiveMode),
        "neon"    => Box::new(VibrantMode),
        "soft"    => Box::new(PastelMode),
        _         => Box::new(ClassicMode), // "balanced"
    };


    let accents = mode.generate(&sorted);

    let bright_accents: Vec<Rgb> = accents.iter().map(|c| adjust::lighten(c, 0.20)).collect();

    let mut colors: [Rgb; 16] = [Rgb::new(0, 0, 0); 16];
    colors[0]  = color0;
    for i in 0..6 {
        colors[i + 1] = accents[i];
        colors[i + 9] = bright_accents[i];
    }
    colors[7]  = color7;
    colors[8]  = color8;
    colors[15] = color15;

    // Apply light mode inversion if requested
    if light_mode {
        colors = adjust::invert_for_light(&colors);
    }

    // Enforce readability contrast (WCAG 4.5:1 minimum)
    // By default, we guarantee the primary and secondary foreground are readable.
    colors[15] = adjust::ensure_contrast(&colors[0], &colors[15], 4.5);
    colors[7] = adjust::ensure_contrast(&colors[0], &colors[7], 4.5);

    // Enforce contrast for all 16 colors against the background (color0)
    // to guarantee consistent accessibility across all themes and wallpapers.
    for i in 1..=6 {
        colors[i] = adjust::ensure_contrast(&colors[0], &colors[i], 4.5);
        colors[i + 8] = adjust::ensure_contrast(&colors[0], &colors[i + 8], 4.5);
    }

    let special = Special {
        background: colors[0],
        foreground: colors[15],
        cursor:     colors[15],
    };

    Ok(ColorDict { wallpaper, alpha, special, colors })
}

/// Find the most vibrant color from the extracted set to use as a primary accent.
fn find_vibrant_base(sorted: &[Rgb]) -> Rgb {
    let mut best = sorted[sorted.len() / 2];
    let mut max_sat = 0.0;
    
    // Skip the very darkest and lightest
    let candidates = sorted.iter().skip(1).take(sorted.len().saturating_sub(2));
    
    for c in candidates {
        let hsl = c.to_hsl();
        if hsl.saturation > max_sat {
            max_sat = hsl.saturation;
            best = *c;
        }
    }
    best
}
/// Unified Trait for Palette Generation Modes
pub trait ColorMode {
    fn generate(&self, sorted: &[Rgb]) -> Vec<Rgb>;
}

/// Smart strategy: Adaptive (Default)
/// Scans the median luminance of all colors. 
/// The darker the image, the more it lightens and saturates the accents.
/// The lighter the image, the more it darkens the accents.
pub struct AdaptiveMode;
impl ColorMode for AdaptiveMode {
    fn generate(&self, sorted: &[Rgb]) -> Vec<Rgb> {
        let base = find_vibrant_base(sorted);
        
        let mut lums: Vec<f32> = sorted.iter().map(|c| adjust::relative_luminance(c)).collect();
        lums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median_lum = if lums.is_empty() { 0.0 } else { lums[lums.len() / 2] };
        
        // Calculate shifts relative to a neutral 0.5 luminance
        let lightness_shift = (0.5 - median_lum) * 0.6;
        let saturation_boost = (0.5 - median_lum).max(0.0) * 0.5;

        let base_adj = if lightness_shift > 0.0 {
            adjust::saturate(&adjust::lighten(&base, lightness_shift), saturation_boost)
        } else {
            adjust::darken(&base, -lightness_shift) // If light, just darken it for contrast
        };
        
        let base_hsl = base_adj.to_hsl();
        use palette::ShiftHue;
        
        // Create an analogous and complementary mix for a visually appealing harmonic spread
        vec![
            base_adj,
            Rgb::from_hsl(base_hsl.shift_hue(30.0)),
            Rgb::from_hsl(base_hsl.shift_hue(-30.0)),
            Rgb::from_hsl(base_hsl.shift_hue(150.0)),
            Rgb::from_hsl(base_hsl.shift_hue(180.0)),
            Rgb::from_hsl(base_hsl.shift_hue(210.0)),
        ]
    }
}

/// Smart strategy: Vibrant
/// Takes the 6 most distinct accent colors from the image,
/// and applies the adaptive luminance logic directly to them.
pub struct VibrantMode;
impl ColorMode for VibrantMode {
    fn generate(&self, sorted: &[Rgb]) -> Vec<Rgb> {
        let raw_accents = pick_accents(sorted, 6);
        
        let mut lums: Vec<f32> = sorted.iter().map(|c| adjust::relative_luminance(c)).collect();
        lums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median_lum = if lums.is_empty() { 0.0 } else { lums[lums.len() / 2] };
        
        let lightness_shift = (0.5 - median_lum) * 0.6;
        let saturation_boost = (0.5 - median_lum).max(0.0) * 0.5;

        raw_accents.into_iter().map(|c| {
            if lightness_shift > 0.0 {
                adjust::saturate(&adjust::lighten(&c, lightness_shift), saturation_boost)
            } else {
                adjust::darken(&c, -lightness_shift)
            }
        }).collect()
    }
}

/// Smart strategy: Pastel
/// Takes the raw colors and flattens out their contrast, returning desaturated pastel tones.
pub struct PastelMode;
impl ColorMode for PastelMode {
    fn generate(&self, sorted: &[Rgb]) -> Vec<Rgb> {
        let raw_accents = pick_accents(sorted, 6);
        
        let mut lums: Vec<f32> = sorted.iter().map(|c| adjust::relative_luminance(c)).collect();
        lums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median_lum = if lums.is_empty() { 0.0 } else { lums[lums.len() / 2] };
        
        let lightness_shift = (0.5 - median_lum) * 0.6;
        
        raw_accents.into_iter().map(|c| {
            let muted = adjust::saturate(&c, -0.3);
            if lightness_shift > 0.0 {
                adjust::lighten(&muted, lightness_shift + 0.1)
            } else {
                adjust::darken(&muted, -lightness_shift)
            }
        }).collect()
    }
}

/// Smart strategy: Classic
/// Matches standard Pywal behavior.
pub struct ClassicMode;
impl ColorMode for ClassicMode {
    fn generate(&self, sorted: &[Rgb]) -> Vec<Rgb> {
        pick_accents(sorted, 6)
    }
}

/// Pick `n` accent colors spread across the middle range of sorted colors (pywal classic).
fn pick_accents(sorted: &[Rgb], n: usize) -> Vec<Rgb> {
    let inner: Vec<Rgb> = sorted
        .iter()
        .skip(1)
        .take(sorted.len().saturating_sub(2))
        .cloned()
        .collect();

    if inner.is_empty() {
        // Fallback: derive accents from color0 by lightening incrementally
        return (0..n)
            .map(|i| adjust::lighten(&sorted[0], 0.1 * (i + 1) as f32))
            .collect();
    }

    // Spread evenly across inner range
    (0..n)
        .map(|i| {
            let idx = (i * inner.len()) / n;
            inner[idx.min(inner.len() - 1)]
        })
        .collect()
}

/// Pad a color slice to at least `min_len` by repeating the last color.
fn pad_to(colors: &[Rgb], min_len: usize) -> Vec<Rgb> {
    let mut out = colors.to_vec();
    while out.len() < min_len {
        out.push(*out.last().unwrap());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_path() -> PathBuf {
        PathBuf::from("/tmp/wall.jpg")
    }

    fn flat_palette(n: usize) -> Vec<Rgb> {
        // spread of grays from dark to light
        (0..n)
            .map(|i| {
                let v = (i * 255 / (n - 1).max(1)) as u8;
                Rgb::new(v, v, v)
            })
            .collect()
    }

    // ── build: basic structure ───────────────────────────────────────────────

    #[test]
    fn test_build_returns_16_colors() {
        let raw = flat_palette(16);
        let dict = build(raw, dummy_path(), 100, false, "classic").unwrap();
        assert_eq!(dict.colors.len(), 16);
    }

    #[test]
    fn test_build_empty_raw_errors() {
        assert!(matches!(
            build(vec![], dummy_path(), 100, false, "classic"),
            Err(RwalError::NoColorsExtracted)
        ));
    }

    #[test]
    fn test_build_wallpaper_and_alpha_preserved() {
        let raw = flat_palette(16);
        let path = PathBuf::from("/home/user/wall.png");
        let dict = build(raw, path.clone(), 80, false, "classic").unwrap();
        assert_eq!(dict.wallpaper, path);
        assert_eq!(dict.alpha, 80);
    }

    // ── slot assignments ─────────────────────────────────────────────────────

    #[test]
    fn test_color0_is_darkest() {
        let raw = flat_palette(16);
        let dict = build(raw, dummy_path(), 100, false, "classic").unwrap();
        // color0 should be the darkest in dark mode
        assert!(dict.colors[0].luminance() <= dict.colors[7].luminance());
        assert!(dict.colors[0].luminance() <= dict.colors[15].luminance());
    }

    #[test]
    fn test_color15_is_brightest() {
        let raw = flat_palette(16);
        let dict = build(raw, dummy_path(), 100, false, "classic").unwrap();
        assert!(dict.colors[15].luminance() >= dict.colors[0].luminance());
        assert!(dict.colors[15].luminance() >= dict.colors[7].luminance());
    }

    #[test]
    fn test_color8_is_darker_than_color0() {
        let raw = flat_palette(16);
        let dict = build(raw, dummy_path(), 100, false, "classic").unwrap();
        assert!(dict.colors[8].luminance() <= dict.colors[0].luminance() + 1.0);
    }

    #[test]
    fn test_bright_accents_lighter_than_base_accents() {
        let raw = flat_palette(16);
        let dict = build(raw, dummy_path(), 100, false, "classic").unwrap();
        for i in 1..=6 {
            assert!(
                dict.colors[i + 8].luminance() >= dict.colors[i].luminance() - 1.0,
                "color{} should be >= color{} luminance", i + 8, i
            );
        }
    }

    // ── special slots ────────────────────────────────────────────────────────

    #[test]
    fn test_special_background_equals_color0() {
        let raw = flat_palette(16);
        let dict = build(raw, dummy_path(), 100, false, "classic").unwrap();
        assert_eq!(dict.special.background, dict.colors[0]);
    }

    #[test]
    fn test_special_foreground_equals_color15() {
        let raw = flat_palette(16);
        let dict = build(raw, dummy_path(), 100, false, "classic").unwrap();
        assert_eq!(dict.special.foreground, dict.colors[15]);
    }

    #[test]
    fn test_special_cursor_equals_color15() {
        let raw = flat_palette(16);
        let dict = build(raw, dummy_path(), 100, false, "classic").unwrap();
        assert_eq!(dict.special.cursor, dict.colors[15]);
    }

    // ── light mode ───────────────────────────────────────────────────────────

    #[test]
    fn test_light_mode_inverts_background_and_foreground() {
        let raw = flat_palette(16);
        let dark  = build(raw.clone(), dummy_path(), 100, false, "classic").unwrap();
        let light = build(raw,         dummy_path(), 100, true,  "classic").unwrap();
        // In light mode color0 (background) should be brighter than dark mode
        assert!(light.colors[0].luminance() > dark.colors[0].luminance());
    }

    #[test]
    fn test_light_mode_color0_equals_dark_color15() {
        let raw = flat_palette(16);
        let dark  = build(raw.clone(), dummy_path(), 100, false, "classic").unwrap();
        let light = build(raw,         dummy_path(), 100, true,  "classic").unwrap();
        assert_eq!(light.colors[0], dark.colors[15]);
    }

    // ── small input ──────────────────────────────────────────────────────────

    #[test]
    fn test_build_with_single_color_does_not_panic() {
        let raw = vec![Rgb::new(100, 150, 200)];
        let result = build(raw, dummy_path(), 100, false, "classic");
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_with_two_colors_does_not_panic() {
        let raw = vec![Rgb::new(20, 20, 20), Rgb::new(200, 200, 200)];
        let result = build(raw, dummy_path(), 100, false, "classic");
        assert!(result.is_ok());
    }

    // ── accessible ───────────────────────────────────────────────────────────

    #[test]
    fn test_accessibility_is_enforced_by_default() {
        let raw = flat_palette(16);
        // build() no longer takes the accessible flag, it's always on.
        let dict = build(raw, dummy_path(), 100, false, "classic").unwrap();
        
        // Everything except color0 and color8 should hit the 4.5 contrast wall
        for i in 1..=15 {
            if i == 8 { continue; }
            let cr = adjust::contrast_ratio(&dict.colors[0], &dict.colors[i]);
            assert!(cr >= 4.4, "Color{} failed contrast check. Ratio: {}", i, cr);
        }
    }
}