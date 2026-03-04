# agent-hooks

Unified hook registration for AI coding CLI tools.

One trait, one bridge script — connect any AI CLI tool to your development workflow.

## What it does

AI coding assistants (Claude Code, Cursor, Codex, Windsurf, Kiro, OpenCode, Gemini CLI) each have their own hook/event systems with different config formats and locations. `agent-hooks` provides:

- **Unified `ToolAdapter` trait** — detect, register, and unregister hooks across all tools
- **Auto-detection** — find which AI tools are installed on the system
- **One-call registration** — `auto_register_all()` hooks into every detected tool at once
- **Bridge script** — normalize events from all tools into a common JSONL format

## Supported Tools

| Tool | Config Location | Events |
|------|----------------|--------|
| Claude Code | `~/.claude/settings.json` | Stop, Notification, UserPromptSubmit, SubagentStart, PreCompact |
| Cursor | `~/.cursor/hooks.json` | stop, beforeSubmitPrompt, preToolUse, postToolUse, subagentStart/Stop |
| Codex CLI | `~/.codex/hooks.json` | postToolUse, userPromptSubmitted, errorOccurred |
| Windsurf | `~/.codeium/windsurf/hooks.json` | post_cascade_response, pre_user_prompt |
| Kiro | `~/.kiro/hooks.json` | stop, userPromptSubmit, agentSpawn |
| OpenCode | `~/.config/opencode/hooks.json` | stop, userPromptSubmit |
| Gemini CLI | `~/.gemini/hooks.json` | turn_complete, user_prompt_submit |

## Usage

```rust
use agent_hooks::{ToolAdapter, ToolStatus, detect_all, auto_register_all};
use std::path::Path;

// Detect all installed tools
for info in detect_all() {
    println!("{}: {}", info.display_name, info.status.label());
}
// Output:
//   Claude Code: registered
//   Cursor: detected
//   Codex CLI: not found
//   ...

// Auto-register hooks for all detected tools
let bridge = Path::new("/path/to/hook_event_bridge.sh");
let registered = auto_register_all(bridge);
println!("Registered: {:?}", registered);

// Or work with a specific adapter
use agent_hooks::CursorAdapter;
let cursor = CursorAdapter::new();
if cursor.is_installed() && !cursor.hooks_registered() {
    cursor.register_hooks(bridge).unwrap();
}
```

## The ToolAdapter Trait

```rust
pub trait ToolAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn display_name(&self) -> &str;
    fn is_installed(&self) -> bool;
    fn hooks_registered(&self) -> bool;
    fn register_hooks(&self, bridge_script: &Path) -> Result<()>;
    fn unregister_hooks(&self) -> Result<()>;
    fn config_path(&self) -> Option<PathBuf>;
    fn supported_events(&self) -> &[&str];
}
```

## Adding a New Tool

Implement `ToolAdapter` for your tool and add it to `all_adapters()` in `lib.rs`:

```rust
pub struct MyToolAdapter;

impl ToolAdapter for MyToolAdapter {
    fn name(&self) -> &str { "mytool" }
    fn display_name(&self) -> &str { "My Tool" }
    fn is_installed(&self) -> bool {
        // Check ~/.mytool/ or `which mytool`
    }
    // ...
}
```

## Event Bridge

All tools' events get normalized to a common format via `hook_event_bridge.sh`:

```jsonl
{"tmux_session":"agent-0","kind":{"type":"stop"},"session_id":"abc123","cwd":"/project","ts":1709568000}
{"tmux_session":"agent-0","kind":{"type":"user_prompt_submit"},"session_id":"abc123","cwd":"/project","ts":1709568005}
```

This makes downstream consumers (like [agent-hand](https://github.com/weykon/agent-deck-rs)) tool-agnostic.

## Installation

```toml
[dependencies]
agent-hooks = { git = "https://github.com/weykon/agent-hooks" }
```

## License

MIT
