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
→ render templates (export::templates)          [only with --template]
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
// use colors::types::Rgb;
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
    // ── 1. Init ─────────────────────────────────────────────────────────────
    let paths = paths::Paths::resolve()?;
    paths.ensure_config()?;

    // ── 2. Resolve ColorDict (Source) ───────────────────────────────────────
    let (dict, is_new) = if let Some(image_arg) = &cli.image {
        // A. Image extraction path
        let image_path = image::loader::resolve(image_arg)?;
        step(&cli, &format!("image: {}", image_path.display()));

        let file_size = cache::scheme::file_size(&image_path);
        let key = cache::scheme::cache_key(&image_path, &cli.backend, &cli.mode, cli.light, file_size);

        if let Ok(Some(cached)) = cache::scheme::load(&paths, &key) {
            step(&cli, "colors: loaded from cache");
            (cached, true) // is_new=true means we should write JSONs/Sequences
        } else {
            step(&cli, &format!("backend: {}", cli.backend));
            let backend = backends::from_name(&cli.backend)?;
            let raw = colors::extractor::extract(&image_path, backend.as_ref(), 16, 10)?;
            step(&cli, "colors: extracted");

            let dict = colors::palette::build(raw, image_path.clone(), 100, cli.light, &cli.mode)?;
            if let Err(e) = cache::scheme::save(&paths, &key, &dict) { warn(&e); }
            (dict, true)
        }
    } else if cli.restore {
        // B. Restore path (explicitly reload then re-apply state)
        let dict = export::colors_json::read(&paths)?;
        step(&cli, "restore: loaded last scheme");
        (dict, true)
    } else {
        // C. Standalone path (read from JSON cache for flags like -r, -p, -d)
        if !paths.base16_json.exists() || !paths.semantic_json.exists() {
            return Err(error::RwalError::CacheReadError(
                paths.base16_json.clone(),
                "one or both color JSONs are missing — run with -i <image> first".to_string(),
            ));
        }
        let dict = export::colors_json::read(&paths)?;
        (dict, false)
    };


    // ── 3. Base Actions (only if new extraction or explicit restore) ────────
    if is_new {
        // Write JSON exports
        export::colors_json::write_base16(&paths, &dict)?;
        if let Err(e) = export::colors_json::write_semantic(&paths, &dict) { warn(&e); }
        step(&cli, "updated: color caches");

        // Apply hot-reload state unless noop
        if !cli.noop {
            if let Err(e) = export::sequences::apply(&paths, &dict) { warn(&e); }
            step(&cli, "applied: hot-reload sequences");
        }


        // Apply wallpaper
        if cli.wallpaper && !dict.wallpaper.as_os_str().is_empty() {
            if let Err(e) = wallpaper::set(&dict.wallpaper) { warn(&e); }
            else { step(&cli, &format!("wallpaper: {}", dict.wallpaper.display())); }
        }

        // Default preview in image/restore flow (skip if user wants explicit full preview or debug)
        if !cli.quiet && !cli.preview && !cli.debug {
            let semantic = colors::semantic::from_dict(&dict);
            export::generate::preview(&semantic, None);
        }

    }

    // ── 4. Chained Action Flags ─────────────────────────────────────────────
    let semantic = colors::semantic::from_dict(&dict);

    // --render [<APP>]
    if let Some(app_opt) = &cli.render {
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

    // --preview (-p)
    if cli.preview {
        export::generate::preview(&semantic, Some(&dict));
    }

    // --debug (-d)
    if cli.debug {
        export::generate::debug(&paths, &semantic)?;
    }

    // --template (-t)
    if cli.template {
        if let Err(e) = export::templates::render_all(&paths, &dict) {
            warn(&e);
        } else {
            step(&cli, "rendered user templates to ~/.cache/rwal/");
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
 