# rwal - A Rust Wallpaper and Color Palette Tool

`rwal` is a fast, robust command-line tool written in Rust that instantly generates gorgeous, readable color palettes from any image and seamlessly applies them system-wide to your terminal, wallpaper, and programs. 

It is designed as a drop-in, memory-safe, entirely statically-typed replacement for the standard python-based `pywal`.

## Current Features

* **Blazing Fast Color Extraction**: Extracts dominant colors instantly. Offers two selectable backends:
  * `kmeans` (Default): Uses the highly-accurate K-Means++ algorithm for perfectly balanced dominant colors. You can tune the accuracy loops using the `--accuracy N` flag.
  * `median_cut`: An extremely fast, memory-optimized algorithm perfect for huge images or lower-end machines.
* **Smart Palette Strategies**: Unlike standard pywal, `rwal` can generate multiple aesthetically pleasing themes based on the image's colors:
  * `classic` (Default): Mimics the standard pywal generation formula.
  * `adaptive`: Actively scans the image's overall median luminance. If the image is dark, it lightens and saturates the terminal colors so they pop. If the image is bright, it darkens them.
  * `vibrant`: Takes the raw, true accent colors from the image and applies the adaptive luminance logic directly to them.
  * `pastel`: Mutes contrast and applies a light brightness bump to create soft, pastel themes.
  * `complementary`: Picks a vibrant base color and generates its exact mathematical complement (+180°).
  * `split_complementary`: Takes the base color and splits the complement into a pleasing Y-shape (+150° and +210°).
  * `analogous`: Uses perfectly adjacent colors on the wheel (-30° and +30°).
  * `monochromatic`: Generates an entire palette from different lightness and saturation values of a single base color.
  * `triadic`: Spaces three distinct colors out mathematically (+120° and +240°).
* **Guaranteed Terminal Readability**: `rwal` features a built-in contrast enforcer. It actively scans the generated background (color0) and main text (color15) using standard WCAG relative luminance math. If the contrast falls below 4.5:1, `rwal` iteratively lightens or darkens the text until the terminal is perfectly readable, eliminating eye strain.
* **Format-agnostic**: Evaluates the literal binary signatures of images rather than blindly trusting file extensions.
* **Light Mode & Saturation Adjustments**: Instantly invert standard dark themes to light themes natively using `-l`, or shift saturation dynamically with `--saturate AMOUNT`.
* **Caching**: Fully hashes image paths + strategies into a lightning-fast LRU file cache. Running the same wallpaper and strategy twice returns instantly.

## Usage

Generate and apply a theme from an image:
```bash
cargo run -- -i ~/Pictures/wallpaper.jpg
```

Change the color generation strategy:
```bash
cargo run -- -i ~/Pictures/wallpaper.jpg --strategy adaptive
```

Switch to the median cut backend:
```bash
cargo run -- -i ~/Pictures/wallpaper.jpg --backend median_cut
```

Skip applying the wallpaper:
```bash
cargo run -- -i ~/Pictures/wallpaper.jpg -n
```

### Full Options

* `-i, --image <PATH>`: Image file or directory to generate colors from.
* `-R, --restore`: Restore the last generated color scheme from cache.
* `-l, --light`: Generate a light color scheme instead of dark.
* `-n, --no-wallpaper`: Skip setting the wallpaper via external tools.
* `-s, --no-sequences`: Skip applying terminal ANSI sequences and rendering templates.
* `--backend <NAME>`: Color extraction backend. `kmeans` or `median_cut`.
* `--accuracy <N>`: K-Means iterations (1-20). Higher is more accurate but slower. Default: 10.
* `--strategy <NAME>`: Generation strategy (`classic`, `adaptive`, `vibrant`, `pastel`, `complementary`, `split_complementary`, `analogous`, `monochromatic`, `triadic`).
* `--saturate <AMOUNT>`: Shift color saturation (-1.0 to 1.0).
* `--alpha <N>`: Transparency value written to JSON exports (0-100).
