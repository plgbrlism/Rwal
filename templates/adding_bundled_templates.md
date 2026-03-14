# Adding New Bundled Templates

This directory contains templates shipped with rwal and embedded at compile time
via [`rust-embed`](https://github.com/pyrossh/rust-embed).

---

## How embedding works

In `src/templates.rs`, the `BundledTemplates` struct points at this folder:

```rust
#[derive(Embed)]
#[folder = "templates/"]
#[exclude = "*.md"]
struct BundledTemplates;
```

Every file dropped here (except `.md` files) is embedded into the binary at
`cargo build` time. No other code changes are needed for new templates — the
`collect_templates()` function iterates all embedded files automatically.

---

## Adding a new template

1. Create your template file in this directory.
   Follow the naming convention:
   - `colors.{filetype}` for generic format templates
   - `colors_{application}.{filetype}` for app-specific templates

   Examples:
   ```
   colors.toml
   colors_alacritty.yml
   colors_waybar.css
   colors_dunst.conf
   ```

2. Use only the supported tokens inside your template:

   | Token          | Value                        |
   |----------------|------------------------------|
   | `{color0}`     | Palette color 0 (`#rrggbb`)  |
   | `{color1}`     | Palette color 1 (`#rrggbb`)  |
   | …              | …                            |
   | `{color15}`    | Palette color 15 (`#rrggbb`) |
   | `{background}` | Background (`#rrggbb`)       |
   | `{foreground}` | Foreground (`#rrggbb`)       |
   | `{cursor}`     | Cursor color (`#rrggbb`)     |
   | `{wallpaper}`  | Absolute wallpaper path      |
   | `{alpha}`      | Opacity value (0–100)        |

   Any unrecognized `{token}` is left as-is in the output — no errors.

3. Run `cargo build`. Your template is now embedded and will be rendered
   on every `rwal` run alongside all other templates.

4. Rendered output is written to `~/.cache/rwal/<filename>` — same filename,
   same flat structure.

---

## User overrides

If a user places a file with the **same filename** in `~/.config/rwal/templates/`,
their version takes priority over the bundled one. This is intentional — users
can customize any bundled template without patching rwal itself.

---

## Excluding files from embedding

The `#[exclude = "*.md"]` attribute on `BundledTemplates` means `.md` files
in this directory are never embedded or rendered. Use `.md` freely for
per-template documentation.

If you ever need to exclude other patterns (e.g. `.example` files), add them
in `src/templates.rs`:

```rust
#[derive(Embed)]
#[folder = "templates/"]
#[exclude = "*.md"]
#[exclude = "*.example"]
struct BundledTemplates;
```

---

## Checklist for a new template PR

- [ ] File is named `colors.{ext}` or `colors_{app}.{ext}`
- [ ] Only supported tokens are used
- [ ] Tested locally with `cargo run` and verified output in `~/.cache/rwal/`
- [ ] A short comment at the top of the file explains what app it targets