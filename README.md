# rwal — Fast, Accessible Color Palettes

`rwal` is a Rust-based tool that generates terminal color palettes from images. It follows the Unix philosophy: it generates palettes and reloads your terminal, everything else is opt-in.

## Key Features

- **Blazing Fast**: Extracted via `kmeans` or `median_cut`.
- **Guaranteed Contrast**: Every color is mathematically forced to meet **WCAG 4.5:1** contrast against the background. No more unreadable text.
- **Cross-Platform**: Native wallpaper support for **Linux (Sway, Hyprland, i3/X11)**, **macOS**, and **Windows**.
- **User-Driven Templates**: Load your own templates directly from `~/.config/rwal/templates/`.
- **Zero Bloat**: No bundled templates, no forced side effects.

## Usage

Generate and apply a theme (terminal colors only):
```bash
rwal -i ~/walls/mountain.jpg
```

Generate colors **and** set the wallpaper:
```bash
rwal -i ~/walls/mountain.jpg -w
```

Restore the last scheme (for startup scripts):
```bash
rwal -R     # re-apply terminal colors
rwal -R -w  # re-apply terminal colors + wallpaper
```


## Supported Modes

- `classic`: The standard Pywal-style generation logic.
- `adaptive`: Lightens/saturates accents based on image luminance.
- `vibrant`: Focuses on the most distinct accent colors.
- `pastel`: Desaturates and lightens for a softer look.

## Full Options

| `-i, --image <PATH>` | Image file or directory to process |
| `-w, --wallpaper` | **Opt-in**: Apply the wallpaper using the best-detected backend |
| `-R, --restore` | Restore the last generated scheme from `colors.json` |
| `-r, --render` | **Render**: Replace placeholders in templates and save to `~/.cache/rwal/` |
| `-m, --map [<APP>]`| **Map**: Symlink cached templates to their final destinations (defined in `config-map.toml`) |
| `-l, --light` | Generate a light color scheme |
| `-p, --preview` | Show a visual preview of the current palette |
| `-d, --debug` | Show debug info for `config-map.toml` |
| `-q, --quiet` | Suppress all output |
| `--mode <NAME>` | Generation mode (`classic`, `adaptive`, `vibrant`, `pastel`) |
| `--backend <NAME>` | Extraction engine (`kmeans` or `median_cut`) |

## Templates & Mapping

`rwal` uses a two-step process for styling external applications: **Rendering** and **Mapping**.

### 1. Rendering (`-r, --render`)
Place your configuration templates in `~/.config/rwal/templates/`. These files can contain placeholders like `{primary}`, `{background}`, `{color4}`, etc.

When you run `rwal -r`, it:
1. Reads every file in the templates folder.
2. Replaces all placeholders with the current theme colors.
3. Writes the finished files to `~/.cache/rwal/`.

### 2. Mapping (`-m, --map`)
To link these cached files to your applications, use `~/.config/rwal/config-map.toml`.

```toml
[btop]
template = "btop.theme"
output   = "~/.config/btop/theme/rwal.theme"

[rofi]
template = "colors.rasi"
output   = "~/.config/rofi/colors.rasi"
```

When you run `rwal -m`, it creates symlinks from the `output` path to the cached file in `~/.cache/rwal/`.

### Evaluation: `-r` vs `-m`
- **`-r` (Render)**: This is the **data processing** step. It creates the actual "content" of your themes. You generally run this whenever you change wallpapers or themes.
- **`-m` (Map)**: This is the **filesystem** step. It ensures your applications are pointing to the right files. You only *need* to run this once to set up the links, or if you add new entries to `config-map.toml`.

**Pro Tip**: Run `rwal -rm` to do both at once!

## Template Tokens

| Token | Description |
|---|---|
| `{background}` | Main background color (hex) |
| `{surface}`    | Slightly lighter background (hex) |
| `{foreground}` | Main text color (hex) |
| `{primary}`    | Main accent color (typically color1) |
| `{error}`, `{success}` | State colors |
| `{color0}` ... `{color15}` | Raw palette slots |
| `{wallpaper}`  | Path to the current wallpaper |
| `{alpha}`      | Opacity value (0-100) |
