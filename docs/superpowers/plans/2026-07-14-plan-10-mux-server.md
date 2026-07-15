# Plan 10: mux_server — Headless Daemon

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Create the `mux_server` crate — the headless daemon that owns PTYs, alacritty terminal emulators, layout tree, session state, and grid diff ring. This is the server-canonical authority for all terminal state.

**Architecture:** mux_server is a standalone binary (`z3rm-server`). It binds a local socket (Unix domain socket / named pipe via `interprocess`), accepts connections from GUI clients, and serves mux_protocol messages. It runs a tokio multi-threaded runtime.

**Dependencies:** `mux_protocol`, `terminal` (alacritty), `gpui_tokio`, `db` (SQLite), `interprocess`, `tokio`.

---

### Task 1: Create crate skeleton + binary

**Files:**
- Create: `crates/mux_server/Cargo.toml`
- Create: `crates/mux_server/src/mux_server.rs`
- Create: `crates/mux_server/src/main.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "mux_server"
version = "0.1.0"
edition = "2024"
publish = false
license = "Apache-2.0"

[lib]
path = "src/mux_server.rs"

[[bin]]
name = "z3rm-server"
path = "src/main.rs"

[dependencies]
mux_protocol = { workspace = true }
terminal = { workspace = true }
gpui_tokio = { workspace = true }
db = { workspace = true }
interprocess = "2"
tokio = { workspace = true, features = ["full"] }
prost = { workspace = true }
anyhow = { workspace = true }
parking_lot = { workspace = true }
tracing = { workspace = true }
```

- [ ] **Step 2: Create main.rs — daemon entry point**

```rust
// Entry point for the z3rm-server daemon binary.
// Binds local socket, accepts connections, serves mux protocol.

use anyhow::Result;

fn main() -> Result<()> {
    z3rm_mux_server::run()
}
```

- [ ] **Step 3: Create lib.rs skeleton with run() function**

```rust
use anyhow::Result;
use std::path::PathBuf;

mod session;
mod pane;
mod layout;
mod grid_sync;
mod connection;
mod persistence;

pub struct Server {
    // Session registry
    sessions: parking_lot::RwLock<Vec<session::Session>>,
    // Local socket listener
    listener: interprocess::local_socket::Listener,
    // Socket path (for cleanup on exit)
    socket_path: PathBuf,
}

/// Entry point. Binds socket, accepts connections, runs event loop.
pub fn run() -> Result<()> {
    // 1. Determine socket path
    let socket_path = default_socket_path()?;
    
    // 2. Bind listener (0600 permissions)
    let listener = bind_socket(&socket_path)?;
    
    // 3. Create server instance
    let server = Server::new(listener, socket_path);
    
    // 4. Run tokio runtime + accept loop
    server::run_accept_loop(server)
}

fn default_socket_path() -> Result<PathBuf> {
    // $XDG_RUNTIME_DIR/z3rm/mux.sock on Unix
    // \\.\pipe\z3rm-mux on Windows
    // See §16.1 for socket path strategy
    todo!("implement socket path resolution")
}

fn bind_socket(path: &PathBuf) -> Result<interprocess::local_socket::Listener> {
    // Set 0600 permissions on Unix before bind
    todo!("implement socket binding with permissions")
}
```

- [ ] **Step 4: Add to workspace Cargo.toml**

- [ ] **Step 5: Verify compilation**

Run: `cargo check -p mux_server`
Expected: PASS (with `#[z3rm_todo]` marks on `todo!()` sites, or with `--features z3rm-migration`)

---

### Task 2: Session module

**Files:**
- Create: `crates/mux_server/src/session.rs`

- [ ] **Step 1: Implement Session struct**

```rust
use crate::pane::Pane;
use crate::layout::LayoutTree;
use std::collections::HashMap;
use parking_lot::RwLock;

pub struct Session {
    pub id: String,
    pub name: String,
    pub cwd: String,
    pub tabs: HashMap<String, Tab>,
    pub layout: LayoutTree,
    pub focused_pane: Option<String>,
    pub attached_clients: Vec<AttachedClient>,
}

pub struct Tab {
    pub id: String,
    pub title: String,
    pub pane_ids: Vec<String>,
}

pub struct AttachedClient {
    pub client_id: String,
    pub mode: AttachMode,
}

pub enum AttachMode {
    Shared,
    ReadOnly,
}
```

- [ ] **Step 2: Implement session lifecycle methods**

Create session, list sessions, kill session, rename session. Each method mutates the sessions registry.

---

### Task 3: Pane module (PTY + alacritty emulator)

**Files:**
- Create: `crates/mux_server/src/pane.rs`

- [ ] **Step 1: Implement Pane struct wrapping alacritty Terminal**

