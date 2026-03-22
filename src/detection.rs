//! Common detection utilities for AI CLI tools.

use std::path::{Path, PathBuf};

/// Get the user's home directory.
pub fn home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

/// Check if a directory exists under the home directory.
pub fn home_dir_exists(rel_path: &str) -> bool {
    home_dir()
        .map(|h| h.join(rel_path).is_dir())
        .unwrap_or(false)
}

/// Check if a file exists under the home directory.
#[allow(dead_code)]
pub fn home_file_exists(rel_path: &str) -> bool {
    home_dir()
        .map(|h| h.join(rel_path).is_file())
        .unwrap_or(false)
}

/// Get an absolute path relative to home directory.
pub fn home_path(rel_path: &str) -> Option<PathBuf> {
    home_dir().map(|h| h.join(rel_path))
}

/// Check if a CLI command is available on PATH.
pub fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Ensure parent directory exists for a file path.
pub fn ensure_parent_dir(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// Back up a file before modifying it.
pub fn backup_file(path: &Path) {
    if path.exists() {
        let bak = path.with_extension(format!(
            "{}.bak",
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("bak")
        ));
        let _ = std::fs::copy(path, bak);
    }
}
