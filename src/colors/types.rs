use std::path::PathBuf;
use palette::{FromColor, Hsl, IntoColor, Srgb};
use serde::{Deserialize, Serialize};

use std::fmt;

// ─── Rgb ────────────────────────────────────────────────────────────────────

/// A single 8-bit RGB color.
/// Thin wrapper around palette's Srgb<u8> so the rest of the codebase
/// doesn't have to import palette directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl Rgb {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parse a hex string like "#1a2b3c" or "1a2b3c".
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Self { r, g, b })
    }

    /// Format as "#rrggbb".
    pub fn to_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// YIQ perceptual luminance — used to sort colors dark→light.
    /// Same formula pywal uses internally.
    pub fn luminance(&self) -> f32 {
        0.299 * self.r as f32 + 0.587 * self.g as f32 + 0.114 * self.b as f32
    }

    /// Convert to palette's Srgb<f32> for HSL operations.
    pub fn to_srgb_f32(&self) -> Srgb<f32> {
        Srgb::new(
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
        )
    }

    /// Convert to HSL via palette.
    pub fn to_hsl(&self) -> Hsl {
        Hsl::from_color(self.to_srgb_f32())
    }

    /// Build from palette's Srgb<f32>.
    pub fn from_srgb_f32(c: Srgb<f32>) -> Self {
        Self {
            r: (c.red.clamp(0.0, 1.0) * 255.0).round() as u8,
            g: (c.green.clamp(0.0, 1.0) * 255.0).round() as u8,
            b: (c.blue.clamp(0.0, 1.0) * 255.0).round() as u8,
        }
    }

    /// Build from HSL via palette.
    pub fn from_hsl(hsl: Hsl) -> Self {
        let srgb: Srgb<f32> = hsl.into_color();
        Self::from_srgb_f32(srgb)
    }
}

// ─── Special ────────────────────────────────────────────────────────────────

/// The three semantic colors pywal exposes as "special".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Special {
    pub background: Rgb,
    pub foreground: Rgb,
    pub cursor:     Rgb,
}

// ─── ColorDict ──────────────────────────────────────────────────────────────

/// The canonical color scheme — mirrors pywal's output JSON exactly.
///
/// ```json
/// {
///   "wallpaper": "/path/to/img.jpg",
///   "alpha": 100,
///   "special": { "background": "#1a1a2e", "foreground": "#cdd6f4", "cursor": "#cdd6f4" },
///   "colors": { "color0": "#1a1a2e", ..., "color15": "#cdd6f4" }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorDict {
    pub wallpaper: PathBuf,
    pub alpha:     u8,
    pub special:   Special,
    pub colors:    [Rgb; 16],
}

impl ColorDict {
    /// Convenience accessor — color0..color15 by index.
    pub fn color(&self, index: usize) -> &Rgb {
        &self.colors[index]
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // --- Rgb::from_hex ---

    #[test]
    fn test_from_hex_with_hash() {
        let c = Rgb::from_hex("#1a2b3c").unwrap();
        assert_eq!(c.r, 0x1a);
        assert_eq!(c.g, 0x2b);
        assert_eq!(c.b, 0x3c);
    }

    #[test]
    fn test_from_hex_without_hash() {
        let c = Rgb::from_hex("ff0000").unwrap();
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 0);
    }

    #[test]
    fn test_from_hex_black() {
        let c = Rgb::from_hex("#000000").unwrap();
        assert_eq!(c, Rgb::new(0, 0, 0));
    }

    #[test]
    fn test_from_hex_white() {
        let c = Rgb::from_hex("#ffffff").unwrap();
        assert_eq!(c, Rgb::new(255, 255, 255));
    }

    #[test]
    fn test_from_hex_invalid_length() {
        assert!(Rgb::from_hex("#fff").is_none());
        assert!(Rgb::from_hex("12345").is_none());
        assert!(Rgb::from_hex("1234567").is_none());
    }

    #[test]
    fn test_from_hex_invalid_chars() {
        assert!(Rgb::from_hex("#gggggg").is_none());
    }

    // --- Rgb::to_hex ---

