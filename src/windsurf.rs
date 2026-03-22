//! Windsurf adapter.
//!
//! Registers hooks in `~/.codeium/windsurf/hooks.json`.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::detection;
use crate::{AdapterError, Result, ToolAdapter};

const HOOKS_REL: &str = ".codeium/windsurf/hooks.json";

const EVENTS: &[&str] = &[
    "post_cascade_response",
    "pre_user_prompt",
];

pub struct WindsurfAdapter;

impl WindsurfAdapter {
    pub fn new() -> Self {
        Self
    }

    fn hooks_path(&self) -> Option<PathBuf> {
        detection::home_path(HOOKS_REL)
    }
}

impl ToolAdapter for WindsurfAdapter {
    fn name(&self) -> &str {
        "windsurf"
    }

    fn display_name(&self) -> &str {
        "Windsurf"
    }

    fn is_installed(&self) -> bool {
        detection::home_dir_exists(".codeium/windsurf") || detection::command_exists("windsurf")
    }

    fn hooks_registered(&self) -> bool {
        let Some(path) = self.hooks_path() else {
            return false;
        };
        let Ok(content) = std::fs::read_to_string(&path) else {
            return false;
        };
        content.contains("hook_event_bridge") || content.contains("agent-hand-bridge")
    }

    fn register_hooks(&self, hook_cmd: &Path) -> Result<()> {
        let path = self.hooks_path().ok_or(AdapterError::NoHomeDir)?;
        detection::ensure_parent_dir(&path)?;

        let mut root: Value = if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            serde_json::from_str(&content).unwrap_or_else(|_| json!({}))
        } else {
            json!({})
        };

        let obj = root
            .as_object_mut()
            .ok_or_else(|| AdapterError::Config("Invalid hooks.json".into()))?;

        let cmd = hook_cmd.to_string_lossy().to_string();

        for event in EVENTS {
            let arr = obj.entry(*event).or_insert_with(|| json!([]));
            if !arr.is_array() {
                *arr = json!([]);
            }
            let hooks = arr.as_array_mut().unwrap();

            let already = hooks.iter().any(|h| {
                h.get("command")
                    .and_then(|c| c.as_str())
                    .is_some_and(|c| c == cmd)
            });

            if !already {
                hooks.push(json!({ "command": cmd }));
            }
        }

        detection::backup_file(&path);
        let output = serde_json::to_string_pretty(&root)?;
        std::fs::write(&path, output)?;
        Ok(())
    }

    fn unregister_hooks(&self) -> Result<()> {
        let path = self.hooks_path().ok_or(AdapterError::NoHomeDir)?;
        if !path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&path)?;
        let mut root: Value = serde_json::from_str(&content)?;

        let Some(obj) = root.as_object_mut() else {
            return Ok(());
        };

        let mut modified = false;
        for event in EVENTS {
            if let Some(arr) = obj.get_mut(*event).and_then(|v| v.as_array_mut()) {
                let before = arr.len();
                arr.retain(|h| {
                    !h.get("command")
                        .and_then(|c| c.as_str())
                        .is_some_and(|c| c.contains("hook_event_bridge") || c.contains("agent-hand-bridge"))
                });
                if arr.len() != before {
                    modified = true;
                }
            }
        }

        if modified {
            detection::backup_file(&path);
            let output = serde_json::to_string_pretty(&root)?;
            std::fs::write(&path, output)?;
        }
        Ok(())
    }

    fn config_path(&self) -> Option<PathBuf> {
        self.hooks_path()
    }

    fn supported_events(&self) -> &[&str] {
        EVENTS
    }
}
