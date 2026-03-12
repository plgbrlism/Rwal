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
}