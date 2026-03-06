use anyhow::Result;
use image::{DynamicImage, io::Reader};

pub fn load_image(path: &std::path::Path) -> Result<DynamicImage> {
    let image = Reader::open(path)?.decode()?;
    Ok(image)
}
