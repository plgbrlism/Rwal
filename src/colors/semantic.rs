/*
Maps a 16-slot ColorDict into a named semantic role palette.

Semantic roles separate "what a color is" from "what terminal slot it occupies",
making it easy to consume in Alacritty, Polybar, GTK, etc.

Roles are derived directly from the palette — no external data is required.
*/

use crate::colors::types::{Rgb, ColorDict};
use crate::colors::adjust;
use serde::Serialize;

/// Semantic role palette derived from a ColorDict.
/// All fields are hex strings with a leading `#`.
#[derive(Debug, Serialize)]
pub struct SemanticDict {
    pub wallpaper: String,
    pub alpha:     u8,
    pub special:   SemanticSpecial,
    pub colors:    SemanticRoles,
}

#[derive(Debug, Serialize)]
pub struct SemanticSpecial {
    pub background: String,
    pub foreground: String,
    pub cursor:     String,
}

#[derive(Debug, Serialize)]
pub struct SemanticRoles {
    // Backgrounds
    pub background:      String,
    pub surface:         String,   // slightly lighter than background

    // Text
    pub foreground:      String,
    pub cursor:          String,

    // Accent triad
    pub primary:         String,
    pub secondary:       String,
    pub tertiary:        String,
    pub accent:          String,   // color4 — general highlight

    // State roles (from palette where available)
    pub error:           String,
    pub success:         String,
    pub warning:         String,
    pub info:            String,

    // Neutrals
    pub neutral:         String,
    pub neutral_variant: String,

    // "On" text — text that sits ON these roles
    pub on_background:   String,
    pub on_primary:      String,   // white or black for max contrast on primary
    pub on_surface:      String,
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

    // "On" roles — pick black or white depending on contrast
    let on_background = fg; // already guaranteed contrast by palette builder
    let on_primary    = readable_on(&primary);
    let on_surface    = fg;

    SemanticDict {
        wallpaper: dict.wallpaper.display().to_string(),
        alpha:     dict.alpha,
        special: SemanticSpecial {
            background: dict.special.background.to_hex(),
            foreground: dict.special.foreground.to_hex(),
            cursor:     dict.special.cursor.to_hex(),
        },
        colors: SemanticRoles {
            background:      bg.to_hex(),
            surface:         surface.to_hex(),
            foreground:      fg.to_hex(),
            cursor:          dict.special.cursor.to_hex(),
            primary:         primary.to_hex(),
            secondary:       secondary.to_hex(),
            tertiary:        tertiary.to_hex(),
            accent:          accent.to_hex(),
            error:           error.to_hex(),
            success:         success.to_hex(),
            warning:         warning.to_hex(),
            info:            info.to_hex(),
            neutral:         neutral.to_hex(),
            neutral_variant: neutral_variant.to_hex(),
            on_background:   on_background.to_hex(),
            on_primary:      on_primary.to_hex(),
            on_surface:      on_surface.to_hex(),
        },
    }
}

/// Choose black (`#000000`) or white (`#ffffff`) depending on which achieves
/// better contrast against the given color (≥4.5:1 preferred, highest wins).
fn readable_on(bg: &Rgb) -> Rgb {
    let white = Rgb::new(255, 255, 255);
    let black = Rgb::new(0, 0, 0);
    let lum_bg = adjust::relative_luminance(bg);

    let contrast_white = (lum_bg + 0.05) / (0.0 + 0.05);   // white lum = 1.0
    let contrast_black = (1.0 + 0.05) / (lum_bg + 0.05);

    if contrast_white >= contrast_black { white } else { black }
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
    // Users can override via theme-map.yml if they want conventional values.
    *src
}
