use anyhow::Result;
use crate::palette_generator::Palette;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn write_config(palette: &Palette, output_dir: &Path) -> Result<()> {
    let json_string = serde_json::to_string_pretty(palette)?;
    let mut file = File::create(output_dir.join("colors.json"))?;
    file.write_all(json_string.as_bytes())?;
    Ok(())
}
