pub mod cli;
pub mod color_extractor;
pub mod config_writer;
pub mod image_loader;
pub mod palette_generator;
pub mod wallpaper;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use std::path::Path;

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate(args) => {
            log::info!("Generating color palette with args: {:?}", args);
            let image = image_loader::load_image(&args.input)?;
            let colors = color_extractor::extract_colors(&image, 16);
            let palette = palette_generator::generate_palette(colors);
            
            if args.apply {
                config_writer::write_config(&palette, Path::new("."))?;
                log::info!("Color palette written to colors.json");
            } else {
                let json_string = serde_json::to_string_pretty(&palette)?;
                println!("{}", json_string);
            }

            if args.backend != "none" {
                wallpaper::set_wallpaper(&args.backend, &args.input);
            }
        }
        Commands::Apply => {
            log::info!("Applying color palette");
            // This would typically read colors.json and apply templates.
            // For the prototype, we'll just log that it's not implemented.
            log::warn!("'apply' command is not fully implemented yet.");
        }
        Commands::SetWallpaper(args) => {
            log::info!("Setting wallpaper with args: {:?}", args);
            wallpaper::set_wallpaper(&args.backend, &args.path);
        }
    }

    Ok(())
}
