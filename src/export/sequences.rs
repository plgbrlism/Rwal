/*
Pywal's killer feature. On every run:

Build OSC escape sequence string for all 16 colors + fg/bg/cursor
Write to ~/.cache/rwal/sequences (users source this in .zshrc/.bashrc for persistence)
Write to every /dev/pts/N tty file to recolor open terminals live

*/
use std::io::Write;
use crate::colors::types::ColorDict;
use crate::error::RwalError;
use crate::paths::Paths;

/// Apply color sequences — writes to both:
/// 1. `~/.cache/rwal/sequences` (sourced on login for new terminals)
/// 2. Every `/dev/pts/N` (live recolor of all open terminals)
pub fn apply(paths: &Paths, dict: &ColorDict) -> Result<(), RwalError> {
    let sequences = build_sequences(dict);

    write_sequence_file(paths, &sequences)?;
    
    // Recolor current terminal natively via stdout on macOS
    let _ = std::io::stdout().write_all(sequences.as_bytes());
    let _ = std::io::stdout().flush();

    Ok(())
}


/// Build the full OSC escape sequence string for a ColorDict.
///
/// OSC 4  — sets a numbered terminal color slot (0–15)
/// OSC 10 — sets foreground
/// OSC 11 — sets background
/// OSC 12 — sets cursor
fn build_sequences(dict: &ColorDict) -> String {
    let mut out = String::new();

    // color0..color15
    for (i, color) in dict.colors.iter().enumerate() {
        out.push_str(&format!(
            "\x1b]4;{};{}\x07",
            i,
            color.to_hex()
        ));
    }

    // Special slots
    out.push_str(&format!("\x1b]10;{}\x07", dict.special.foreground.to_hex()));
    out.push_str(&format!("\x1b]11;{}\x07", dict.special.background.to_hex()));
    out.push_str(&format!("\x1b]12;{}\x07", dict.special.cursor.to_hex()));

    out
}

/// Write sequences to `~/.cache/rwal/sequences` for login shell sourcing.
fn write_sequence_file(paths: &Paths, sequences: &str) -> Result<(), RwalError> {
    std::fs::write(&paths.sequences, sequences)
        .map_err(|e| RwalError::SequenceWriteError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use crate::colors::types::{Rgb, Special};

    struct TempDir { path: PathBuf }

    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "rwal_seq_test_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .subsec_nanos()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
        }
        fn path(&self) -> &Path { &self.path }
    }

    impl Drop for TempDir {
        fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.path); }
    }

    fn fake_paths(tmp: &TempDir) -> Paths {
        let p = Paths::from_home(tmp.path().to_path_buf());
        p.ensure_dirs().unwrap();
        p
    }

    fn dummy_dict() -> ColorDict {
        ColorDict {
            wallpaper: PathBuf::from("/tmp/wall.jpg"),
            alpha: 100,
            special: Special {
                background: Rgb::new(10,  10,  10),
                foreground: Rgb::new(240, 240, 240),
                cursor:     Rgb::new(240, 240, 240),
            },
            colors: [Rgb::new(30, 30, 30); 16],
        }
    }

    // ── build_sequences ──────────────────────────────────────────────────────

    #[test]
    fn test_sequences_contain_all_16_color_slots() {
        let dict = dummy_dict();
        let seq  = build_sequences(&dict);
        for i in 0..16 {
            assert!(seq.contains(&format!("\x1b]4;{};", i)),
                "missing OSC sequence for color{i}");
        }
    }

    #[test]
    fn test_sequences_contain_foreground() {
        let dict = dummy_dict();
        let seq  = build_sequences(&dict);
        assert!(seq.contains("\x1b]10;"));
    }

    #[test]
    fn test_sequences_contain_background() {
        let dict = dummy_dict();
        let seq  = build_sequences(&dict);
        assert!(seq.contains("\x1b]11;"));
    }

    #[test]
    fn test_sequences_contain_cursor() {
        let dict = dummy_dict();
        let seq  = build_sequences(&dict);
        assert!(seq.contains("\x1b]12;"));
    }

    #[test]
    fn test_sequences_use_hex_colors() {
        let dict = dummy_dict();
        let seq  = build_sequences(&dict);
        assert!(seq.contains('#'), "sequences should contain hex colors");
    }

    #[test]
    fn test_sequences_use_bell_terminator() {
        let dict = dummy_dict();
        let seq  = build_sequences(&dict);
        // OSC sequences must be terminated with BEL (\x07)
        assert!(seq.contains('\x07'));
    }

    #[test]
    fn test_sequences_color_values_match_dict() {
        let mut dict  = dummy_dict();
        dict.colors[0] = Rgb::new(0xAB, 0xCD, 0xEF);
        let seq = build_sequences(&dict);
        assert!(seq.contains("#abcdef"));
    }

    // ── write_sequence_file ──────────────────────────────────────────────────

    #[test]
    fn test_write_sequence_file_creates_file() {
        let tmp   = TempDir::new();
        let paths = fake_paths(&tmp);
        let dict  = dummy_dict();
        let seq   = build_sequences(&dict);

        write_sequence_file(&paths, &seq).unwrap();
        assert!(paths.sequences.exists());
    }

    #[test]
    fn test_write_sequence_file_content_matches() {
        let tmp   = TempDir::new();
        let paths = fake_paths(&tmp);
        let dict  = dummy_dict();
        let seq   = build_sequences(&dict);

        write_sequence_file(&paths, &seq).unwrap();
        let contents = std::fs::read_to_string(&paths.sequences).unwrap();
        assert_eq!(contents, seq);
    }

    #[test]
    fn test_write_sequence_file_overwrites_existing() {
        let tmp   = TempDir::new();
        let paths = fake_paths(&tmp);

        write_sequence_file(&paths, "old content").unwrap();
        write_sequence_file(&paths, "new content").unwrap();

        let contents = std::fs::read_to_string(&paths.sequences).unwrap();
        assert_eq!(contents, "new content");
    }

    // ── apply ────────────────────────────────────────────────────────────────

    #[test]
    fn test_apply_writes_sequence_file() {
        let tmp   = TempDir::new();
        let paths = fake_paths(&tmp);
        let dict  = dummy_dict();

        apply(&paths, &dict).unwrap();
        assert!(paths.sequences.exists());
    }
}