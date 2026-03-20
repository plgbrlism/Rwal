# Adding New Bundled Templates

> [!NOTE]
> **Bundling is currently disabled by default.** `rwal` prefers loading user templates from `~/.config/rwal/templates/` at runtime. This avoids bloated binaries and keeps the tool focused.

---

## Why use bundled templates?

Bundling is useful if you are building a custom version of `rwal` that you want to distribute with a set of "standard" templates that work out of the box without any user configuration.

---

## How to re-enable bundling

To enable bundling, you must modify `src/export/templates.rs`. 

1. Add the `rust-embed` dependency back to the top of the file:
   ```rust
   use rust_embed::RustEmbed;
   ```

2. Define the `BundledTemplates` struct:
   ```rust
   #[derive(RustEmbed)]
   #[folder = "templates/"]
   #[exclude = "*.md"]
   struct BundledTemplates;
   ```

3. Update `collect_templates()` to include the embedded files:
   ```rust
   fn collect_templates(paths: &Paths) -> Result<HashMap<String, String>, RwalError> {
       let mut map: HashMap<String, String> = HashMap::new();

       // 1. Load bundled templates (embedded at compile time)
       for filename in BundledTemplates::iter() {
           if let Some(file) = BundledTemplates::get(&filename) {
               if let Ok(contents) = std::str::from_utf8(file.data.as_ref()) {
                   map.insert(filename.to_string(), contents.to_string());
               }
           }
       }

       // 2. Overlay user templates (user wins on clash)
       // ... existing user-loading logic ...
       Ok(map)
   }
   ```

---

## Adding a new template to this folder

1. Create your template file in this directory (`templates/`).
   Follow the naming convention:
   - `colors.{filetype}` for generic format templates
   - `colors_{application}.{filetype}` for app-specific templates

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

3. Re-compile with `cargo build`.

---

## User overrides (Always Active)

Regardless of whether bundling is enabled, any file placed in `~/.config/rwal/templates/` with the **same filename** as an embedded or generated file will take priority. This allows users to override any "standard" behavior.