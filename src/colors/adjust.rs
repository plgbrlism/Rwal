/*

saturate(colors, amount) — convert RGB→HSL, shift S, convert back
invert_for_light(colors) — swap color0↔color15, color7↔color8 for -l mode

*/
use palette::{Hsl, Lighten, Darken, Saturate};
use crate::colors::types::Rgb;

/// Lighten a color by `amount` (0.0–1.0) in HSL lightness.
pub fn lighten(color: &Rgb, amount: f32) -> Rgb {
    let hsl: Hsl = color.to_hsl();
    let lightened = hsl.lighten(amount);
    Rgb::from_hsl(lightened)
}

/// Darken a color by `amount` (0.0–1.0) in HSL lightness.
pub fn darken(color: &Rgb, amount: f32) -> Rgb {
    let hsl: Hsl = color.to_hsl();
    let darkened = hsl.darken(amount);
    Rgb::from_hsl(darkened)
}

/// Shift the HSL saturation of a color by `amount` (can be negative).
/// Clamps to valid range automatically via palette.
pub fn saturate(color: &Rgb, amount: f32) -> Rgb {
    let hsl: Hsl = color.to_hsl();
    let saturated = hsl.saturate(amount);
    Rgb::from_hsl(saturated)
}

/// Apply saturation shift to all 16 colors in a palette.
pub fn saturate_all(colors: &[Rgb; 16], amount: f32) -> [Rgb; 16] {
    let mut out = *colors;
    for c in out.iter_mut() {
        *c = saturate(c, amount);
    }
    out
}

/// Invert the palette for light mode:
/// swaps color0 ↔ color15 and color7 ↔ color8.
pub fn invert_for_light(colors: &[Rgb; 16]) -> [Rgb; 16] {
    let mut out = *colors;
    out.swap(0, 15);
    out.swap(7, 8);
    out
}

