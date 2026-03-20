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

Load a saved theme:
```bash
rwal --theme catppuccin
```

## Supported Modes

- `classic`: The standard Pywal-style generation logic.
- `adaptive`: Lightens/saturates accents based on image luminance.
- `vibrant`: Focuses on the most distinct accent colors.
- `pastel`: Desaturates and lightens for a softer look.

## Full Options

| Flag | Description |
|---|---|
| `-i, --image <PATH>` | Image file or directory to process |
| `-w, --wallpaper` | **Opt-in**: Apply the wallpaper using the best-detected backend |
| `-R, --restore` | Restore the last generated scheme from `colors.json` |
| `-l, --light` | Generate a light color scheme |
| `-q, --quiet` | Suppress all output |
| `--mode <NAME>` | Generation mode (`classic`, `adaptive`, `vibrant`, `pastel`) |
| `--theme <NAME>` | Load a saved `.json` theme from `~/.config/rwal/themes/` |
| `--backend <NAME>` | Extraction engine (`kmeans` or `median_cut`) |
| `--list-themes` | Show all available themes |

## Templates

Place any file in `~/.config/rwal/templates/` to have it rendered to `~/.cache/rwal/` on every run.
Tokens like `{color0}`, `{background}`, `{foreground}`, `{wallpaper}`, and `{alpha}` are supported.
