//! Unified hook adapter for AI CLI tools.
//!
//! Provides the [`ToolAdapter`] trait and implementations for registering
//! agent-hand hook scripts into various AI coding assistants (Claude Code,
//! Cursor, Copilot/Codex, Windsurf, Kiro, OpenCode, Gemini CLI).

mod detection;

mod claude;
mod codex;
mod cursor;
mod gemini;
mod kiro;
mod opencode;
mod windsurf;

pub use claude::ClaudeAdapter;
pub use codex::CodexAdapter;
pub use cursor::CursorAdapter;
pub use gemini::GeminiAdapter;
pub use kiro::KiroAdapter;
pub use opencode::OpenCodeAdapter;
pub use windsurf::WindsurfAdapter;

use std::path::{Path, PathBuf};

/// Errors from tool adapter operations.
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("TOML deserialize error: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("TOML serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("Config error: {0}")]
    Config(String),
    #[error("Home directory not found")]
    NoHomeDir,
}

pub type Result<T> = std::result::Result<T, AdapterError>;

/// Status of an AI CLI tool on this system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatus {
    /// Tool is not installed on this system.
    NotInstalled,
    /// Tool is installed but hooks are not registered.
    Detected,
    /// Tool is installed and hooks are registered.
    HooksRegistered,
}

impl ToolStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::NotInstalled => "\u{2717}", // ✗
            Self::Detected => "\u{25cf}",     // ●
            Self::HooksRegistered => "\u{2713}", // ✓
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::NotInstalled => "not found",
            Self::Detected => "detected",
            Self::HooksRegistered => "registered",
        }
    }
}

/// Adapter for an AI CLI tool's hook system.
///
/// Each implementation knows how to detect whether a tool is installed,
/// check if agent-hand hooks are registered, and register/unregister them.
pub trait ToolAdapter: Send + Sync {
    /// Internal identifier (e.g. "claude", "cursor").
    fn name(&self) -> &str;

    /// Human-readable display name (e.g. "Claude Code", "Cursor").
    fn display_name(&self) -> &str;

    /// Check if this tool is installed on the system.
    fn is_installed(&self) -> bool;

    /// Check if agent-hand hooks are registered in this tool's config.
    fn hooks_registered(&self) -> bool;

    /// Register agent-hand hooks into this tool's config.
    ///
    /// `hook_cmd` is the absolute path to the hook command (typically `agent-hand-bridge`).
    fn register_hooks(&self, hook_cmd: &Path) -> Result<()>;

    /// Remove agent-hand hooks from this tool's config.
    fn unregister_hooks(&self) -> Result<()>;

    /// Path to this tool's configuration file (for display purposes).
    fn config_path(&self) -> Option<PathBuf>;

    /// Event types this tool supports.
    fn supported_events(&self) -> &[&str];

    /// Get the current status of this tool.
    fn status(&self) -> ToolStatus {
        if !self.is_installed() {
            ToolStatus::NotInstalled
        } else if self.hooks_registered() {
            ToolStatus::HooksRegistered
        } else {
            ToolStatus::Detected
        }
    }
}

/// Information about a tool adapter's status, for UI display.
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub display_name: String,
    pub status: ToolStatus,
    pub config_path: Option<PathBuf>,
}

/// Return all known tool adapters.
pub fn all_adapters() -> Vec<Box<dyn ToolAdapter>> {
    vec![
        Box::new(ClaudeAdapter::new()),
        Box::new(CursorAdapter::new()),
        Box::new(CodexAdapter::new()),
        Box::new(WindsurfAdapter::new()),
        Box::new(KiroAdapter::new()),
        Box::new(OpenCodeAdapter::new()),
        Box::new(GeminiAdapter::new()),
    ]
}

/// Detect all tools and return their status info.
pub fn detect_all() -> Vec<ToolInfo> {
    all_adapters()
        .iter()
        .map(|a| ToolInfo {
            name: a.name().to_string(),
            display_name: a.display_name().to_string(),
            status: a.status(),
            config_path: a.config_path(),
        })
        .collect()
}

/// Auto-register hooks for all detected (but unregistered) tools.
///
/// Returns the names of tools that were successfully registered.
pub fn auto_register_all(hook_cmd: &Path) -> Vec<String> {
    let mut registered = Vec::new();
    for adapter in all_adapters() {
        if adapter.is_installed() && !adapter.hooks_registered() {
            if adapter.register_hooks(hook_cmd).is_ok() {
                registered.push(adapter.display_name().to_string());
            }
        }
    }
    registered
}

/// Get a specific adapter by name.
pub fn get_adapter(name: &str) -> Option<Box<dyn ToolAdapter>> {
    all_adapters().into_iter().find(|a| a.name() == name)
}