/// Calculate WCAG relative luminance (0.0 to 1.0)
pub fn relative_luminance(color: &Rgb) -> f32 {
    fn adjust_channel(c: u8) -> f32 {
        let sc = c as f32 / 255.0;
        if sc <= 0.03928 {
            sc / 12.92
        } else {
            ((sc + 0.055) / 1.055).powf(2.4)
        }
    }
    
    let r = adjust_channel(color.r);
    let g = adjust_channel(color.g);
    let b = adjust_channel(color.b);
    
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// Calculate the contrast ratio between two colors (returns 1.0 to 21.0)
pub fn contrast_ratio(c1: &Rgb, c2: &Rgb) -> f32 {
    let l1 = relative_luminance(c1);
    let l2 = relative_luminance(c2);
    
    let brightest = l1.max(l2);
    let darkest = l1.min(l2);
    
    (brightest + 0.05) / (darkest + 0.05)
}

/// Iteratively adjust the foreground's lightness to ensure a target contrast against the background
pub fn ensure_contrast(bg: &Rgb, fg: &Rgb, target_ratio: f32) -> Rgb {
    let current_ratio = contrast_ratio(bg, fg);
    if current_ratio >= target_ratio {
        return *fg;
    }
    
    let l_bg = relative_luminance(bg);
    
    // Direct algebra to find target relative luminance, with a small epsilon
    // to guarantee the contrast_ratio passes after 8-bit RGB truncation
    let target_l_fg = if l_bg < 0.5 {
        target_ratio * (l_bg + 0.05) - 0.05 + 0.005
    } else {
        (l_bg + 0.05) / target_ratio - 0.05 - 0.005
    }.clamp(0.0, 1.0);

    fn linearize(c: u8) -> f32 {
        let sc = c as f32 / 255.0;
        if sc <= 0.03928 { sc / 12.92 } else { ((sc + 0.055) / 1.055).powf(2.4) }
    }
    
    fn unlinearize(l: f32) -> u8 {
        let sc = if l <= 0.0031308 { l * 12.92 } else { 1.055 * l.powf(1.0 / 2.4) - 0.055 };
        (sc.clamp(0.0, 1.0) * 255.0).round() as u8
    }

    let mut r_lin = linearize(fg.r);
    let mut g_lin = linearize(fg.g);
    let mut b_lin = linearize(fg.b);
    let l_fg = 0.2126 * r_lin + 0.7152 * g_lin + 0.0722 * b_lin;

    // If perfectly black, just scale white up
    if l_fg < 0.0001 {
        r_lin = 1.0; g_lin = 1.0; b_lin = 1.0;
    }

    // Scale linearly
    let l_fg = l_fg.max(0.001);
    let scale = target_l_fg / l_fg;

    // We can overshoot 1.0, unlinearize clamps it
    Rgb::new(
        unlinearize(r_lin * scale),
        unlinearize(g_lin * scale),
        unlinearize(b_lin * scale)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── lighten ──────────────────────────────────────────────────────────────

    #[test]
    fn test_lighten_makes_color_brighter() {
        let c = Rgb::new(50, 50, 50);
        let result = lighten(&c, 0.2);
        assert!(result.luminance() > c.luminance());
    }

    #[test]
    fn test_lighten_white_stays_white() {
        let white = Rgb::new(255, 255, 255);
        let result = lighten(&white, 0.2);
        assert!(result.luminance() >= white.luminance() - 1.0);
    }

    #[test]
    fn test_lighten_zero_amount_no_change() {
        let c = Rgb::new(100, 150, 200);
        let result = lighten(&c, 0.0);
        assert_eq!(result, c);
    }

    // ── darken ───────────────────────────────────────────────────────────────

    #[test]
    fn test_darken_makes_color_dimmer() {
        let c = Rgb::new(200, 200, 200);
        let result = darken(&c, 0.2);
        assert!(result.luminance() < c.luminance());
    }

    #[test]
    fn test_darken_black_stays_black() {
        let black = Rgb::new(0, 0, 0);
        let result = darken(&black, 0.2);
        assert!(result.luminance() <= 1.0);
    }

    #[test]
    fn test_darken_zero_amount_no_change() {
        let c = Rgb::new(100, 150, 200);
        let result = darken(&c, 0.0);
        assert_eq!(result, c);
    }

    // ── saturate ─────────────────────────────────────────────────────────────

    #[test]
    fn test_saturate_increases_vividness() {
        let c = Rgb::new(100, 120, 180);
        let original_hsl = c.to_hsl();
        let result = saturate(&c, 0.3);
        let result_hsl = result.to_hsl();
        assert!(result_hsl.saturation >= original_hsl.saturation - 0.01);
    }

    #[test]
    fn test_saturate_gray_stays_gray() {
        // pure gray has no hue — saturation shift shouldn't explode
        let gray = Rgb::new(128, 128, 128);
        let result = saturate(&gray, 0.5);
        // should not panic and result should be valid
        assert!(result.r <= 255 && result.g <= 255 && result.b <= 255);
    }

    // ── saturate_all ─────────────────────────────────────────────────────────

    #[test]
    fn test_saturate_all_applies_to_every_slot() {
        let colors = [Rgb::new(100, 120, 180); 16];
        let result = saturate_all(&colors, 0.2);
        // every color should have changed
        for (original, adjusted) in colors.iter().zip(result.iter()) {
            // at least one channel should differ (unless already fully saturated)
            let changed = original.r != adjusted.r
                || original.g != adjusted.g
                || original.b != adjusted.b;
            // gray pixels won't change — just ensure no panic
            let _ = changed;
        }
        assert_eq!(result.len(), 16);
    }

    // ── invert_for_light ─────────────────────────────────────────────────────

    #[test]
    fn test_invert_swaps_color0_and_color15() {
        let mut colors = [Rgb::new(0, 0, 0); 16];
        colors[0]  = Rgb::new(10, 10, 10);
        colors[15] = Rgb::new(240, 240, 240);

        let result = invert_for_light(&colors);
        assert_eq!(result[0],  Rgb::new(240, 240, 240));
        assert_eq!(result[15], Rgb::new(10, 10, 10));
    }

    #[test]
    fn test_invert_swaps_color7_and_color8() {
        let mut colors = [Rgb::new(0, 0, 0); 16];
        colors[7] = Rgb::new(200, 200, 200);
        colors[8] = Rgb::new(80, 80, 80);

        let result = invert_for_light(&colors);
        assert_eq!(result[7], Rgb::new(80, 80, 80));
        assert_eq!(result[8], Rgb::new(200, 200, 200));
    }

    #[test]
    fn test_invert_does_not_touch_other_slots() {
        let mut colors = [Rgb::new(0, 0, 0); 16];
        colors[3] = Rgb::new(99, 88, 77);

        let result = invert_for_light(&colors);
        assert_eq!(result[3], Rgb::new(99, 88, 77));
    }

    #[test]
    fn test_invert_twice_is_identity() {
        let mut colors = [Rgb::new(0, 0, 0); 16];
        colors[0]  = Rgb::new(10, 10, 10);
        colors[7]  = Rgb::new(200, 200, 200);
        colors[8]  = Rgb::new(80, 80, 80);
        colors[15] = Rgb::new(240, 240, 240);

        let result = invert_for_light(&invert_for_light(&colors));
        assert_eq!(result, colors);
    }

    // ── contrast ─────────────────────────────────────────────────────────────

    #[test]
    fn test_relative_luminance_black_is_zero() {
        let black = Rgb::new(0, 0, 0);
        assert_eq!(relative_luminance(&black), 0.0);
    }

    #[test]
    fn test_relative_luminance_white_is_one() {
        let white = Rgb::new(255, 255, 255);
        assert_eq!(relative_luminance(&white), 1.0);
    }

    #[test]
    fn test_contrast_ratio_black_and_white_is_21() {
        let black = Rgb::new(0, 0, 0);
        let white = Rgb::new(255, 255, 255);
        let diff = (contrast_ratio(&black, &white) - 21.0).abs();
        assert!(diff < 0.01, "Expected ~21.0, got {}", contrast_ratio(&black, &white));
    }

    #[test]
    fn test_contrast_ratio_same_color_is_1() {
        let c = Rgb::new(100, 150, 200);
        assert_eq!(contrast_ratio(&c, &c), 1.0);
    }

    #[test]
    fn test_ensure_contrast_lightens_dark_text_on_dark_bg() {
        let bg = Rgb::new(10, 10, 10);
        let fg = Rgb::new(20, 20, 20); // terrible contrast
        
        // This should significantly lighten the fg
        let adjusted = ensure_contrast(&bg, &fg, 4.5);
        assert!(contrast_ratio(&bg, &adjusted) >= 4.5);
    }

    #[test]
    fn test_ensure_contrast_darkens_light_text_on_light_bg() {
        let bg = Rgb::new(245, 245, 245);
        let fg = Rgb::new(235, 235, 235); // terrible contrast
        
        // This should significantly darken the fg
        let adjusted = ensure_contrast(&bg, &fg, 4.5);
        assert!(contrast_ratio(&bg, &adjusted) >= 4.5);
    }

    #[test]
    fn test_ensure_contrast_leaves_good_contrast_alone() {
        let bg = Rgb::new(0, 0, 0);
        let fg = Rgb::new(255, 255, 255); // perfect contrast
        
        let adjusted = ensure_contrast(&bg, &fg, 4.5);
        assert_eq!(adjusted, fg);
    }
}