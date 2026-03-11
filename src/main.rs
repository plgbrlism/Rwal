// Declare the public modules that make up the application's functionality.
pub mod cli;
pub mod color_extractor;
pub mod config_writer;
pub mod image_loader;
pub mod palette_generator;
pub mod paths;
pub mod wallpaper;

// Import necessary items from other modules and external crates.
use anyhow::Result; // For flexible error handling.
use clap::Parser; // For parsing command-line arguments.
use cli::{Cli, Commands}; // For CLI structure and commands.

/// The main entry point of the application.
fn main() -> Result<()> {
    // Initialize the logger to enable logging throughout the application.
    env_logger::init();
    // Parse the command-line arguments provided by the user.
    let cli = Cli::parse();

    // Process the specific command given by the user.
    match cli.command {
        // If the command is "generate"...
        Commands::Generate(args) => {
            // Log the start of the palette generation process for debugging.
            log::info!("Generating color palette with args: {:?}", args);
            // Load the image from the path specified by the user.
            let image = image_loader::load_image(&args.input)?;
            // Extract a predefined number of colors (16) from the loaded image.
            let colors = color_extractor::extract_colors(&image, args.num_colors);
            // Generate a color palette from the extracted colors.
            let palette = palette_generator::generate_palette(colors, &args.mode, &args.input.to_string_lossy());

            // Check if the user wants to apply the palette (e.g., save it).
            if args.apply {
                // Get the path to the cache directory.
                let cache = paths::cache_dir();
                // Ensure the cache directory exists, creating it if necessary.
                std::fs::create_dir_all(&cache)?;
                // Write the generated color palette to a "colors.json" file in the cache directory.
                config_writer::write_config(&palette, &cache)?;
                log::info!("Color palette generated");
                // Log that the palette has been successfully saved.
                log::info!("Color palette written to cache");
                wallpaper::set_wallpaper("feh", &args.input);
            } else {
                // If not applying, convert the palette to a pretty-printed JSON string.
                let json_string = serde_json::to_string_pretty(&palette)?;
                // Print the JSON string to the standard output.
                println!("{}", json_string);
            }

        }
        // If the command is "apply"...
        Commands::Apply => {
            log::info!("Applying color palette");
            // Note: This is a placeholder for future functionality.
            // It would typically read a "colors.json" file and apply the theme.
            log::warn!("'apply' command is not fully implemented yet.");
        }
        // If the command is "set-wallpaper"...
        Commands::SetWallpaper(args) => {
            log::info!("Setting wallpaper with args: {:?}", args);
            // Call the function to set the desktop wallpaper.
            wallpaper::set_wallpaper(&args.backend, &args.path);
        }
    }

    // If all operations were successful, return Ok.
    Ok(())
}
