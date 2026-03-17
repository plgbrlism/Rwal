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
    strategy: &str,
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

    let accents = match strategy {
        "complementary" => generate_complementary(&sorted),
        "analogous" => generate_analogous(&sorted),
        "monochromatic" => generate_monochromatic(&sorted),
        "adaptive" => generate_adaptive(&sorted),
        "vibrant" => generate_vibrant(&sorted),
        "pastel" => generate_pastel(&sorted),
        "split_complementary" => generate_split(&sorted),
        "triadic" => generate_triadic(&sorted),
        _ => pick_accents(&sorted, 6), // "classic" pywal style
    };

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

    // Apply saturation shift if requested
    if let Some(amount) = saturate_amount {
        colors = adjust::saturate_all(&colors, amount);
    }

    // Apply light mode inversion if requested
    if light_mode {
        colors = adjust::invert_for_light(&colors);
    }

    // Enforce readability contrast (WCAG 4.5:1 minimum)
    // The background is colors[0], primary text is colors[15]
    colors[15] = adjust::ensure_contrast(&colors[0], &colors[15], 4.5);
    // Also ensure near-lightest (frequently used as alt foreground) is readable
    colors[7] = adjust::ensure_contrast(&colors[0], &colors[7], 4.5);

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

/// Smart strategy: Complementary
/// Picks a vibrant base, generates its complement, and maps them to the 6 accent slots.
fn generate_complementary(sorted: &[Rgb]) -> Vec<Rgb> {
    let base = find_vibrant_base(sorted);
    let base_hsl = base.to_hsl();
    
    // Complementary hue is +180 degrees
    let mut comp_hsl = base_hsl;
    use palette::ShiftHue;
    comp_hsl = comp_hsl.shift_hue(180.0);
    
    let base_rgb = Rgb::from_hsl(base_hsl);
    let comp_rgb = Rgb::from_hsl(comp_hsl);
    
    // Interleave base and complementary, slightly varying lightness
    vec![
        base_rgb,
        adjust::darken(&comp_rgb, 0.1),
        adjust::lighten(&base_rgb, 0.1),
        comp_rgb,
        adjust::darken(&base_rgb, 0.15),
        adjust::lighten(&comp_rgb, 0.15),
    ]
}

/// Smart strategy: Analogous
/// Picks a vibrant base and finds neighbors (-30, +30 degrees in hue).
fn generate_analogous(sorted: &[Rgb]) -> Vec<Rgb> {
    let base = find_vibrant_base(sorted);
    let base_hsl = base.to_hsl();
    
    use palette::ShiftHue;
    let anal1 = Rgb::from_hsl(base_hsl.shift_hue(-30.0));
    let anal2 = Rgb::from_hsl(base_hsl.shift_hue(30.0));
    let anal3 = Rgb::from_hsl(base_hsl.shift_hue(-60.0));
    
    vec![
        base,
        anal1,
        anal2,
        adjust::lighten(&base, 0.1),
        anal3,
        adjust::darken(&anal1, 0.1),
    ]
}

/// Smart strategy: Monochromatic
/// Creates an entire palette from varying lightnesses and slightly modifying saturations of one base color.
fn generate_monochromatic(sorted: &[Rgb]) -> Vec<Rgb> {
    let base = find_vibrant_base(sorted);
    
    vec![
        adjust::darken(&base, 0.3),
        adjust::darken(&base, 0.15),
        base,
        adjust::lighten(&base, 0.15),
        adjust::lighten(&base, 0.3),
        adjust::saturate(&base, -0.3),
    ]
}

