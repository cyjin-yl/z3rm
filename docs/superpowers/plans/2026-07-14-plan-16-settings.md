# Plan 16: Settings Schema Rewrite

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Rewrite the settings schema (`settings_json`, `settings_content`) for terminal/mux/extension configuration. Retain settings engine (`settings`, `settings_macros`) and settings UI (`settings_ui`, `settings_profile_selector`).

**Architecture:** Client-side settings (font, theme, keymap, chrome config) watched by client. Server-side settings (scrollback, PTY behavior, keep_alive, quota) watched by server. Both hot-reload on file change.

**Dependencies:** `settings`, `settings_macros`, `settings_json`, `settings_content`, `settings_ui`.

---

### Task 1: Define new settings schema

**Files:**
- Modify: `crates/settings_json/src/settings_json.rs`
- Modify: `crates/settings_content/src/settings_content.rs`

- [ ] **Step 1: Remove all Zed editor/agent settings fields**

Delete: editor settings (tab_size, soft_wrap, etc.), agent settings, AI provider settings, collab settings, vim settings, snippet settings, extension preview settings, LSP settings.

- [ ] **Step 2: Add terminal settings**

```json
{
    "terminal": {
        "font": { "family": "monospace", "size": 14 },
        "shell": { "program": "/bin/zsh", "args": [] },
        "env": {},
        "scrollback_lines": 10000,
        "font_ligatures": true,
        "cursor_style": "block"
    }
}
```

- [ ] **Step 3: Add mux settings**

```json
{
    "mux": {
        "socket_path": null,
        "connect_timeout_ms": 500,
        "keep_alive": true,
        "keep_alive_seconds": null,
        "keymap_profile": "default",
        "tabbar_style": "top",
        "scroll_mode": "per_client"
    }
}
```

- [ ] **Step 4: Add shadow snapshot settings**

```json
{
    "shadow_snapshot": {
        "enabled": true,
        "quota_mode": "per_project",
        "per_project_quota_mb": 500,
        "ignore_patterns": [],
        "binary_detection": true,
        "debounce_ms": 500,
        "frequency_circuit_breaker_k": 10,
        "git_commit_hook": "clear"
    }
}
```

- [ ] **Step 5: Add extension settings**

```json
{
    "extensions": {
        "directory": "~/.config/z3rm/extensions",
        "auto_sync_to_remote": true
    }
}
```

- [ ] **Step 6: Add remote connection settings**

```json
{
    "remote": {
        "remote_server_path": null,
        "auto_install": true
    }
}
```

---

### Task 2: Implement settings hot reload

- [ ] **Step 1: Client-side settings file watch**

Client watches `~/.config/z3rm/settings.json`. On change → reload → apply to: font/theme/keymap/chrome/tabbar style.

- [ ] **Step 2: Server-side settings file watch**

mux_server watches same file (local daemon) or its own file (remote daemon). On change → apply to: scrollback limit, keep_alive, PTY behavior, quota config.

---

### Task 3: Update settings_ui

- [ ] **Step 1: Remove editor/agent settings sections from settings pane**

- [ ] **Step 2: Add terminal/mux/extension/remote settings sections**

---

### Task 4: Tests + Commit

- [ ] **Step 1: Settings schema validation test**

- [ ] **Step 2: Hot reload test**

- [ ] **Step 3: Commit**

```bash
git add crates/settings_json crates/settings_content crates/settings_ui
git commit -m "Rewrite settings schema for terminal/mux/extension/remote"
```
