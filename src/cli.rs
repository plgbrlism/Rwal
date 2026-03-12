/*
CLI Commands:

-i <path>          image or directory
-l                 light mode
-n                 skip wallpaper
-s                 skip sequences + templates
-R                 restore last scheme (read cache, re-export)
-q                 quiet
--backend <name>   kmeans (default) | median_cut
--saturate <0.0-1.0>
--alpha <0-100>
--theme <name>     load a saved .json theme instead of image
*/
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name    = "rwal",
    version = env!("CARGO_PKG_VERSION"),
    about   = "Generate color schemes from images and apply them system-wide",
    long_about = None,
)]
pub struct Cli {
    /// Image file or directory to generate colors from
    #[arg(short = 'i', long = "image", value_name = "PATH")]
    pub image: Option<PathBuf>,

    /// Restore the last generated color scheme from cache
    #[arg(short = 'R', long = "restore", default_value_t = false)]
    pub restore: bool,

    /// Generate a light color scheme instead of dark
    #[arg(short = 'l', long = "light", default_value_t = false)]
    pub light: bool,

    /// Skip setting the wallpaper
    #[arg(short = 'n', long = "no-wallpaper", default_value_t = false)]
    pub no_wallpaper: bool,

    /// Skip applying sequences and rendering templates
    #[arg(short = 's', long = "no-sequences", default_value_t = false)]
    pub no_sequences: bool,

    /// Quiet mode — suppress all output
    #[arg(short = 'q', long = "quiet", default_value_t = false)]
    pub quiet: bool,

    /// Color extraction backend to use
    #[arg(
        long = "backend",
        value_name = "NAME",
        default_value = "kmeans",
        value_parser = ["kmeans", "median_cut"],
    )]
    pub backend: String,

    /// Number of k-means iterations (1-20, higher = more accurate but slower)
    #[arg(
        long = "accuracy",
        value_name = "N",
        default_value_t = 10,
        value_parser = clap::value_parser!(u8).range(1..=20),
    )]
    pub accuracy: u8,

    /// Shift color saturation (-1.0 to 1.0)
    #[arg(
        long = "saturate",
        value_name = "AMOUNT",
        allow_negative_numbers = true,
    )]
    pub saturate: Option<f32>,

    /// Transparency value written to colors.json (0-100)
    #[arg(
        long = "alpha",
        value_name = "N",
        default_value_t = 100,
        value_parser = clap::value_parser!(u8).range(0..=100),
    )]
    pub alpha: u8,

    /// Load a saved theme by name instead of generating from image
    #[arg(long = "theme", value_name = "NAME")]
    pub theme: Option<String>,
}

impl Cli {
    /// Validate that the user provided either -i, --theme, or -R.
    pub fn validate(&self) -> Result<(), String> {
        if self.image.is_none() && self.theme.is_none() && !self.restore {
            return Err(
                "no input provided — use -i <image>, --theme <n>, or -R to restore".into()
            );
        }
        Ok(())
    }
}