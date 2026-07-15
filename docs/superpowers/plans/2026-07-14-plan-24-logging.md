# Plan 24: Logging & Diagnostics

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Implement file logging for mux_server daemon, `z3rm-server status` CLI, and GPUI notifications for daemon problems. All Day 0.

**Dependencies:** `zlog`, `zlog_settings`, `mux_server`, `gpui`, `notifications`.

---

### Task 1: File logging

**Files:**
- Modify: `crates/mux_server/src/main.rs`

- [ ] **Step 1: Configure zlog for daemon**

```rust
fn setup_logging() -> Result<()> {
    let log_dir = paths::log_dir();  // ~/.local/share/z3rm/logs/ or platform equivalent
    std::fs::create_dir_all(&log_dir)?;
    
    zlog::init_file_logger(&log_dir.join("mux-server.log"), zlog::LevelFilter::Info)?;
    Ok(())
}
```

- [ ] **Step 2: Implement log rotation**

Rotate at 10MB, keep 3 rotations.

- [ ] **Step 3: Log key events**

- Session create/kill
- Pane spawn/exit (with exit code)
- Client attach/detach (with client ID)
- Grid sync errors
- Extension load/crash
- Socket bind/connect failures

---

### Task 2: Status CLI

**Files:**
- Modify: `crates/mux_server/src/main.rs`

- [ ] **Step 1: Implement `z3rm-server status` subcommand**

```
$ z3rm-server status
z3rm-server v0.1.0
Uptime: 2h 34m
Sessions: 2 (1 attached)
Panes: 5
Memory: 47 MB
Socket: /run/user/1000/z3rm/mux.sock
```

- [ ] **Step 2: Implement `z3rm-server kill` and `kill --session <id>`**

Graceful shutdown: SIGHUP all PTY children, wait, clean socket, exit.

---

### Task 3: GPUI notifications for daemon problems

**Files:**
- Modify: `crates/z3rm/src/daemon.rs`

- [ ] **Step 1: Daemon connection loss notification**

If MuxDomain connection drops unexpectedly → show toast: "Connection to mux_server lost. Reconnecting..."

- [ ] **Step 2: Daemon error notification**

If server sends an error notification → show as GPUI error toast.

- [ ] **Step 3: Daemon idle notification**

Optional: if daemon has been idle (no panes) for > 1 hour → informational toast: "mux_server is idle. It will stay running until you kill it."

---

### Task 4: Tests + Commit

- [ ] **Step 1: Log file creation + rotation test**

- [ ] **Step 2: Status output format test**

- [ ] **Step 3: Commit**

```bash
git add crates/mux_server/src/main.rs crates/z3rm/src/daemon.rs
git commit -m "Add daemon logging, status CLI, GPUI notifications"
```
