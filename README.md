# rwal - A Rust Wallpaper and Color Palette Tool

`rwal` is a command-line tool written in Rust that generates a color palette from an image, and can optionally set your wallpaper. It is designed to be fast, simple, and opinionated.

## Current Features

*   **Color Palette Generation**: Extracts a 16-color palette from a given image using k-means clustering.
*   **Opinionated Palette**: Creates a 4-color palette with the roles `background`, `foreground`, `accent`, and `secondary`.
*   **JSON Output**: Outputs the generated 4-color palette to a `colors.json` file.
*   **Wallpaper Setting**: Can set the wallpaper using `feh`, `swaybg`, or `xwallpaper`.

## Build and Run

To build and run `rwal`, you need to have Rust and Cargo installed.

1.  Clone the repository:
    ```sh
    git clone <repository_url>
    cd rwal
    ```

2.  Build the project:
    ```sh
    cargo build
    ```

3.  Run the application:
    ```sh
    cargo run -- <command>
    ```

## Usage

The main command is `generate`, which takes an image path and several options.

```sh
cargo run -- generate <path_to_image> [OPTIONS]
```

### Options

*   `--mode <dark|light>`: Set the theme mode (currently not implemented).
*   `--contrast <low|medium|high>`: Set the contrast level (currently not implemented).
*   `--backend <feh|swaybg|xwallpaper|none>`: The wallpaper backend to use. Defaults to `none`.
*   `--apply`: Write the generated palette to `colors.json`. If not provided, the JSON is printed to stdout.

### Example

```sh
cargo run -- generate ~/Pictures/my-wallpaper.png --apply --backend feh
```
This command will:
1.  Generate a color palette from `~/Pictures/my-wallpaper.png`.
2.  Write the palette to `colors.json` in the project directory.
3.  Set the wallpaper using `feh`.

## Current State and Future Improvements

This is a working prototype of `rwal`. The core functionality is in place, but there are some limitations and areas for improvement.

### Palette Generation

The current implementation of the palette generator is very basic. It takes the first 4 colors from the 16 colors extracted by the color extractor.

A more sophisticated approach would be to:
*   Sort the extracted colors by luminance.
*   Select the background and foreground colors based on contrast rules for accessibility.
*   Select the accent and secondary colors based on saturation and hue.

This would result in a much more pleasing and usable color palette. The `palette_generator.rs` module is the place to implement this logic.
