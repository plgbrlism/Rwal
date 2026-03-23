/*
CLI Commands:

-i <path>          image or directory
-l                 light mode
-w                 also set the wallpaper after generating colors
-n                 skip sequences + templates
-R                 restore last scheme (re-export from colors.json)
-q                 quiet
--backend <name>   kmeans:accurate (default) | fast:median_cut
--mode <name>      [default] classic:balanced |adaptive:dynamic | vibrant:neon | pastel:soft
--theme <name>     load a saved .json theme instead of image
--wallpaper        also apply the wallpaper using the detected backend
-g, --generate [N] render app configs (legacy flag)

Subcommands:
generate [APP]    render app configs from config-map.toml
preview           show semantic roles using dot-palette style
debug             check config-map.toml for errors
*/

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name    = "rwal",
    version = env!("CARGO_PKG_VERSION"),
    about   = "Generate accessible color schemes from images.\n\
               Managed via chained commands.\n\
               One flag — one function",
    long_about = None,
)]
pub struct Cli {
    /// Image file or directory to generate colors from
    #[arg(short = 'i', long = "image", value_name = "PATH")]
    pub image: Option<PathBuf>,

    /// Restores the last generated color scheme from cache
    #[arg(short = 'R', long = "restore", default_value_t = false, conflicts_with = "image")]
    pub restore: bool,

    /// Show semantic roles + base16 preview
    #[arg(short = 'p', long = "preview", default_value_t = false, conflicts_with_all = ["quiet"])]
    pub preview: bool,

    /// Validate config-map.toml mappings
    #[arg(short = 'd', long = "debug", default_value_t = false, conflicts_with_all = ["quiet"])]
    pub debug: bool,

    /// Render user templates from ~/.config/rwal/templates/
    #[arg(short = 'r', long = "render", default_value_t = false, conflicts_with = "noop")]
    pub render: bool,

    /// Map rendered templates to their final destinations
    #[arg(short = 'm', long = "map", value_name = "APP", num_args = 0..=1, conflicts_with = "noop",
    help = "Map rendered templates to their final destinations from config-map.toml\n\
            Can be standalone (use last cache) or chained with -i (new image).\n\
            Optional: specify a single app name to map (default: all)")]
    pub map: Option<Option<String>>,

    /// Generate a light color scheme
    #[arg(short = 'l', long = "light", default_value_t = false)]
    pub light: bool,

    /// Also apply the wallpaper after generating/restoring colors
    #[arg(short = 'w', long = "wallpaper", default_value_t = false, conflicts_with = "noop")]
    pub wallpaper: bool,

    /// Skip writing sequences and templates (preview only)
    #[arg(short = 'n', long = "noop", default_value_t = false, conflicts_with_all = ["render", "wallpaper"])]
    pub noop: bool,

    /// Quiet mode — suppress all output
    #[arg(short = 'q', long = "quiet", default_value_t = false, conflicts_with_all = ["preview", "debug"])]
    pub quiet: bool,

    /// Color Extraction Logic
    #[arg(
        long = "backend",
        value_name = "NAME",
        default_value = "accurate",
        value_parser = ["accurate", "fast"],
    )]
    pub backend: String,

    /// Palette Generation Mode
    #[arg(
        long = "mode",
        value_name = "NAME",
        default_value = "balanced",
        value_parser = ["balanced", "dynamic", "neon", "soft"],
    )]
    pub mode: String,
}

impl Cli {
    /// Validate that the user provided at least one primary action.
    pub fn validate(&self) -> Result<(), String> {
        if self.image.is_none() 
            && !self.restore 
            && self.map.is_none()
            && !self.render
            && !self.preview
            && !self.debug
        {
            return Err(
                "no action provided — use -i <image>, -R to restore, -m to map, -r to render, -p to preview, or -d to debug".into()
            );
        }
        Ok(())
    }
}