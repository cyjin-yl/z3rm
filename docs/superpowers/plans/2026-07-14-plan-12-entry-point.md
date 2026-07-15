# Plan 12: z3rm Entry Point

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Create the slimmed `crates/z3rm/src/main.rs` that replaces Zed's 72KB main.rs. Auto-spawns daemon, creates window, connects to daemon via MuxDomain.

**Architecture:** Minimal entry point. No editor/agent/collab initialization. GPUI Application → theme → settings → daemon spawn → window creation → workspace with terminal panes.

**Dependencies:** `mux`, `terminal_view`, `workspace`, `gpui`, `theme`, `settings`, `paths`.

---

### Task 1: Strip Zed's main.rs

**Files:**
- Modify: `crates/z3rm/src/main.rs`

- [ ] **Step 1: Remove all editor/agent/collab/client imports**

Delete imports of: `agent_ui`, `editor`, `collab_ui`, `channel_view`, `client`, `project` (buffer-related), `prompt_store`, `onboarding`, `remote`, `recent_projects` (remote), `cli`, `copilot`, all LLM providers.

- [ ] **Step 2: Remove all editor/agent/collab initialization code**

Delete: AgentPanel setup, Editor registration, collab UI setup, client/cloud initialization, onboarding logic, extension host proxy (node_runtime), language registry LSP setup.

- [ ] **Step 3: Keep GPUI Application setup, theme, settings, paths**

Keep: `Application::new()`, `AppContext`, theme loading, settings store initialization, `paths::APP_NAME`, crash handler init, auto_update (optional).

---

### Task 2: Implement daemon auto-spawn

**Files:**
- Create: `crates/z3rm/src/daemon.rs`
- Modify: `crates/z3rm/src/main.rs`

- [ ] **Step 1: Implement `ensure_daemon_running()`**

```rust
/// Ensures mux_server daemon is running and returns a connected MuxDomain.
/// 1. Try connecting to default socket path.
/// 2. If connection fails (timeout), spawn `z3rm-server --daemonize`.
/// 3. Poll connect with configurable timeout (default 500ms, from settings).
/// 4. Once connected, return MuxDomain.
pub async fn ensure_daemon_running() -> anyhow::Result<MuxDomain> {
    // Try connect first
    match MuxDomain::connect_local().await {
        Ok(domain) => return Ok(domain),
        Err(_) => {}
    }
    // Spawn daemon
    spawn_daemon()?;
    // Wait for socket
    wait_for_socket(socket_path, timeout).await?;
    MuxDomain::connect_local().await
}

fn spawn_daemon() -> anyhow::Result<()> {
    std::process::Command::new("z3rm-server")
        .arg("--daemonize")
        .spawn()?;
    Ok(())
}
```

- [ ] **Step 2: Handle named sessions**

If `--session <name>` CLI arg present, use named socket path. If daemon doesn't exist, spawn with `--session <name>`. If daemon exists, connect and attach/create session.

---

### Task 3: Implement default session creation

**Files:**
- Modify: `crates/z3rm/src/daemon.rs`

- [ ] **Step 1: On first launch (no sessions), create default session**

```rust
let sessions = domain.list_sessions().await?;
if sessions.is_empty() {
    let default_cwd = dirs::home_dir().unwrap_or_default();
    domain.create_session("default", &default_cwd).await?;
}
```

---

### Task 4: Implement window creation

**Files:**
- Modify: `crates/z3rm/src/main.rs`

- [ ] **Step 1: Create GPUI window with workspace**

```rust
cx.open_window(window_options, |window, cx| {
    // Create workspace with MuxDomain
    // Attach to default (or named) session
    // Spawn initial terminal pane
    workspace::Workspace::new(domain, session_id, window, cx)
});
```

- [ ] **Step 2: Wire window close = detach**

Window close handler: call `domain.detach()`. Daemon keeps running (keep_alive=true).

---

### Task 5: Verify smoke test

- [ ] **Step 1: Manual test**

Run: `cargo run -p z3rm`
Expected: window opens, shell prompt appears, typing works, closing window doesn't kill shell.

- [ ] **Step 2: Verify daemon persists**

After closing window, run: `z3rm-server status`
Expected: shows 1 session, panes alive.

- [ ] **Step 3: Verify reattach**

Reopen z3rm. Expected: session is reattached, panes visible.

- [ ] **Step 4: Commit**

```bash
git add crates/z3rm/src/
git commit -m "Slim main.rs: daemon auto-spawn, window creation, session attach"
```
