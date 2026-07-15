# Plan 14: quickjs_runtime + Extension System

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Implement QuickJS runtime crate, rewrite extension host to use QuickJS instead of node_runtime, implement extension API for terminal, implement native chrome baseline + extension chrome replacement, implement extension sync to remote server.

**Architecture:** QuickJS runs on a dedicated OS thread per extension host instance. Resource limits (CPU fuel, memory, IO rate). Extensions declare runtime side (server/client/both) in manifest. Native GPUI chrome is primary Day 0 baseline.

**Dependencies:** `rquickjs`, `gpui`, `mux`, `extension`, `extension_host`, `mux_protocol`.

---

### Task 1: Create quickjs_runtime crate

**Files:**
- Create: `crates/quickjs_runtime/Cargo.toml`
- Create: `crates/quickjs_runtime/src/quickjs_runtime.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "quickjs_runtime"
version = "0.1.0"
edition = "2024"
publish = false
license = "Apache-2.0"

[lib]
path = "src/quickjs_runtime.rs"

[dependencies]
rquickjs = "0.6"
parking_lot = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
```

- [ ] **Step 2: Implement QuickJS runtime wrapper with resource limits**

```rust
use rquickjs::{Runtime, Context, MemoryUsage};

pub struct QuickJsRuntime {
    runtime: Runtime,
}

impl QuickJsRuntime {
    pub fn new(memory_limit_mb: usize) -> Self {
        let runtime = Runtime::new().unwrap();
        runtime.set_memory_limit(memory_limit_mb * 1024 * 1024);
        // CPU fuel: set interrupt handler that tracks time budget
        runtime.set_interrupt_handler(Some(Box::new(|_| {
            // Check if budget exhausted; return true to interrupt
            // Implemented via thread-local time tracking
            false
        })));
        Self { runtime }
    }

    pub fn create_context(&self) -> Context {
        self.runtime.new_context().unwrap()
    }
}
```

- [ ] **Step 3: Add to workspace Cargo.toml**

---

### Task 2: Rewrite extension_api

**Files:**
- Modify: `crates/extension_api/src/*.rs`

- [ ] **Step 1: Define terminal-oriented API types**

Replace all Zed editor extension API types with terminal-oriented types:

```rust
// Extension manifest runtime declaration
pub struct ExtensionRuntime {
    pub side: RuntimeSide,  // Server | Client | Both
    pub sync: bool,
}

pub enum RuntimeSide {
    Server,
    Client,
    Both,
}

// API surface exposed to JS extensions
pub trait ExtensionContext {
    fn mux(&self) -> &MuxApi;
    fn commands(&self) -> &CommandApi;
    fn keymaps(&self) -> &KeymapApi;
    fn settings(&self) -> &SettingsApi;
    fn terminal(&self) -> &TerminalApi;
    fn register_chrome_view(&self, id: &str, view: Box<dyn ChromeView>);
}

// Mux operations available to extensions
pub trait MuxApi {
    fn subscribe(&self, event: &str, callback: JsCallback);
    fn split_pane(&self, direction: &str);
    fn current_session(&self) -> SessionInfo;
    fn focused_pane(&self) -> PaneInfo;
}
```

- [ ] **Step 2: Add runtime.side and sync to extension.toml format**

Parse `[runtime]` section from `extension.toml`:
```toml
[runtime]
side = "server"
sync = true
```

---

### Task 3: Rewrite extension_host

**Files:**
- Modify: `crates/extension_host/src/*.rs`

- [ ] **Step 1: Replace node_runtime dependency with quickjs_runtime**

Remove all references to `node_runtime`. Replace Node.js process management with QuickJS in-process execution.

- [ ] **Step 2: Implement extension loading**

```rust
pub fn load_extensions(extensions_dir: &Path) -> Vec<LoadedExtension> {
    // Scan extensions_dir for extension.toml files
    // Parse manifest (including [runtime] section)
    // For each extension:
    //   - Create QuickJS context
    //   - Load JS source from src/index.js
    //   - Bind API objects to context
    //   - Execute activate(context)
}
```

- [ ] **Step 3: Implement extension lifecycle**

- `activate`: called on load/reload
- `deactivate`: called on unload/reload
- Chrome view registration: extension registers chrome views; host tracks them
- Extension crash detection: QuickJS context throws → mark extension as crashed → native chrome reappears

