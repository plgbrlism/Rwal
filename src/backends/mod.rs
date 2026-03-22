/*

Take pre-sampled pixels, return (N) dominant colors. 
Selection by name string from CLI.

*/
use crate::colors::types::Rgb;
use crate::error::RwalError;

pub mod kmeans;
pub mod median_cut;

/// Every color extraction backend implements this trait.
/// Input: pre-sampled pixels from the image.
/// Output: `count` dominant colors.
pub trait Backend: Send + Sync {
    fn name(&self) -> &str;
    fn generate(&self, pixels: &[Rgb], count: usize, iterations: u8) -> Result<Vec<Rgb>, RwalError>;
}

/// Select a backend by name string (from CLI --backend flag).
pub fn from_name(name: &str) -> Result<Box<dyn Backend>, RwalError> {
    match name {
        "accurate"    => Ok(Box::new(kmeans::KMeans)),
        "fast"        => Ok(Box::new(median_cut::MedianCut)),
        other         => Err(RwalError::UnsupportedBackend(other.to_string())),
    }
}


/// Default backend used when none is specified.
pub fn default() -> Box<dyn Backend> {
    Box::new(kmeans::KMeans)
}