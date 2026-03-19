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

    /// Palette generation mode
    #[arg(
        long = "mode",
        value_name = "NAME",
        default_value = "classic",
        value_parser = ["adaptive", "vibrant", "pastel", "classic"],
    )]
    pub mode: String,

    /// Load a saved theme by name instead of generating from image
    #[arg(long = "theme", value_name = "NAME")]
    pub theme: Option<String>,

    /// List all available themes (bundled + user)
    #[arg(long = "list-themes", default_value_t = false)]
    pub list_themes: bool,
 
    /// List all available color extraction backends
    #[arg(long = "list-backends", default_value_t = false)]
    pub list_backends: bool,
}

impl Cli {
    /// Validate that the user provided either -i, --theme, or -R.
    pub fn validate(&self) -> Result<(), String> {
        if self.list_themes || self.list_backends {
            // No validation needed when listing themes or backends
            return Ok(());
        }

        if self.image.is_none() && self.theme.is_none() && !self.restore {
            return Err(
                "no input provided — use -i <image>, --theme <n>, or -R to restore".into()
            );
        }
        Ok(())
    }
}