    #[test]
    fn test_to_hex_roundtrip() {
        let original = "#1a2b3c";
        let c = Rgb::from_hex(original).unwrap();
        assert_eq!(c.to_hex(), original);
    }

    #[test]
    fn test_to_hex_black() {
        assert_eq!(Rgb::new(0, 0, 0).to_hex(), "#000000");
    }

    #[test]
    fn test_to_hex_white() {
        assert_eq!(Rgb::new(255, 255, 255).to_hex(), "#ffffff");
    }

    #[test]
    fn test_to_hex_lowercase() {
        // must be lowercase to match pywal output format
        assert_eq!(Rgb::new(0xAB, 0xCD, 0xEF).to_hex(), "#abcdef");
    }

    // --- Rgb::luminance ---

    #[test]
    fn test_black_has_lowest_luminance() {
        let black = Rgb::new(0, 0, 0);
        let white = Rgb::new(255, 255, 255);
        assert!(black.luminance() < white.luminance());
    }

    #[test]
    fn test_white_has_highest_luminance() {
        let white = Rgb::new(255, 255, 255); // 0.299*255 + 0.587*255 + 0.114*255 = 255.0
        assert!(white.luminance() > 254.0);
    }

    #[test]
    fn test_green_heavier_than_red_in_luminance() {
        // YIQ weights green (0.587) more than red (0.299)
        let red   = Rgb::new(255, 0, 0);
        let green = Rgb::new(0, 255, 0);
        assert!(green.luminance() > red.luminance());
    }

    // --- HSL roundtrip ---

    #[test]
    fn test_hsl_roundtrip_is_lossless_within_tolerance() {
        let original = Rgb::new(100, 150, 200);
        let roundtripped = Rgb::from_hsl(original.to_hsl());
        // allow ±1 per channel due to float rounding
        assert!((original.r as i16 - roundtripped.r as i16).abs() <= 1);
        assert!((original.g as i16 - roundtripped.g as i16).abs() <= 1);
        assert!((original.b as i16 - roundtripped.b as i16).abs() <= 1);
    }

    #[test]
    fn test_hsl_roundtrip_black() {
        let c = Rgb::new(0, 0, 0);
        let rt = Rgb::from_hsl(c.to_hsl());
        assert_eq!(rt, c);
    }

    #[test]
    fn test_hsl_roundtrip_white() {
        let c = Rgb::new(255, 255, 255);
        let rt = Rgb::from_hsl(c.to_hsl());
        assert_eq!(rt, c);
    }

    // --- serde roundtrip ---

    #[test]
    fn test_rgb_serde_roundtrip() {
        let c = Rgb::new(10, 20, 30);
        let json = serde_json::to_string(&c).unwrap();
        let back: Rgb = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn test_color_dict_serde_roundtrip() {
        let dict = ColorDict {
            wallpaper: PathBuf::from("/tmp/wall.jpg"),
            alpha: 100,
            special: Special {
                background: Rgb::new(0, 0, 0),
                foreground: Rgb::new(255, 255, 255),
                cursor:     Rgb::new(255, 255, 255),
            },
            colors: [Rgb::new(0, 0, 0); 16],
        };
        let json = serde_json::to_string(&dict).unwrap();
        let back: ColorDict = serde_json::from_str(&json).unwrap();
        assert_eq!(dict, back);
    }

    // --- ColorDict::color() accessor ---

    #[test]
    fn test_color_accessor_returns_correct_slot() {
        let mut colors = [Rgb::new(0, 0, 0); 16];
        colors[7] = Rgb::new(1, 2, 3);

        let dict = ColorDict {
            wallpaper: PathBuf::from("/tmp/wall.jpg"),
            alpha: 100,
            special: Special {
                background: Rgb::new(0, 0, 0),
                foreground: Rgb::new(255, 255, 255),
                cursor:     Rgb::new(255, 255, 255),
            },
            colors,
        };

        assert_eq!(dict.color(7), &Rgb::new(1, 2, 3));
        assert_eq!(dict.color(0), &Rgb::new(0, 0, 0));
    }
}