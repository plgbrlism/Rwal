use palette::Srgb;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Palette {
    pub background: String,
    pub foreground: String,
    pub accent: String,
    pub secondary: String,
}

fn to_hex(color: Srgb<u8>) -> String {
    format!("#{:02x}{:02x}{:02x}", color.red, color.green, color.blue)
}

pub fn generate_palette(colors: Vec<Srgb<u8>>) -> Palette {
    if colors.len() < 4 {
        // Not enough colors to generate a meaningful palette
        return Palette {
            background: "#000000".to_string(),
            foreground: "#ffffff".to_string(),
            accent: "#ff0000".to_string(),
            secondary: "#00ff00".to_string(),
        };
    }

    Palette {
        background: to_hex(colors[0]),
        foreground: to_hex(colors[1]),
        accent: to_hex(colors[2]),
        secondary: to_hex(colors[3]),
    }
}