---

### Task 4: Implement native chrome baseline

**Files:**
- Create: `crates/z3rm/src/chrome/` (or in ui crate)
- Create native implementations for each chrome component

- [ ] **Step 1: Implement native tab bar**

GPUI view that renders tabs from server layout snapshot. Supports top tabbar and stacked tabbar (runtime switchable via setting).

- [ ] **Step 2: Implement native status bar**

GPUI view showing: session name, focused pane title, git branch (if available), clock.

- [ ] **Step 3: Implement native command palette**

Reuse Zed's `command_palette` crate. Adapt for terminal commands.

- [ ] **Step 4: Implement native which-key hints**

Reuse Zed's `which_key` crate.

- [ ] **Step 5: Implement chrome replacement mechanism**

When extension chrome activates → replace native view with extension's VDOM rendering. If extension crashes → native view reappears. Core commands remain available through native keybindings regardless of chrome state.

---

### Task 5: Implement chrome VDOM bridge

**Files:**
- Create: `crates/extension_host/src/vdom_bridge.rs`

- [ ] **Step 1: Implement VDOM JSON → GPUI element mapping**

Extension ChromeView.render() returns JSON VDOM:
```json
{"type": "hbox", "children": [
    {"type": "text", "text": "session-name", "style": "emphasis"},
    {"type": "spacer"},
    {"type": "text", "text": "git-branch", "style": "dim"}
]}
```

Bridge parses JSON, creates GPUI div() elements with children.

- [ ] **Step 2: Implement VDOM diff**

Compare consecutive VDOM trees, only update changed elements. Minimize GPUI element recreation.

---

### Task 6: Implement extension sync to remote server

**Files:**
- Modify: `crates/extension_host/src/sync.rs`

- [ ] **Step 1: Implement on-connect sync**

When MuxDomain connects to remote server:
1. Read local extensions directory
2. For each extension with `runtime.side = "server"` or `"both"` and `sync = true`:
   - Send manifest + JS source via mux_protocol (new RPC: `InstallExtension`)
3. Server loads received extensions into its QuickJS host

- [ ] **Step 2: Implement on-install sync**

When user installs a new extension locally:
- Detect change in extensions directory
- Push new extension to all connected remote servers

---

### Task 7: Implement server-side extension chrome RPC

**Files:**
- Modify: `crates/mux_server/src/` (add extension host module)

- [ ] **Step 1: Server-side QuickJS host**

mux_server includes a QuickJS runtime for server-side extensions. Server-side extensions can access: PTY grid state, remote filesystem, agent state.

- [ ] **Step 2: Chrome output via mux_protocol**

When server-side extension renders chrome → serialize VDOM → send `ExtensionChromeUpdate` notification → client renders.

---

### Task 8: Implement extension CLI install

**Files:**
- Modify: `crates/z3rm/src/main.rs` (add subcommand)

- [ ] **Step 1: Implement `z3rm extension install <path-or-url>`**

- Local path: copy directory to `~/.config/z3rm/extensions/`
- Git URL: clone to extensions directory
- Trigger reload of extension host

---

### Task 9: Tests

- [ ] **Step 1: Unit tests — QuickJS resource limits**

Load extension that does `while(true){}` → verify fuel exhaustion kills it within ~150ms.

Load extension that allocates >64MB → verify memory limit throws.

- [ ] **Step 2: Unit tests — extension loading/activation**

Create test extension → load → verify activate() called → verify chrome view registered.

- [ ] **Step 3: Unit tests — VDOM diff**

Two consecutive VDOM renders → verify diff produces minimal GPUI updates.

- [ ] **Step 4: Fuzz testing — QuickJS↔Rust FFI boundary**

Fuzz the VDOM bridge: feed malformed JSON → verify graceful error, not panic.

- [ ] **Step 5: Run tests**

Run: `cargo test -p quickjs_runtime -p extension_host`
Expected: ALL PASS

- [ ] **Step 6: Commit**

```bash
git add crates/quickjs_runtime crates/extension_api crates/extension_host Cargo.toml
git commit -m "Add QuickJS runtime, rewrite extension system for terminal, native chrome baseline, extension sync"
```
