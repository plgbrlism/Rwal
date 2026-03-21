/*
Pywal's killer feature. On every run:

Build OSC escape sequence string for all 16 colors + fg/bg/cursor
Write to ~/.cache/rwal/sequences (users source this in .zshrc/.bashrc for persistence)
Write to every /dev/pts/N tty file to recolor open terminals live

*/
use std::io::Write;
use std::path::PathBuf;
use crate::colors::types::ColorDict;
use crate::error::{RwalError, warn};
use crate::paths::Paths;

/// Apply color sequences — writes to both:
/// 1. `~/.cache/rwal/sequences` (sourced on login for new terminals)
/// 2. Every `/dev/pts/N` (live recolor of all open terminals)
pub fn apply(paths: &Paths, dict: &ColorDict) -> Result<(), RwalError> {
    let sequences = build_sequences(dict);

    write_sequence_file(paths, &sequences)?;
    
    // Live recolor open terminals (Linux /dev/pts)
    write_to_terminals(&sequences);

    // Recolor current terminal (Universal OSC via stdout)
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

/// Write sequences to every open terminal under `/dev/pts/`.
/// Warns on individual failures but never stops — other terminals
/// should still be recolored even if one fails.
fn write_to_terminals(sequences: &str) {
    let pts_dir = PathBuf::from("/dev/pts");

    if !pts_dir.exists() {
        return;
    }

    let entries = match std::fs::read_dir(&pts_dir) {
        Ok(e)  => e,
        Err(e) => {
            warn(&RwalError::SequenceWriteError(format!(
                "could not read /dev/pts: {e}"
            )));
            return;
        }
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();

        // Skip non-numeric entries (e.g. /dev/pts/ptmx)
        if !is_pts_terminal(&path) {
            continue;
        }

        if let Err(e) = write_to_pts(&path, sequences) {
            warn(&e);
        }
    }
}

fn write_to_pts(path: &PathBuf, sequences: &str) -> Result<(), RwalError> {
    use std::fs::OpenOptions;
    use std::os::unix::fs::OpenOptionsExt;

    let mut options = OpenOptions::new();
    options.write(true).create(false); // don't create new pts files, only open existing ones

    // 0o4000 is O_NONBLOCK on Linux. This prevents rwal from hanging 
    // indefinitely if a background pseudo-terminal is stalled or dead.
    options.custom_flags(0o4000);

    let mut file = options.open(path)

        .map_err(|e| RwalError::SequenceWriteError(format!(
            "could not open {}: {e}", path.display()
        )))?;

    // We don't check the output of write_all too strictly because 
    // WouldBlock errors on a dead TTY are fine to ignore.
    let _ = file.write_all(sequences.as_bytes());

    Ok(())
}

/// Returns true if the path is a numeric pts entry (a real terminal).
fn is_pts_terminal(path: &PathBuf) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.chars().all(|c| c.is_ascii_digit()))
        .unwrap_or(false)
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

    // ── is_pts_terminal ──────────────────────────────────────────────────────

    #[test]
    fn test_is_pts_terminal_numeric() {
        assert!(is_pts_terminal(&PathBuf::from("/dev/pts/0")));
        assert!(is_pts_terminal(&PathBuf::from("/dev/pts/12")));
    }

    #[test]
    fn test_is_pts_terminal_rejects_ptmx() {
        assert!(!is_pts_terminal(&PathBuf::from("/dev/pts/ptmx")));
    }

    #[test]
    fn test_is_pts_terminal_rejects_mixed() {
        assert!(!is_pts_terminal(&PathBuf::from("/dev/pts/1abc")));
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