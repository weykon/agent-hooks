//! Claude Code adapter.
//!
//! Registers hooks in `~/.claude/settings.json` under the `hooks` key.
//! Claude Code uses PascalCase event names: Stop, UserPromptSubmit, Notification, etc.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::detection;
use crate::{AdapterError, Result, ToolAdapter};

const SETTINGS_REL: &str = ".claude/settings.json";

/// Claude Code hook event types.
const EVENTS: &[&str] = &[
    "Stop",
    "Notification",
    "UserPromptSubmit",
    "SubagentStart",
    "PreCompact",
    "PreToolUse",
    "PostToolUse",
];

pub struct ClaudeAdapter;

impl ClaudeAdapter {
    pub fn new() -> Self {
        Self
    }

    fn settings_path(&self) -> Option<PathBuf> {
        detection::home_path(SETTINGS_REL)
    }

    /// Register a hook command for a single event type.
    fn register_event(&self, settings_path: &Path, bridge_cmd: &str, event: &str) -> Result<()> {
        detection::ensure_parent_dir(settings_path)?;

        let mut root: Value = if settings_path.exists() {
            let content = std::fs::read_to_string(settings_path)?;
            serde_json::from_str(&content)?
        } else {
            json!({})
        };

        let obj = root
            .as_object_mut()
            .ok_or_else(|| AdapterError::Config("Invalid root JSON".into()))?;

        let hooks = obj.entry("hooks").or_insert_with(|| json!({}));
        if !hooks.is_object() {
            *hooks = json!({});
        }
        let hooks_obj = hooks
            .as_object_mut()
            .ok_or_else(|| AdapterError::Config("Invalid hooks format".into()))?;

        let event_hooks = hooks_obj.entry(event).or_insert_with(|| json!([]));
        if !event_hooks.is_array() {
            *event_hooks = json!([]);
        }
        let arr = event_hooks
            .as_array_mut()
            .ok_or_else(|| AdapterError::Config("Invalid event hooks format".into()))?;

        // Check if already registered
        let already = arr.iter().any(|item| {
            item.get("hooks")
                .and_then(|h| h.as_array())
                .map(|a| {
                    a.iter().any(|h| {
                        h.get("command")
                            .and_then(|c| c.as_str())
                            .is_some_and(|c| c == bridge_cmd)
                    })
                })
                .unwrap_or(false)
        });

        if !already {
            arr.push(json!({
                "matcher": "",
                "hooks": [{
                    "type": "command",
                    "command": bridge_cmd
                }]
            }));

            detection::backup_file(settings_path);
            let output = serde_json::to_string_pretty(&root)?;
            std::fs::write(settings_path, output)?;
        }

        Ok(())
    }

    /// Remove agent-hand hook entries from a single event type.
    fn unregister_event(
        &self,
        settings_path: &Path,
        bridge_cmd: &str,
        event: &str,
    ) -> Result<()> {
        if !settings_path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(settings_path)?;
        let mut root: Value = serde_json::from_str(&content)?;

        let modified = if let Some(hooks) = root
            .as_object_mut()
            .and_then(|o| o.get_mut("hooks"))
            .and_then(|h| h.as_object_mut())
        {
            if let Some(arr) = hooks.get_mut(event).and_then(|v| v.as_array_mut()) {
                let before = arr.len();
                arr.retain(|item| {
                    !item
                        .get("hooks")
                        .and_then(|h| h.as_array())
                        .map(|a| {
                            a.iter().any(|h| {
                                h.get("command")
                                    .and_then(|c| c.as_str())
                                    .is_some_and(|c| c == bridge_cmd)
                            })
                        })
                        .unwrap_or(false)
                });
                arr.len() != before
            } else {
                false
            }
        } else {
            false
        };

        if modified {
            detection::backup_file(settings_path);
            let output = serde_json::to_string_pretty(&root)?;
            std::fs::write(settings_path, output)?;
        }

        Ok(())
    }

    /// Check if a specific bridge command is registered for any event.
    fn has_bridge_hook(&self, settings_path: &Path) -> bool {
        let Ok(content) = std::fs::read_to_string(settings_path) else {
            return false;
        };
        let Ok(root) = serde_json::from_str::<Value>(&content) else {
            return false;
        };

        let Some(hooks) = root.get("hooks").and_then(|h| h.as_object()) else {
            return false;
        };

        // Check if any event has a hook command containing our bridge identifiers
        for event in EVENTS {
            if let Some(arr) = hooks.get(*event).and_then(|v| v.as_array()) {
                for item in arr {
                    if let Some(inner) = item.get("hooks").and_then(|h| h.as_array()) {
                        for h in inner {
                            if let Some(cmd) = h.get("command").and_then(|c| c.as_str()) {
                                if cmd.contains("hook_event_bridge") || cmd.contains("agent-hand-bridge") {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }

        false
    }
}

impl ToolAdapter for ClaudeAdapter {
    fn name(&self) -> &str {
        "claude"
    }

    fn display_name(&self) -> &str {
        "Claude Code"
    }

    fn is_installed(&self) -> bool {
        detection::home_dir_exists(".claude") || detection::command_exists("claude")
    }

    fn hooks_registered(&self) -> bool {
        self.settings_path()
            .map(|p| self.has_bridge_hook(&p))
            .unwrap_or(false)
    }

    fn register_hooks(&self, hook_cmd: &Path) -> Result<()> {
        let settings = self
            .settings_path()
            .ok_or(AdapterError::NoHomeDir)?;
        let cmd = hook_cmd.to_string_lossy().to_string();

        for event in EVENTS {
            self.register_event(&settings, &cmd, event)?;
        }
        Ok(())
    }

    fn unregister_hooks(&self) -> Result<()> {
        let settings = self
            .settings_path()
            .ok_or(AdapterError::NoHomeDir)?;

        // Remove both legacy shell script and new binary hook commands
        let cmds: Vec<String> = [
            ".agent-hand/hooks/hook_event_bridge.sh",
            ".agent-hand/bin/agent-hand-bridge",
        ]
        .iter()
        .filter_map(|rel| detection::home_path(rel).map(|p| p.to_string_lossy().to_string()))
        .collect();

        for cmd in &cmds {
            for event in EVENTS {
                self.unregister_event(&settings, cmd, event)?;
            }
        }
        Ok(())
    }

    fn config_path(&self) -> Option<PathBuf> {
        self.settings_path()
    }

    fn supported_events(&self) -> &[&str] {
        EVENTS
    }
}