```rust
use terminal::Terminal;
use std::sync::Arc;
use parking_lot::Mutex;

pub struct Pane {
    pub id: String,
    pub cwd: String,
    pub title: RwLock<String>,
    pub command: Option<String>,
    pub terminal: Mutex<Terminal>,  // alacritty terminal instance
    pub generation: AtomicU64,       // grid generation counter
    pub grid_diff_ring: RwLock<GridDiffRing>,  // bounded ring of recent diffs
    pub alive: AtomicBool,
}

pub struct GridDiffRing {
    entries: VecDeque<(Generation, GridDiff)>,
    capacity: usize,  // default 64
}
```

- [ ] **Step 2: Implement PTY spawn + read loop**

Spawn shell via `portable_pty` or alacritty's PTY system. Read PTY output in a background task, feed to alacritty Terminal, compute row-level diff from dirty_lines, bump generation, push diff to ring.

- [ ] **Step 3: Implement grid diff computation**

After feeding PTY bytes to the alacritty Terminal, extract `dirty_lines`. For each dirty line, read the current row from the terminal grid. Build `GridDiff { rows: Vec<RowChange> }`. Bump generation counter.

- [ ] **Step 4: Implement `fetch_grid_update(since_generation)`**

Walk the diff ring from `since_generation` to current. If all intermediate diffs are in ring → return merged Diff. If `since_generation` is older than ring's oldest entry → return FullSnapshot.

---

### Task 4: Layout module (split tree + geometry)

**Files:**
- Create: `crates/mux_server/src/layout.rs`

- [ ] **Step 1: Implement LayoutTree migrated from workspace pane_group**

This is the geometry calculation logic extracted from `crates/workspace/src/pane_group.rs`. Split direction, size ratios, resize math — all server-side now.

- [ ] **Step 2: Implement layout serialization (tmux-style checksummed format)**

From §3.7: absolute cell counts with checksum.

---

### Task 5: Connection handler (mux_protocol message dispatch)

**Files:**
- Create: `crates/mux_server/src/connection.rs`

- [ ] **Step 1: Implement framed binary reader/writer**

Read length-prefixed prost Envelope from socket. Dispatch Request bodies to handlers. Send Response/Notification back.

- [ ] **Step 2: Implement request dispatch table**

Map each `Request.body` variant to a handler function. Each handler accesses the Server state, performs the operation, and returns a Response.

- [ ] **Step 3: Implement notification fan-out**

When session state changes (pane added, layout changed, pane dirty), push Notification to all attached clients on that session.

---

### Task 6: Persistence (SQLite layout metadata)

**Files:**
- Create: `crates/mux_server/src/persistence.rs`

- [ ] **Step 1: Implement SQLite schema**

```sql
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    cwd TEXT NOT NULL,
    layout_snapshot TEXT,  -- serialized layout tree
    last_snapshot_timestamp INTEGER NOT NULL
);
```

- [ ] **Step 2: Implement periodic snapshot (~10s)**

Background task: every 10s, snapshot all sessions' layout metadata to SQLite. Grid content is NOT persisted.

- [ ] **Step 3: Implement crash recovery**

On startup: query SQLite for sessions. If non-clean-exit sessions found, offer restoration (shells only, no auto-rerun).

---

### Task 7: Idle behavior (passive daemon)

- [ ] **Step 1: Implement idle mode**

When no panes exist across all sessions: minimal CPU (no polling, no timers except the SQLite snapshot timer). Daemon stays alive (keep_alive = true default).

- [ ] **Step 2: Implement `z3rm-server status` CLI subcommand**

Print: uptime, session count, pane count, memory usage.

---

### Task 8: Adaptive output coalescing

- [ ] **Step 1: Implement three-mode coalescing**

Interactive (0ms), Normal (2ms), High-throughput (8-16ms). Detect mode from keyboard activity and PTY output rate.

- [ ] **Step 2: Implement DEC-2026 BSU/ESU tracking**

Server emulator parses DEC-2026 markers. Defer generation bump until ESU. Unpaired BSU timeout: 100ms → force flush.

---

### Task 9: Process keepalive

- [ ] **Step 1: Verify PTY survives client disconnect**

Client disconnect → server detects socket EOF → session marked detached → PTYs continue running. No SIGHUP sent.

- [ ] **Step 2: Implement `z3rm-server kill` and `kill --session <id>`**

Graceful shutdown: send SIGHUP to all PTY children, wait for exit, clean up socket file, exit.

---

### Task 10: Unit tests

- [ ] **Step 1: Write unit tests for each module**

- Grid diff ring: push/overflow/full snapshot
- Layout tree: split/merge/resize/serialize
- Generation counter: increment/query/ring miss
- Session lifecycle: create/list/kill/attach/detach

- [ ] **Step 2: Run tests**

Run: `cargo test -p mux_server`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add crates/mux_server Cargo.toml
git commit -m "Add mux_server daemon: PTY management, alacritty emulator, layout engine, session persistence, grid diff ring"
```
