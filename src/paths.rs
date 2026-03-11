use dirs;
use std::path::PathBuf;

/// Returns the path to the cache directory, e.g., ~/.cache/rwal
pub fn cache_dir() -> PathBuf {
    let home_dir = dirs::home_dir().expect("Could not determine home directory");
    home_dir.join(".cache").join("rwal")

}
