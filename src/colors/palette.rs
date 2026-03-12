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
    saturate_amount: Option<f32>,
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

    // Ensure we have enough colors — pad by repeating if needed
    let sorted = pad_to(&sorted, 8);

    // Assign the 8 base slots
    let color0 = sorted[0];                          // darkest
    let color7 = sorted[sorted.len() - 2];           // near-lightest
    let color15 = sorted[sorted.len() - 1];          // lightest

    // Pick 6 accent colors from the middle range
    let accents = pick_accents(&sorted, 6);

    // Build color8: color0 darkened by 20%
    let color8 = adjust::darken(&color0, 0.20);

    // Build color9–14: accents lightened by 20%
    let bright_accents: Vec<Rgb> = accents.iter().map(|c| adjust::lighten(c, 0.20)).collect();

    // Assemble the 16-slot array
    let mut colors: [Rgb; 16] = [Rgb::new(0, 0, 0); 16];
    colors[0]  = color0;
    colors[1]  = accents[0];
    colors[2]  = accents[1];
    colors[3]  = accents[2];
    colors[4]  = accents[3];
    colors[5]  = accents[4];
    colors[6]  = accents[5];
    colors[7]  = color7;
    colors[8]  = color8;
    colors[9]  = bright_accents[0];
    colors[10] = bright_accents[1];
    colors[11] = bright_accents[2];
    colors[12] = bright_accents[3];
    colors[13] = bright_accents[4];
    colors[14] = bright_accents[5];
    colors[15] = color15;

    // Apply saturation shift if requested
    if let Some(amount) = saturate_amount {
        colors = adjust::saturate_all(&colors, amount);
    }

    // Apply light mode inversion if requested
    if light_mode {
        colors = adjust::invert_for_light(&colors);
    }

    let special = Special {
        background: colors[0],
        foreground: colors[15],
        cursor:     colors[15],
    };

    Ok(ColorDict { wallpaper, alpha, special, colors })
}

/// Pick `n` accent colors spread across the middle range of sorted colors.
/// Skips the first (darkest) and last (lightest) entries.
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
        let dict = build(raw, dummy_path(), 100, false, None).unwrap();
        assert_eq!(dict.colors.len(), 16);
    }

    #[test]
    fn test_build_empty_raw_errors() {
        assert!(matches!(
            build(vec![], dummy_path(), 100, false, None),
            Err(RwalError::NoColorsExtracted)
        ));
    }

    #[test]
    fn test_build_wallpaper_and_alpha_preserved() {
        let raw = flat_palette(16);
        let path = PathBuf::from("/home/user/wall.png");
        let dict = build(raw, path.clone(), 80, false, None).unwrap();
        assert_eq!(dict.wallpaper, path);
        assert_eq!(dict.alpha, 80);
    }

    // ── slot assignments ─────────────────────────────────────────────────────

    #[test]
    fn test_color0_is_darkest() {
        let raw = flat_palette(16);
        let dict = build(raw, dummy_path(), 100, false, None).unwrap();
        // color0 should be the darkest in dark mode
        assert!(dict.colors[0].luminance() <= dict.colors[7].luminance());
        assert!(dict.colors[0].luminance() <= dict.colors[15].luminance());
    }

    #[test]
    fn test_color15_is_brightest() {
        let raw = flat_palette(16);
        let dict = build(raw, dummy_path(), 100, false, None).unwrap();
        assert!(dict.colors[15].luminance() >= dict.colors[0].luminance());
        assert!(dict.colors[15].luminance() >= dict.colors[7].luminance());
    }

    #[test]
    fn test_color8_is_darker_than_color0() {
        let raw = flat_palette(16);
        let dict = build(raw, dummy_path(), 100, false, None).unwrap();
        assert!(dict.colors[8].luminance() <= dict.colors[0].luminance() + 1.0);
    }

    #[test]
    fn test_bright_accents_lighter_than_base_accents() {
        let raw = flat_palette(16);
        let dict = build(raw, dummy_path(), 100, false, None).unwrap();
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
        let dict = build(raw, dummy_path(), 100, false, None).unwrap();
        assert_eq!(dict.special.background, dict.colors[0]);
    }

    #[test]
    fn test_special_foreground_equals_color15() {
        let raw = flat_palette(16);
        let dict = build(raw, dummy_path(), 100, false, None).unwrap();
        assert_eq!(dict.special.foreground, dict.colors[15]);
    }

    #[test]
    fn test_special_cursor_equals_color15() {
        let raw = flat_palette(16);
        let dict = build(raw, dummy_path(), 100, false, None).unwrap();
        assert_eq!(dict.special.cursor, dict.colors[15]);
    }

    // ── light mode ───────────────────────────────────────────────────────────

    #[test]
    fn test_light_mode_inverts_background_and_foreground() {
        let raw = flat_palette(16);
        let dark  = build(raw.clone(), dummy_path(), 100, false, None).unwrap();
        let light = build(raw,         dummy_path(), 100, true,  None).unwrap();
        // In light mode color0 (background) should be brighter than dark mode
        assert!(light.colors[0].luminance() > dark.colors[0].luminance());
    }

    #[test]
    fn test_light_mode_color0_equals_dark_color15() {
        let raw = flat_palette(16);
        let dark  = build(raw.clone(), dummy_path(), 100, false, None).unwrap();
        let light = build(raw,         dummy_path(), 100, true,  None).unwrap();
        assert_eq!(light.colors[0], dark.colors[15]);
    }

    // ── small input ──────────────────────────────────────────────────────────

    #[test]
    fn test_build_with_single_color_does_not_panic() {
        let raw = vec![Rgb::new(100, 150, 200)];
        let result = build(raw, dummy_path(), 100, false, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_with_two_colors_does_not_panic() {
        let raw = vec![Rgb::new(20, 20, 20), Rgb::new(200, 200, 200)];
        let result = build(raw, dummy_path(), 100, false, None);
        assert!(result.is_ok());
    }

    // ── saturation ───────────────────────────────────────────────────────────

    #[test]
    fn test_saturate_amount_applied() {
        let raw = flat_palette(16);
        let normal   = build(raw.clone(), dummy_path(), 100, false, None).unwrap();
        let saturated = build(raw,        dummy_path(), 100, false, Some(0.5)).unwrap();
        // at least some color should differ
        let any_diff = normal.colors.iter().zip(saturated.colors.iter())
            .any(|(a, b)| a != b);
        assert!(any_diff, "saturate should change at least one color");
    }
}