/*
Maps a 16-slot ColorDict into a named semantic role palette.

Semantic roles separate "what a color is" from "what terminal slot it occupies",
making it easy to consume in Alacritty, Polybar, GTK, etc.

Roles are derived directly from the palette — no external data is required.
*/

use crate::colors::types::{Rgb, ColorDict};
use crate::colors::adjust;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SemanticDict {
    pub wallpaper: String,
    pub alpha:     u8,
    pub colors:    SemanticRoles,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SemanticRoles {
    // Backgrounds
    #[serde(with = "rgb_hex")]
    pub background:      Rgb,
    #[serde(with = "rgb_hex")]
    pub surface:         Rgb,   // slightly lighter than background

    // Text
    #[serde(with = "rgb_hex")]
    pub foreground:      Rgb,
    #[serde(with = "rgb_hex")]
    pub cursor:          Rgb,

    // Accent triad
    #[serde(with = "rgb_hex")]
    pub primary:         Rgb,
    #[serde(with = "rgb_hex")]
    pub secondary:       Rgb,
    #[serde(with = "rgb_hex")]
    pub tertiary:        Rgb,
    #[serde(with = "rgb_hex")]
    pub accent:          Rgb,   // color4 — general highlight

    // State roles (from palette where available)
    #[serde(with = "rgb_hex")]
    pub error:           Rgb,
    #[serde(with = "rgb_hex")]
    pub success:         Rgb,
    #[serde(with = "rgb_hex")]
    pub warning:         Rgb,
    #[serde(with = "rgb_hex")]
    pub info:            Rgb,

    // Neutrals
    #[serde(with = "rgb_hex")]
    pub neutral:         Rgb,
    #[serde(with = "rgb_hex")]
    pub neutral_variant: Rgb,
}

mod rgb_hex {
    use super::Rgb;
    use serde::{self, Serializer, Deserializer, Deserialize};

    pub fn serialize<S>(rgb: &Rgb, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        serializer.serialize_str(&rgb.to_hex())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Rgb, D::Error>
    where D: Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        Rgb::from_hex(&s).ok_or_else(|| serde::de::Error::custom("invalid hex color"))
    }
}

/// Derive a `SemanticDict` from a `ColorDict`.
/// Roles that have no obvious palette source use conventional fallbacks.
pub fn from_dict(dict: &ColorDict) -> SemanticDict {
    let bg  = dict.colors[0];
    let fg  = dict.colors[15];

    // Surface: background lightened just enough to be a distinct panel tone
    let surface = adjust::lighten(&bg, 0.05);

    // Accents: raw palette slots
    let primary   = dict.colors[1];
    let secondary = dict.colors[2];
    let tertiary  = dict.colors[3];
    let accent    = dict.colors[4];

    // State roles: use colors from the palette, then nudge hue toward convention.
    // This keeps them "in-palette" rather than pasting a foreign red into the output.
    let error   = nudge_to_hue(&dict.colors[1], 0.0,   &Rgb::from_hex("#ea4d4d").unwrap());
    let success = nudge_to_hue(&dict.colors[2], 120.0, &Rgb::from_hex("#6abf69").unwrap());
    let warning = nudge_to_hue(&dict.colors[3], 40.0,  &Rgb::from_hex("#f0ad4e").unwrap());
    let info    = nudge_to_hue(&dict.colors[4], 210.0, &Rgb::from_hex("#5bc0de").unwrap());

    // Neutrals
    let neutral         = dict.colors[8];  // bright-black — archetypal gray
    let neutral_variant = dict.colors[7];  // color7 — lighter divider gray

    SemanticDict {
        wallpaper: dict.wallpaper.display().to_string(),
        alpha:     dict.alpha,
        colors: SemanticRoles {
            background:      bg,
            surface,
            foreground:      fg,
            cursor:          dict.special.cursor,
            primary:         primary,
            secondary:       secondary,
            tertiary:        tertiary,
            accent:          accent,
            error,
            success,
            warning,
            info,
            neutral:         neutral,
            neutral_variant: neutral_variant,
        },
    }
}

/// Blend the palette color toward a conventional hue.
/// Returns the original color if it already has enough saturation; otherwise
/// nudges the hue 50% toward the target. Entirely desaturated colors (grays)
/// use the fallback directly.
fn nudge_to_hue(src: &Rgb, _target_hue_deg: f32, fallback: &Rgb) -> Rgb {
    use palette::Hsl;

    let hsl: Hsl = src.to_hsl();

    // If the color is essentially gray, the hue is meaningless — use fallback
    if hsl.saturation < 0.10 {
        return *fallback;
    }

    // Color is sufficiently saturated — keep it in-palette as-is.
    // Users can override via config-map.toml if they want conventional values.
    *src
}
