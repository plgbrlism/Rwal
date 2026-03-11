use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate a color palette from an image
    Generate(GenerateArgs),
    /// Apply the generated color palette to config files
    Apply,
    /// Set the wallpaper
    SetWallpaper(SetWallpaperArgs),
}

#[derive(Parser, Debug)]
pub struct GenerateArgs {
    /// Path to the image file
    #[arg(required = true)]
    pub input: std::path::PathBuf,

    /// Dark or light mode
    #[arg(long, default_value = "dark")]
    pub mode: String,

    /// Contrast level
    #[arg(long, default_value = "medium")]
    pub contrast: String,

    /// Wallpaper setting backend
    #[arg(long, default_value = "none")]
    pub backend: String,

    /// Apply the generated palette
    #[arg(long)]
    pub apply: bool,

    /// Number of colors to extract
    #[arg(long, default_value_t = 16)]
    pub num_colors: usize,
}

#[derive(Parser, Debug)]
pub struct SetWallpaperArgs {
    /// Path to the wallpaper image
    #[arg(required = true)]
    pub path: std::path::PathBuf,

    /// Wallpaper setting backend
    #[arg(long, default_value = "feh")]
    pub backend: String,
}