/// Smart strategy: Adaptive
/// Scans the median luminance of all colors. 
/// The darker the image, the more it lightens and saturates the accents.
/// The lighter the image, the more it darkens the accents.
fn generate_adaptive(sorted: &[Rgb]) -> Vec<Rgb> {
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

/// Smart strategy: Vibrant (Upgraded monochrome)
/// Takes the 6 most distinct accent colors from the image,
/// and applies the adaptive luminance logic directly to them,
/// making the real wallpaper colors drastically more vibrant and readable.
fn generate_vibrant(sorted: &[Rgb]) -> Vec<Rgb> {
    let raw_accents = pick_accents(sorted, 6);
    
    let mut lums: Vec<f32> = sorted.iter().map(|c| adjust::relative_luminance(c)).collect();
    lums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_lum = if lums.is_empty() { 0.0 } else { lums[lums.len() / 2] };
    
    // Calculate shifts relative to a neutral 0.5 luminance
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

/// Smart strategy: Pastel
/// Takes the raw colors and flattens out their contrast, returning desaturated pastel tones.
/// Uses the adaptive logic so the pastels are always visible on the selected terminal bg.
fn generate_pastel(sorted: &[Rgb]) -> Vec<Rgb> {
    let raw_accents = pick_accents(sorted, 6);
    
    let mut lums: Vec<f32> = sorted.iter().map(|c| adjust::relative_luminance(c)).collect();
    lums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_lum = if lums.is_empty() { 0.0 } else { lums[lums.len() / 2] };
    
    let lightness_shift = (0.5 - median_lum) * 0.6;
    
    raw_accents.into_iter().map(|c| {
        // Flat, muted saturation
        let muted = adjust::saturate(&c, -0.3);
        // Adaptive lightness
        if lightness_shift > 0.0 {
            adjust::lighten(&muted, lightness_shift + 0.1) // slightly extra bright for pastel look
        } else {
            adjust::darken(&muted, -lightness_shift)
        }
    }).collect()
}

/// Smart strategy: Split Complementary
/// Finds a base and splits its complement (+150 and +210 deg), utilizing the adaptive visibility check.
fn generate_split(sorted: &[Rgb]) -> Vec<Rgb> {
    let base = find_vibrant_base(sorted);
    
    let mut lums: Vec<f32> = sorted.iter().map(|c| adjust::relative_luminance(c)).collect();
    lums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_lum = if lums.is_empty() { 0.0 } else { lums[lums.len() / 2] };
    
    let lightness_shift = (0.5 - median_lum) * 0.6;
    let saturation_boost = (0.5 - median_lum).max(0.0) * 0.5;

    let base_adj = if lightness_shift > 0.0 {
        adjust::saturate(&adjust::lighten(&base, lightness_shift), saturation_boost)
    } else {
        adjust::darken(&base, -lightness_shift)
    };
    
    let base_hsl = base_adj.to_hsl();
    use palette::ShiftHue;
    
    let split_1 = Rgb::from_hsl(base_hsl.shift_hue(150.0));
    let split_2 = Rgb::from_hsl(base_hsl.shift_hue(210.0));
    
    vec![
        base_adj,
        split_1,
        split_2,
        adjust::lighten(&base_adj, 0.15),
        adjust::lighten(&split_1, 0.15),
        adjust::lighten(&split_2, 0.15),
    ]
}

/// Smart strategy: Triadic
/// Finds a base and casts a triangle (+120 and +240 deg) across the wheel with adaptive scaling.
fn generate_triadic(sorted: &[Rgb]) -> Vec<Rgb> {
    let base = find_vibrant_base(sorted);
    
    let mut lums: Vec<f32> = sorted.iter().map(|c| adjust::relative_luminance(c)).collect();
    lums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_lum = if lums.is_empty() { 0.0 } else { lums[lums.len() / 2] };
    
    let lightness_shift = (0.5 - median_lum) * 0.6;
    let saturation_boost = (0.5 - median_lum).max(0.0) * 0.5;

    let base_adj = if lightness_shift > 0.0 {
        adjust::saturate(&adjust::lighten(&base, lightness_shift), saturation_boost)
    } else {
        adjust::darken(&base, -lightness_shift)
    };
    
    let base_hsl = base_adj.to_hsl();
    use palette::ShiftHue;
    
    let tri_1 = Rgb::from_hsl(base_hsl.shift_hue(120.0));
    let tri_2 = Rgb::from_hsl(base_hsl.shift_hue(240.0));
    
    vec![
        base_adj,
        tri_1,
        tri_2,
        adjust::darken(&base_adj, 0.2),
        adjust::saturate(&tri_1, 0.2),
        adjust::saturate(&tri_2, 0.2),
    ]
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
        let dict = build(raw, dummy_path(), 100, false, None, "classic").unwrap();
        assert_eq!(dict.colors.len(), 16);
    }

    #[test]
    fn test_build_empty_raw_errors() {
        assert!(matches!(
            build(vec![], dummy_path(), 100, false, None, "classic"),
            Err(RwalError::NoColorsExtracted)
        ));
    }

    #[test]
    fn test_build_wallpaper_and_alpha_preserved() {
        let raw = flat_palette(16);
        let path = PathBuf::from("/home/user/wall.png");
        let dict = build(raw, path.clone(), 80, false, None, "classic").unwrap();
        assert_eq!(dict.wallpaper, path);
        assert_eq!(dict.alpha, 80);
    }

    // ── slot assignments ─────────────────────────────────────────────────────

    #[test]
    fn test_color0_is_darkest() {
        let raw = flat_palette(16);
        let dict = build(raw, dummy_path(), 100, false, None, "classic").unwrap();
        // color0 should be the darkest in dark mode
        assert!(dict.colors[0].luminance() <= dict.colors[7].luminance());
        assert!(dict.colors[0].luminance() <= dict.colors[15].luminance());
    }

    #[test]
    fn test_color15_is_brightest() {
        let raw = flat_palette(16);
        let dict = build(raw, dummy_path(), 100, false, None, "classic").unwrap();
        assert!(dict.colors[15].luminance() >= dict.colors[0].luminance());
        assert!(dict.colors[15].luminance() >= dict.colors[7].luminance());
    }

    #[test]
    fn test_color8_is_darker_than_color0() {
        let raw = flat_palette(16);
        let dict = build(raw, dummy_path(), 100, false, None, "classic").unwrap();
        assert!(dict.colors[8].luminance() <= dict.colors[0].luminance() + 1.0);
    }

    #[test]
    fn test_bright_accents_lighter_than_base_accents() {
        let raw = flat_palette(16);
        let dict = build(raw, dummy_path(), 100, false, None, "classic").unwrap();
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
        let dict = build(raw, dummy_path(), 100, false, None, "classic").unwrap();
        assert_eq!(dict.special.background, dict.colors[0]);
    }

    #[test]
    fn test_special_foreground_equals_color15() {
        let raw = flat_palette(16);
        let dict = build(raw, dummy_path(), 100, false, None, "classic").unwrap();
        assert_eq!(dict.special.foreground, dict.colors[15]);
    }

    #[test]
    fn test_special_cursor_equals_color15() {
        let raw = flat_palette(16);
        let dict = build(raw, dummy_path(), 100, false, None, "classic").unwrap();
        assert_eq!(dict.special.cursor, dict.colors[15]);
    }

    // ── light mode ───────────────────────────────────────────────────────────

    #[test]
    fn test_light_mode_inverts_background_and_foreground() {
        let raw = flat_palette(16);
        let dark  = build(raw.clone(), dummy_path(), 100, false, None, "classic").unwrap();
        let light = build(raw,         dummy_path(), 100, true,  None, "classic").unwrap();
        // In light mode color0 (background) should be brighter than dark mode
        assert!(light.colors[0].luminance() > dark.colors[0].luminance());
    }

    #[test]
    fn test_light_mode_color0_equals_dark_color15() {
        let raw = flat_palette(16);
        let dark  = build(raw.clone(), dummy_path(), 100, false, None, "classic").unwrap();
        let light = build(raw,         dummy_path(), 100, true,  None, "classic").unwrap();
        assert_eq!(light.colors[0], dark.colors[15]);
    }

    // ── small input ──────────────────────────────────────────────────────────

    #[test]
    fn test_build_with_single_color_does_not_panic() {
        let raw = vec![Rgb::new(100, 150, 200)];
        let result = build(raw, dummy_path(), 100, false, None, "classic");
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_with_two_colors_does_not_panic() {
        let raw = vec![Rgb::new(20, 20, 20), Rgb::new(200, 200, 200)];
        let result = build(raw, dummy_path(), 100, false, None, "classic");
        assert!(result.is_ok());
    }

    // ── saturation ───────────────────────────────────────────────────────────

    #[test]
    fn test_saturate_amount_applied() {
        let raw = flat_palette(16);
        let normal   = build(raw.clone(), dummy_path(), 100, false, None, "classic").unwrap();
        let saturated = build(raw,        dummy_path(), 100, false, Some(0.5), "classic").unwrap();
        // at least some color should differ
        let any_diff = normal.colors.iter().zip(saturated.colors.iter())
            .any(|(a, b)| a != b);
        assert!(any_diff, "saturate should change at least one color");
    }
}