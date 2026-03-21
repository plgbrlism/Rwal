#![allow(dead_code)]
/*
Full pipeline in order:

parse cli args
→ resolve image (image::loader)
→ check cache (cache::scheme)
→ if miss: extract colors (colors::extractor) → adjust palette (colors::palette)
→ write cache
→ write base16-colors.json  (export::colors_json::write_base16)
→ write semantic-colors.json (export::colors_json::write_semantic)
→ render templates (export::templates)          [unless -s skipped]
→ send sequences (export::sequences)            [unless -s skipped]
→ set wallpaper (wallpaper)                     [only with --wallpaper]
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
    paths.ensure_config()?;

    // Handle --list-themes and --list-backends before doing any work
    if cli.list_themes {
        export::theme::list_all(&paths);
        return Ok(());
    }
    if cli.list_backends {
        println!("kmeans\nmedian_cut");
        return Ok(());
    }

    // ── 2. Subcommands ───────────────────────────────────────────────────────
    if let Some(cmd) = &cli.command {
        match cmd {
            cli::Commands::Generate { app } => {
                let dict = export::colors_json::read(&paths)?;
                let semantic = colors::semantic::from_dict(&dict);
                match app {
                    Some(name) => export::generate::render_one(&paths, &semantic, name)?,
                    None => export::generate::render_all(&paths, &semantic)?,
                }
                step(&cli, "generated configs from cache");
                return Ok(());
            }
            cli::Commands::Preview => {
                let dict = export::colors_json::read(&paths)?;
                let semantic = colors::semantic::from_dict(&dict);
                export::generate::preview(&semantic);
                return Ok(());
            }
            cli::Commands::Debug => {
                let dict = export::colors_json::read(&paths)?;
                let semantic = colors::semantic::from_dict(&dict);
                export::generate::debug(&paths, &semantic)?;
                return Ok(());
            }
        }
    }


    // ── 3. Resolve ColorDict ─────────────────────────────────────────────────
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

    // ── 7. Write dual JSON outputs ──────────────────────────────────────────
    export::colors_json::write_base16(&paths, &dict)?;
    step(&cli, "wrote: base16-colors.json");

    if let Err(e) = export::colors_json::write_semantic(&paths, &dict) {
        warn(&e);
    } else {
        step(&cli, "wrote: semantic-colors.json");
    }

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

    // ── 9. Set wallpaper (opt-in via --wallpaper / -w) ───────────────────────
    if cli.wallpaper && !dict.wallpaper.as_os_str().is_empty() {
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

    // ── 11. Generate app configs (opt-in via -r / --render) ─────────────────
    if let Some(ref app_opt) = cli.render {
        // Read the semantic dict back from disk (or derive it from in-memory dict)
        let semantic = colors::semantic::from_dict(&dict);
        
        match app_opt {
            Some(app_name) => {
                export::generate::render_one(&paths, &semantic, app_name)?;
                step(&cli, &format!("generated config for: {app_name}"));
            }
            None => {
                export::generate::render_all(&paths, &semantic)?;
                step(&cli, "generated all configs from theme-map.toml");
            }
        }
    }

    Ok(())
}

/// Restore the last scheme from colors.json and re-export without regenerating.
/// Always re-applies terminal sequences (hot-reload). Wallpaper is opt-in via --wallpaper.
fn restore(paths: &paths::Paths, cli: &Cli) -> Result<(), error::RwalError> {
    let dict = export::colors_json::read(paths)?;

    step(cli, "restore: loaded last scheme");

    // Always render templates and hot-reload all open terminals from colors.json
    if !cli.no_sequences {
        if let Err(e) = export::templates::render_all(paths, &dict) {
            warn(&e);
        }
        step(cli, "rendered: templates");

        if let Err(e) = export::sequences::apply(paths, &dict) {
            warn(&e);
        }
        step(cli, "applied: terminal sequences");
    }

    // Wallpaper is opt-in: only set it when --wallpaper / -w is passed
    if cli.wallpaper && !dict.wallpaper.as_os_str().is_empty() {
        if let Err(e) = wallpaper::set(&dict.wallpaper) {
            warn(&e);
        } else {
            step(cli, &format!("wallpaper: {}", dict.wallpaper.display()));
        }
    }

    if !cli.quiet {
        print_palette(&dict.colors);
    }

    // ── Generate app configs (opt-in via -r / --render) ─────────────────────
    if let Some(app_opt) = &cli.render {
        let semantic = colors::semantic::from_dict(&dict);
        match app_opt {
            Some(app_name) => {
                export::generate::render_one(paths, &semantic, app_name)?;
                step(cli, &format!("generated config for: {app_name}"));
            }
            None => {
                export::generate::render_all(paths, &semantic)?;
                step(cli, "generated all configs from theme-map.toml");
            }
        }
    }

    Ok(())
}
 
/// Print a step message unless quiet mode is on.
fn step(cli: &Cli, msg: &str) {
    if !cli.quiet {
        println!("\x1b[32m::\x1b[0m {msg}");
    }
}
 
/// Print all 16 colors as colored blocks with hex values natively overlaid inside.
fn print_palette(colors: &[Rgb; 16]) {
    println!();
 
    for row in 0..4 {
        for col in 0..4 {
            let c = &colors[row * 4 + col];
            let fg = if crate::colors::adjust::relative_luminance(c) > 0.5 {
                "\x1b[38;2;0;0;0m" // Black text on bright blocks
            } else {
                "\x1b[38;2;255;255;255m" // White text on dark blocks
            };
            
            print!("\x1b[48;2;{};{};{}m{} {} \x1b[0m", c.r, c.g, c.b, fg, c.to_hex());
        }
        println!();
    }
    println!();
}
 