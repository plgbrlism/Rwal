#![allow(dead_code)]
/*
Full pipeline in order:

parse cli args
→ resolve image (image::loader)
→ check cache (cache::scheme)
→ if miss: extract colors (colors::extractor) → adjust palette (colors::palette)
→ write cache
→ write colors.json (export::colors_json)
→ render templates (export::templates)          [unless -s skipped]
→ send sequences (export::sequences)            [unless -s skipped]
→ set wallpaper (wallpaper)                     [unless -n skipped]
*/

mod error;
mod paths;
mod colors;
mod image;
mod backends;
mod cache;
mod export;
mod wallpaper;
mod cli;

use clap::Parser;
use cli::Cli;
use colors::types::Rgb;
use error::warn;
 
fn main() {
    let cli = Cli::parse();
 
    if let Err(e) = cli.validate() {
        eprintln!("\x1b[31merror\x1b[0m: {e}");
        std::process::exit(1);
    }
 
    if let Err(e) = run(cli) {
        error::error(&e);
        std::process::exit(1);
    }
}
 
fn run(cli: Cli) -> Result<(), error::RwalError> {
    // ── 1. Setup paths ───────────────────────────────────────────────────────
    let paths = paths::Paths::resolve()?;
    paths.ensure_dirs()?;

    // Handle --list-themes and --list-backends before doing any work
    if cli.list_themes {
        export::theme::list_all(&paths);
        return Ok(());
    }
    if cli.list_backends {
        println!("kmeans\nmedian_cut");
        return Ok(());
    }

    // ── 2. Resolve ColorDict ─────────────────────────────────────────────────
    let dict = if let Some(name) = &cli.theme {
        // --theme: skip image extraction entirely, load from colorschemes/
        let dict = export::theme::load(&paths, name)?;
        step(&cli, &format!("theme: {name}"));
        dict

    } else if cli.restore {
        // --restore: re-export last scheme from colors.json
        return restore(&paths, &cli);

    } else {
        // Normal path: extract from image
        let image_path = match &cli.image {
            Some(p) => image::loader::resolve(p)?,
            None => return Err(error::RwalError::ImageNotFound(
                std::path::PathBuf::from("<no image>"),
            )),
        };

        step(&cli, &format!("image: {}", image_path.display()));

        // ── 3. Check cache ───────────────────────────────────────────────────
        let file_size = cache::scheme::file_size(&image_path);
        let key = cache::scheme::cache_key(
            &image_path,
            &cli.backend,
            &cli.mode,
            cli.light,
            file_size,
        );

        match cache::scheme::load(&paths, &key) {
            Ok(Some(cached)) => {
                step(&cli, "colors: loaded from cache");
                cached
            }
            Ok(None) | Err(_) => {
                // ── 4. Extract colors ────────────────────────────────────────
                step(&cli, &format!("backend: {}", cli.backend));

                let backend = backends::from_name(&cli.backend)?;
                let raw = colors::extractor::extract(
                    &image_path,
                    backend.as_ref(),
                    16,
                    10,
                )?;

                step(&cli, "colors: extracted");

                // ── 5. Build palette ─────────────────────────────────────────
                let dict = colors::palette::build(
                    raw,
                    image_path.clone(),
                    100,
                    cli.light,
                    None,
                    &cli.mode,
                )?;

                // ── 6. Write cache ───────────────────────────────────────────
                if let Err(e) = cache::scheme::save(&paths, &key, &dict) {
                    warn(&e);
                }

                dict
            }
        }
    };

    // ── 7. Write colors.json ─────────────────────────────────────────────────
    export::colors_json::write(&paths, &dict)?;
    step(&cli, "wrote: ~/.cache/rwal/colors.json");

    // ── 8. Render templates + sequences ─────────────────────────────────────
    if !cli.no_sequences {
        if let Err(e) = export::templates::render_all(&paths, &dict) {
            warn(&e);
        }
        step(&cli, "rendered: templates");

        if let Err(e) = export::sequences::apply(&paths, &dict) {
            warn(&e);
        }
        step(&cli, "applied: terminal sequences");
    }

    // ── 9. Set wallpaper ─────────────────────────────────────────────────────
    if !cli.no_wallpaper && !dict.wallpaper.as_os_str().is_empty() {
        if let Err(e) = wallpaper::set(&dict.wallpaper) {
            warn(&e);
        } else {
            step(&cli, &format!("wallpaper: {}", dict.wallpaper.display()));
        }
    }

    // ── 10. Print palette ────────────────────────────────────────────────────
    if !cli.quiet {
        print_palette(&dict.colors);
    }

    Ok(())
}

/// Restore the last scheme from colors.json and re-export without regenerating.
fn restore(paths: &paths::Paths, cli: &Cli) -> Result<(), error::RwalError> {
    let dict = export::colors_json::read(paths)?;

    step(cli, "restore: loaded last scheme");

    if !cli.no_sequences {
        if let Err(e) = export::templates::render_all(paths, &dict) {
            warn(&e);
        }
        if let Err(e) = export::sequences::apply(paths, &dict) {
            warn(&e);
        }
    }

    if !cli.quiet {
        print_palette(&dict.colors);
    }

    Ok(())
}
 
/// Print a step message unless quiet mode is on.
fn step(cli: &Cli, msg: &str) {
    if !cli.quiet {
        println!("\x1b[32m::\x1b[0m {msg}");
    }
}
 
/// Print all 16 colors as colored blocks with hex values.
fn print_palette(colors: &[Rgb; 16]) {
    println!();
 
    // Top row: color blocks
    for (i, color) in colors.iter().enumerate() {
        print!(
            "\x1b[48;2;{};{};{}m  \x1b[0m",
            color.r, color.g, color.b
        );
        if i == 7 {
            println!();
        }
    }
    println!();
 
    // Bottom row: hex values
    for (i, color) in colors.iter().enumerate() {
        print!("{} ", color.to_hex());
        if i == 7 {
            println!();
        }
    }
    println!();
}
 