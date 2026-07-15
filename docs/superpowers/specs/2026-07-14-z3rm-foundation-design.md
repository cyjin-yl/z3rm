# Z3rm Foundation Design

> Converting a Zed editor fork into a high-performance terminal + multiplexer.

## 1. Product Definition

**Z3rm** is a high-performance GPU-rendered terminal with a built-in multiplexer (tmux-class capability), a read-only file viewer with diff review, and a QuickJS extension system where all UI chrome is implemented as plugins.

Forked from Zed. All editor/AI/collaboration features are removed. The retained core: GPUI rendering engine, terminal emulation (alacritty-based), workspace pane management, theme/settings infrastructure, and a slimmed read-only editor for file/diff viewing.

### 1.1 Process Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    z3rm (GUI client)                       │
│                                                               │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌─────────────────┐  │
│  │Terminal  │ │File      │ │Settings  │ │QuickJS Extension│  │
│  │View      │ │Viewer    │ │Pane      │ │Host             │  │
│  │(GPUI)    │ │(editor   │ │          │ │                 │  │
│  │          │ │ readonly)│          │ │                 │  │
│  └────┬─────┘ └────┬─────┘ └──────────┘ └───────┬─────────┘  │
│       │            │                         │              │
│  ┌────▼────────────▼─────────────────────────▼────────────┐  │
│  │              workspace (pane_group/tab/resize)          │  │
│  └────────────────────────┬───────────────────────────────┘  │
│                           │                                   │
│  ┌────────────────────────▼───────────────────────────────┐  │
│  │         MuxDomain (single unified Domain)               │  │
│  │  ┌─────────────────────────────────────────────────┐   │  │
│  │  │ MuxTransport (auto-selected)                     │   │  │
│  │  │  ├── Local: Unix socket / Named pipe             │   │  │
│  │  │  ├── Remote: SSH tunnel (TCP-like)               │   │  │
│  │  │  └── Remote resilient: UDP + AEAD + roaming      │   │  │
│  │  └─────────────────────────────────────────────────┘   │  │
│  └────────────────────────────────────────────────────────┘  │
└────────────┬─────────────────────────────────────┬───────────┘
             │                                      │
        ┌────▼─────┐                    ┌──────────▼──────────┐
        │ Local    │                    │ mux_server           │
        │ mux_     │                    │ (local or remote)    │
        │ server   │                    │ PTY + session +      │
        │ (same    │                    │ snapshot engine      │
        │ machine) │                    │ + process keepalive  │
        └──────────┘                    └──────────────────────┘
```

### 1.2 Key Design Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Upstream sync strategy | Hard fork + periodic cherry-pick | Destructive changes make merge untenable; GPUI updates are manually portable |
| Migration approach | Progressive pruning of `zed` + `workspace` crate | Reuse window/settings/theme infrastructure; mark holes with `z3rm_todo` macro |
| Project/git depth | Lightweight project (worktree + git status only) | Support file tree sidebar + diff view for CLI agent workflows |
| Editor | Preserve slimmed read-only editor crate | Reuse tree-sitter syntax highlighting, line numbers, search, diff rendering |
| Terminal state ownership | Server-canonical | mux_server owns PTY + alacritty emulator + grid; client renders grid only; no dual-parser divergence |
| Multiplexer model | MuxDomain over Unix socket (local) or SSH tunnel (remote) | Single data path; same framed binary protocol for local and remote |
| Process model | Client-server (mux_server daemon) | PTY ownership in daemon enables detach/reattach + keepalive. **Cost:** every local session requires a daemon process — adds cold-start latency, stale socket cleanup, daemon version skew as failure modes. Accepted as the price of server-canonical state. |
| Extension system | QuickJS (bundled), new terminal-oriented API | Reuse Zed extension project format; chrome implemented as plugins |
| Daemon lifecycle | `keep_alive = true` by default | Matches tmux expectations; never silently kill PTYs |
| Session persistence | Layout metadata only (no grid content to disk) | Grid lives in memory; crash = lose grid, keep layout + shells |
| Platform priority | All platforms, Windows requires real CI runner (not Wine) | ConPTY cannot be tested via Wine; Wine only for compilation checks |
| Licensing | Layered: inherited GPL-3.0-or-later, new crates Apache-2.0 | Preserve copyleft; own new work under permissive terms |

### 1.3 Competitive Research Summary

Detailed reports in `docs/competitive-research/`. Key takeaways:

| Source | Lesson Applied |
| **mosh** | SSP transport design (UDP + per-packet AEAD + stateless roaming + frame-rate control + local-echo prediction); transport-layer resilience is orthogonal to multiplexing |
| **ghostty** | Per-surface read/parse/render thread separation (z3rm gets this for free via server/client split); compile-time SIMD VT parsing; library-first design (libghostty C ABI forces clean separation); glyph atlas + textured-quad compositing is the consensus pattern — keep GPUI's existing atlas path |
| **kitty** | Kitty graphics protocol = de-facto inline-image standard (adopt wholesale; its local-shm/remote-chunked transmission maps to z3rm's local-socket/SSH-tunnel split); capability-scoped permission model with action-glob passwords for mux control; versioned JSON wire protocol + async/streaming framing; pluggable overlay layouts (Tall/Fat/Grid/Splits) as client pane arrangement; transmit-image-once/place-many model for multiplexer image rendering |
| **competitive gap** | Neither ghostty nor kitty has a headless persistent session daemon with detach/reattach. Ghostty has none; kitty approximates with listen-socket + scripts. z3rm's server-canonical mux_server with attach/detach + keep_alive is the genuine differentiator. |
| **tmux** | Server owns all PTYs + screen models; layout as recursive tree with checksummed serialization; versioned wire protocol from day one |
| **zellij** | Thread-per-concern actor model with typed instruction enums; all chrome as plugins (WASM→QuickJS for us); prost forward-compat contract from day one; avoid god-object files |
| **wezterm** | Domain trait abstraction — local/remote panes share one interface; notification bus decouples PTY I/O from rendering; output coalescing + DEC-2026 sync output; GPU glyph atlas + dirty-region tracking |
| **herdr** | Server-owns-PTYs + rendered-frame streaming (validates our server-canonical model); Rust project successfully FFI-binding Ghostty's Zig VT core (`libghostty-vt`) — evidence that alternative VT cores are integrable; PTY fd passing for live handoff across server replacement (worth studying for daemon updates); explicit state/runtime separation in AGENTS.md (validates §15 constraints); their early server/TUI coupling was a documented mistake we avoid by design |
| **competitive gap** | None of tmux, zellij, ghostty, kitty, or herdr combines server-canonical multiplexing + GPUI GPU rendering + read-only editor/file-viewer + CLI agent diff review. herdr is closest in architecture (server-canonical, agent-aware) but is a TUI (ratatui), not a GPU-rendered GUI. z3rm occupies the gap between GUI terminal performance and multiplexer durability. |

## 2. Crate Classification

### 2.1 Retained and Pruned Zed Crates (~50)

**Core infrastructure (near-unchanged):**
`gpui`, `gpui_macos`, `gpui_linux`, `gpui_windows`, `gpui_platform`, `gpui_wgpu`, `gpui_web`, `gpui_shared_string`, `gpui_macros`, `gpui_util`, `gpui_tokio`, `text`, `rope`, `collections`, `util`, `util_macros`, `sum_tree`, `clock`, `fuzzy`, `fuzzy_nucleo`, `refineable`, `feature_flags`, `feature_flags_macros`

**Terminal (modified for Domain abstraction):**
- `terminal` — alacritty PTY + VT parsing preserved; PTY management migrates to mux_server, accessed via Domain trait
- `terminal_view` — GPUI rendering preserved; data source changes from direct Terminal entity to Domain pane subscription

**Editor (surgical pruning to read-only viewer + diff):**

Zed's editor already has a `read_only` mode (used for preview buffers). The pruning strategy is surgical excision of editing-only and LSP-only modules. No fallback — the goal is to preserve the full tree-sitter syntax highlighting, display map, folding, search, and diff rendering.

**Delete entirely (editing-only / LSP-only modules):**
- `input.rs` (121KB) — text input handling
- `completions.rs`, `code_actions.rs`, `code_context_menus.rs`, `code_lens.rs` — LSP-driven editing UI
- `edit_prediction.rs` — AI inline predictions
- `inlays/inlay_hints.rs` (204KB), `inlays.rs` — LSP inlay hints
- `signature_help.rs`, `hover_links.rs`, `hover_popover.rs` — LSP hover/signature
- `lsp_ext.rs`, `rust_analyzer_ext.rs`, `clangd_ext.rs` — LSP-specific extensions
- `jsx_tag_auto_close.rs`, `linked_editing_ranges.rs` — LSP-driven editing features
- `runnables.rs`, `tasks.rs` — LSP task integration
- `rewrap.rs`, `markdown_actions.rs` — editing actions

**Retain (rendering + read-only navigation + diff):**
- `element.rs` (488KB) — core rendering engine
- `display_map.rs` + `display_map/` — text layout, wrapping, fold visualization
- `editor.rs` (446KB) — core state; prune edit action handlers, enforce `read_only = true`
- `scroll.rs` + `scroll/`, `movement.rs`, `selection.rs`, `selections_collection.rs` — navigation + selection (copy in read-only)
- `items.rs` — workspace item integration
- `fold.rs`, `folding_ranges.rs` — visual code folding
- `git.rs`, `git/blame.rs` — git blame display
- `semantic_tokens.rs`, `document_colors.rs`, `document_links.rs` — syntax highlighting + visual
- `highlight_matching_bracket.rs`, `indent_guides.rs`, `bracket_colorization.rs` — visual
- `blink_manager.rs`, `clipboard.rs` (copy only), `bookmarks.rs` — UI utilities
- `actions.rs`, `editor_settings.rs`, `config.rs` — action/settings definitions (prune edit fields)
- `split.rs`, `split_editor_view.rs` — split view (needed for side-by-side diff)
- `persistence.rs` — cursor/scroll position persistence
- `navigation.rs`, `diagnostics.rs`, `mouse_context_menu.rs` — retain for display-only (prune LSP/edit parts)
- `element/header.rs`, `element/mouse.rs` — editor header + mouse interaction
- `multi_buffer` — retained (generic multi-file buffer; diff display uses it for side-by-side)
- `buffer_diff` — fully retained (diff computation)
- `language`, `language_core` — preserve syntax highlighting, delete LSP integration
- `grammars` — fully retained (tree-sitter)
- `syntax_theme` — fully retained
- `language_selector` — retained

**Workspace / Project (pruned for terminal-first):**
- `workspace` — preserve pane/tab/resize; delete editor/project/buffer coupling; default item = terminal
- `project` — major prune: delete buffer/language registry/LSP/index/task; retain worktree (filesystem monitoring) + basic git status
- `worktree` — retained (`.gitignore` support reused)
- `git`, `git_hosting_providers` — retained
- `git_ui` — major prune: delete commit/diff editing; retain staged files list + diff viewer
- `project_panel` — retained (file tree sidebar)
- `recent_projects`, `file_finder`, `file_icons` — retained
- `search` — retained but **must be reworked to ripgrep-on-worktree** (not Zed's buffer/multi-buffer-based search). Zed's content search depends on project's buffer model, which is being deleted. The search crate will be pruned to: filesystem search via `rg` (content) + worktree entries (filename). This is a known `broken-ref` category hole during migration.

**Settings / Theme (engine retained, schema rewritten):**
- `settings`, `settings_macros` — engine retained
- `settings_json`, `settings_content` — rewrite schema (terminal + mux config)
- `settings_ui`, `settings_profile_selector` — retained (settings pane as workspace item)
- `theme`, `theme_settings`, `theme_extension`, `theme_importer`, `theme_selector` — retained
- `assets`, `icons`, `component`, `ui`, `ui_input`, `ui_macros`, `ui_prompt` — retained

**Extension system (Node → QuickJS):**
- `extension`, `extension_host` — skeleton retained, replace Node runtime with QuickJS, rewrite API
- `extension_api` — rewritten for terminal-oriented API
- `extensions_ui`, `extension_cli` — retained
- `node_runtime` — **deleted**

**RPC / Remote (retained, renamed):**
- `proto`, `rpc` — RPC framework retained; protocol definitions rewritten for mux
- `remote`, `remote_connection`, `remote_server` — retained and renamed to avoid confusion with Zed's server process

**Utilities (retained):**
`command_palette`, `command_palette_hooks`, `which_key`, `keymap_editor`, `tab_switcher`, `picker`, `picker_preview`, `menu`, `title_bar`, `platform_title_bar`, `sidebar`, `panel`, `http_client`, `http_client_tls`, `http_proxy`, `reqwest_client`, `net`, `markdown`, `html_to_markdown`, `notifications`, `askpass`, `paths`, `fs`, `env_var`, `encoding_selector`, `line_ending_selector`, `db`, `sqlez`, `sqlez_macros`

**Reliability / Logging (retained as dead code or localized):**
`zlog`, `zlog_settings`, `telemetry` (dead code, future repurpose), `telemetry_events`, `crashes`, `ztracing`, `ztracing_macro`, `auto_update`, `auto_update_helper`, `auto_update_ui`, `release_channel`, `session`

### 2.2 Deleted Crates (~90)

**AI / Agent / LLM:**
`agent`, `agent_servers`, `agent_settings`, `agent_skills`, `agent_ui`, `ai_onboarding`, `acp_thread`, `acp_tools`, `edit_prediction` (all variants), `prompt_store`, `web_search`, `web_search_providers`

**LLM Providers:**
`anthropic`, `bedrock`, `cloud_llm_client`, `codestral`, `deepseek`, `google_ai`, `language_model` (all variants), `lmstudio`, `mistral`, `ollama`, `open_ai`, `open_router`, `opencode`, `x_ai`, `copilot`, `copilot_chat`, `cloud_api_client`, `cloud_api_types`

**Collaboration / Communication:**
`collab`, `collab_ui`, `channel`, `call`, `audio`, `livekit_api`, `livekit_client`, `client`

**Editor auxiliary:**
`vim`, `vim_mode_setting`, `debugger_tools`, `debugger_ui`, `dap`, `dap_adapters`, `debug_adapter_extension`, `repl`, `svg_preview`, `csv_preview`, `image_viewer`, `mermaid_render`, `prettier`, `dev_container`, `tasks_ui`, `task`, `feedback`, `journal`

**Other:**
`node_runtime`, `schema_generator`, `streaming_diff`, `media`, `migrator`, `install_cli`, `open_path_prompt`, `onboarding`, `language_onboarding`, `system_specs`, `time_format`, `scheduler`, `watch`, `input_latency_ui`, `inspector_ui`, `miniprofiler_ui`, `component_preview`, `markdown_preview`, `language_tools`, `snippet`, `snippet_provider`, `snippets_ui`, `outline`, `outline_panel`, `breadcrumbs`, `action_log`, `activity_indicator`, `context_server`, `windows_resources`, `toolchain_selector`, `explorer_command_injector`, `eval_cli`, `eval_utils`, all benchmark crates

### 2.3 New Crates — Foundation (Day 0)

All crates below must compile and be functional after the two-pass migration is complete (M4). No scaffold-and-stub.

| Crate | Responsibility | License |
|---|---|---|
| `crates/z3rm` | New entry point (replaces `zed`), slimmed main.rs | Apache-2.0 |
| `crates/mux` | MuxDomain, MuxTransport (enum: Local/Ssh), session lifecycle, notification stream, grid sync (generation counter), file fetch RPCs | Apache-2.0 |
| `crates/mux_server` | Headless daemon: PTY management, alacritty emulator, session keepalive, layout metadata persistence | Apache-2.0 |
| `crates/mux_protocol` | prost/protobuf wire protocol (versioned contract), grid diff types, file fetch types | Apache-2.0 |
| `crates/shadow_snapshot` | Persistent-tree filesystem versioning (version tree, WAL, SQLite), runs on file's host machine (local or remote mux_server), per-project quota | Apache-2.0 |
| `crates/quickjs_runtime` | QuickJS engine bundled via `rquickjs`, replaces `node_runtime`, resource limits (CPU fuel, memory, IO rate), dedicated OS thread | Apache-2.0 |
| `crates/z3rm_macros` | `#[z3rm_todo]` macro + project-specific proc macros | Apache-2.0 |
| `crates/transport_resilient` | UDP + per-packet AEAD + stateless roaming + RTT estimation (mosh-inspired transport layer) | Apache-2.0 |

### 2.4 Post-Foundation

No new crates. All crates are Day 0 (§2.3). Post-foundation enhancements (marketplace, Kitty graphics, log viewer UI) are additions to existing Day 0 crates.

## 3. Mux Architecture

### 3.1 Server-Cannonical Terminal State

**mux_server is the single source of truth for terminal state.** It owns PTY fds, runs the alacritty terminal emulator, parses DEC-2026, and holds scrollback. The GUI client never parses PTY bytes — it only renders grid snapshots/diffs received from the server. This eliminates the dual-parser divergence problem.

There is exactly **one data path** for both local and remote:

```
PTY bytes → mux_server alacritty emulator → terminal grid state
                                                │
                                    ┌───────────┴───────────┐
                                    │ local                 │ remote
                                    │ Unix socket           │ SSH tunnel
                                    │ framed binary         │ framed binary
                                    │ grid diff             │ grid diff
                                    └───────────────────────┘
                                                │
                                        GUI renders grid only
```

Local and remote use the **same protocol** (framed binary messages over a socket). The only difference is the socket type: Unix domain socket (local) vs SSH-forwarded channel (remote). No shared memory, no dual parsing, no special-cased local fast path. Benchmark first; optimize later if needed.

### 3.2 MuxDomain — Concrete Struct, No Trait

There is exactly one implementation — `MuxDomain`. No `Domain` trait: a trait with a single implementation is premature abstraction. If a second implementation is needed later (e.g., test mock), extract a trait then.

```rust
pub struct MuxDomain {
    transport: MuxTransport,
}

// Enum for platform-correct local sockets.
// Local uses interprocess crate's local socket (Unix domain socket on Unix,
// named pipe on Windows). Ssh uses a forwarded channel.
pub enum MuxTransport {
    Local(LocalSocketStream),
    Ssh(SshChannel),
}

impl MuxDomain {
    // Session management
    pub async fn list_sessions(&self) -> Result<Vec<SessionInfo>>;
    pub async fn create_session(&self, name: &str, cwd: &Path) -> Result<SessionId>;
    pub async fn kill_session(&self, id: SessionId) -> Result<()>;
    pub async fn rename_session(&self, id: SessionId, name: &str) -> Result<()>;

    // Pane/tab management
    pub async fn spawn_pane(&self, session: SessionId, tab: TabId, size: TerminalSize,
                            command: Option<ShellCommand>, cwd: Option<&Path>) -> Result<PaneId>;
    pub async fn split_pane(&self, pane: PaneId, direction: SplitDirection) -> Result<PaneId>;
    pub async fn close_pane(&self, pane: PaneId) -> Result<()>;
    pub async fn focus_pane(&self, pane: PaneId) -> Result<()>;
    pub async fn resize_pane(&self, pane: PaneId, cols: u16, rows: u16) -> Result<()>;
    pub async fn set_pane_title(&self, pane: PaneId, title: &str) -> Result<()>;

    // Input
    pub async fn send_input(&self, pane: PaneId, bytes: &[u8]) -> Result<()>;
    pub async fn paste(&self, pane: PaneId, text: &str) -> Result<()>; // bracketed paste aware

    // Grid state (pull-based, generation-tracked)
    pub async fn fetch_grid_update(&self, pane: PaneId, since: Generation) -> Result<GridUpdate>;
    pub async fn fetch_scrollback(&self, pane: PaneId, from: usize, count: usize) -> Result<ScrollbackChunk>;

    // Attach/detach
    pub async fn attach(&self, session: SessionId) -> Result<()>;
    pub async fn detach(&self) -> Result<()>;

    // Subscription (push notifications for low-frequency events)
    // Returns a typed channel, not impl Stream (avoids RPITIT dyn-compat issues).
    pub fn subscribe(&self) -> mpsc::Receiver<MuxNotification>;
}
```

Notes on API choices:
- No trait: `MuxDomain` is a concrete struct. No `async_trait` overhead, no `Pin<Box<dyn>`, no RPITIT compatibility concerns.
- `fetch_grid_update` returns `Result`: transport can fail mid-fetch (socket EOF, SSH disconnect). This is the hot path and must not panic or hang.
- `resize_pane` is the single resize method (takes `cols`/`rows` — the universal terminal size unit). No separate `resize` with `TerminalSize`.
- `subscribe` returns `mpsc::Receiver` instead of `impl Stream`: concrete type, no RPITIT, single consumer pattern.
- Missing from this list but planned for post-foundation: search, copy mode, pane zoom, multiple windows per session, per-client identity/permissions.

### 3.3 Grid Sync — Generation Counter

Grid updates use a **per-pane generation counter**, not a dirty bool. This solves the race condition where output arrives between dirty-set and client-fetch:

```rust
pub struct GridUpdate {
    pub from_generation: Generation,
    pub to_generation: Generation,
    pub update: GridUpdateKind,
}

pub enum GridUpdateKind {
    Diff(TerminalGridDiff),     // incremental: client is behind by a few frames
    FullSnapshot(TerminalGrid), // ring overflow or client too far behind → full resync
}

pub struct TerminalGrid {
    pub cells: Vec<Cell>,       // rows × cols
    pub cursor: CursorState,
    pub scroll_offset: usize,   // current scroll position
    pub alternate_screen: bool,
}
```

**Protocol:**
1. mux_server increments the pane's generation counter each time the grid mutates (PTY output processed).
2. Client tracks `last_seen_generation` per pane.
3. On each repaint frame, client calls `fetch_grid_update(pane, last_seen_generation)`.
4. If `last_seen_generation < oldest_diff_in_ring` → `FullSnapshot` (ring overflow, client was too far behind).
5. If `last_seen_generation == current` → no update (returns same generation).
6. If `last_seen_generation >= oldest_diff_in_ring` → `Diff` (delta from last seen to current).

**Ring buffer (server-side):** mux_server keeps a bounded ring of recent grid diffs per pane (configurable, default 64 entries ≈ 64 frames of history). When the ring wraps, old diffs are discarded. A client that falls behind the ring's oldest entry gets a full snapshot.

**Push wakeup:** The pull model needs a lightweight push signal to wake the render loop. mux_server sends a `PaneDirty(PaneId)` push notification (no data, just "pane X has new output"). GPUI receives this, schedules a repaint, and on the next frame calls `fetch_grid_update`. This is "push a signal + pull the data."

**Adaptive coalescing:** The PTY reader thread dynamically adjusts its batching window:

| Mode | Trigger | Window | Behavior |
|---|---|---|---|
| Interactive | Keyboard input active, PTY < 4KB/s | 0ms | Zero added latency for keystroke-to-cursor |
| Normal | Default | 2ms | Batch small TUI updates |
| High-throughput | PTY > 100KB/s sustained > 500ms | 8–16ms | Frame dropping: only latest grid rendered |

DEC-2026 synchronized output: mux_server parses BSU/ESU markers in the PTY stream and defers grid generation bump until ESU arrives.

### 3.4 Notification Model

All low-frequency events (pane added/removed/focused, tab title, layout, exit, pane dirty signal) use **push** delivery via a `Stream<Item = MuxNotification>`:

```rust
pub enum MuxNotification {
    PaneDirty(PaneId),            // lightweight: "new output, pull on next frame"
    PaneAdded(PaneId),
    PaneRemoved(PaneId, ExitStatus),
    PaneFocused(PaneId),
    TabTitleChanged(TabId, SharedString),
    SessionLayoutChanged(LayoutSnapshot),
}
```

**Delivery guarantee:** `PaneDirty` is at-most-once (missing one is harmless — next repaint polls). Lifecycle events (`PaneAdded`, `PaneRemoved`, `SessionLayoutChanged`) are **at-least-once** — losing a `PaneRemoved` creates a zombie pane. Ordered within a single pane ID. On reconnect, the client does **not** rely on missed notifications — it calls `attach()` which returns a full authoritative session snapshot (panes, tabs, layout, generations) per §15.4.

### 3.5 Process Keepalive

```
GUI client disconnects (window close, crash, network drop)
    │
    ├── mux_server continues holding PTY master fds
    ├── Child processes keep running
    └── User reopens z3rm → client connects → reattach
        → client fetches full grid snapshot for each pane → renders
```

**Daemon lifecycle:**
- Default: `keep_alive = true` — daemon stays alive until explicitly killed via `z3rm-server kill` or system shutdown. This matches tmux user expectations; silently killing PTYs after a timeout is a footgun.
- Configurable: `keep_alive_seconds` for users who prefer auto-exit after idle period.
- `z3rm-server kill` / `z3rm-server kill --session <id>` CLI commands for manual termination.

### 3.6 Session Persistence

**Runtime state (best-effort):** mux_server snapshots session **layout metadata** (layout tree, pane cwd/title/command, tab structure) to SQLite periodically (~10s). Grid content is NOT persisted to disk; only layout metadata is. SQLite runs in WAL mode. The terminal grid lives in memory. If mux_server crashes, all grid state and all PTYs are lost (§15.6). Layout metadata survives for crash recovery.

**Crash recovery (conservative):** On startup, if non-clean-exit sessions are found in SQLite, prompt: "Found N interrupted sessions. Restore layout and working directories? Commands will NOT be re-run automatically." Recovery restores layout + opens shells at recorded cwd. **Commands are never auto-re-spawned** — auto-re-running `terraform apply` or `cargo publish` is dangerous. User can manually re-run from the restored shell.

### 3.7 Layout Persistence

Persistence uses **absolute cell counts** with checksum (matching tmux's `layout_dump`), not percentages:

```
# b74e,80x24,0,0{40x24,0,0,0,40x24,40,0,1}
# <checksum>,WxH,xoff,yoff,paneid (leaf) | {children} (LR split) | [children] (TB split)
# Stored as absolute; scaled proportionally on restore to current window size.
```

### 3.8 Failure Modes and Invariants

| Failure | Recovery |
|---|---|
| GUI client crash | mux_server detects socket EOF, keeps PTYs alive. User reopens → reattach → fetch full grid per pane. |
| mux_server crash (local) | All PTYs receive SIGHUP, die. Layout metadata in SQLite. Restart: offer layout restoration (shells only, no auto-rerun). |
| mux_server crash (remote) | Remote PTYs die. Client detects transport EOF, shows reconnect dialog. |
| Both crash | SQLite layout metadata survives. Start fresh, offer layout restoration. |
| Clock rollback (NTP) | Not relevant for foundation — layout persistence uses seq_no for ordering. |
| Disk full | mux_server stops layout snapshotting, notifies client. Terminal continues working. |
| SQLite corruption | WAL replay. If DB corrupt, start fresh. Layout metadata is best-effort. |
| Network drop (remote, SSH) | SSH keepalive detects failure. Client shows reconnect dialog. No auto-resume in foundation (requires UDP resilient transport, deferred). |

### 3.9 prost Protocol Versioning

- `PROTOCOL_VERSION` field in every message header
- Field numbers never reused (reserved for deleted fields)
- Unknown fields preserved (forward compatibility)
- Version mismatch on connect → server sends `ProtocolVersion`, client downgrades or disconnects
- **Shared memory ABI versioning** is not needed in the foundation — there is no shared memory path. Local uses Unix socket + framed binary, same as remote.

## 4. Shadow Snapshot Engine

### 4.1 Purpose

Global, time-granular, crash-safe filesystem versioning. Enables undo/decline of CLI agent file modifications at any granularity, independent of git. Works in non-git directories. Persists across process crashes and application restarts.

### 4.2 Operation Complexity

| Operation | Complexity | Notes |
|---|---|---|
| Record file change | $O(\|diff\|)$ amortized | Delta only; Zstd level-1 compressed. Materialization at $D_{max}$ boundary costs $O(\|full\|)$ but amortized over $D_{max}$ versions. |
| Query content at version V | $O(\log N + \sum_{i} \|delta_i\|)$ | $\log N$ for BTree lookup. Delta replay is additive: $\sum_{i=1}^{d} \|delta_i\|$ where $d \leq D_{max}$. Rope-level apply: each delta costs $O(\log N + \|delta_i\|)$. |
| Query changed files in [T1,T2] | $O(\log N + M + U)$ | $M$ = entries in SeqNo range. Dedup via `HashSet<PathHash>`: $O(M)$ to build, $U$ = unique paths returned. |
| Undo/Decline to version V | $O(\log N + \sum_{i} \|delta_i\| + \text{fsync})$ | Lookup + content replay + WAL append. |
| Diff(V1, V2) | $O(\log D_{depth} + \|V1 \ominus V2\|)$ | LCA via binary lifting: $O(\log D_{depth})$. Content diff is output-sensitive. |
| Quota eviction (GC) | $O(E \cdot (\|content_{max}\| + S))$ | $E$ = evicted nodes. Each may need promote-to-full ($O(\|content\|)$) plus subtree ancestor-pointer fixup ($S$). |
| Crash recovery | $O(W)$ | $W$ = WAL length. |

Where $N$ = total version nodes, $D_{max} = 16$, $M$ = entries in query range, $U$ = unique paths, $E$ = evicted nodes.

### 4.3 Concurrency: Concurrent File Writes

When two CLI agents (or panes) write the same file concurrently:

```
Agent A: writes src/foo.rs → VersionNode(N),   parent = HEAD (N-1)
Agent B: writes src/foo.rs → VersionNode(N'),  parent = HEAD (N-1)
```

Both children share the same parent. This is a valid tree fork. **The watcher processes events in SeqNo order** (monotonic internal counter assigned on event receipt, not file mtime). The first event received gets the lower SeqNo. The HEAD pointer advances to the most recent SeqNo — whichever write the watcher saw last becomes HEAD. The other branch becomes an orphan subject to the orphan branch policy (§4.3 original).

This is acceptable: concurrent writes to the same file are a race by definition. The version tree preserves both versions; the user can diff HEAD vs the orphan branch to see the losing write. No merge semantics are provided (out of scope).

WAL + MemTable synchronization: WAL appends and MemTable inserts happen on a single watcher processing thread. No concurrent insertion. WAL replay on startup is single-threaded. `seq_no` is assigned atomically before WAL append. No checkpoint race exists because there is exactly one writer thread.

### 4.4 Version Tree (not DAG)

Version history is a **persistent branching tree** (each node has exactly one `parent_id`). Undo creates branches:

```
A ──→ B ──→ C ──→ D        (normal change chain)
      │
      └──→ C' ──→ E ← HEAD  (decline C: C' content = B; then edit to E)

D is now an orphan branch (not reachable from HEAD).
```

**Orphan branch policy:** branches not reachable from any HEAD pointer are marked `gc-eligible` after a grace period (configurable, default 24h). Pruned during quota GC. Users can pin branches to prevent pruning.

**File deletion handling:** deletion is a `VersionNode` with `trigger = Delete`, `full_content = None`, `delta = None`. The file's content is recorded as "absent." A subsequent `touch` creates a new `VersionNode` with `parent_id` pointing to the delete node, `full_content = hash(empty)`. This maintains a coherent chain: content → absent → empty content → new content.

**LCA (Lowest Common Ancestor) via binary lifting:** each `VersionNode` stores $\lceil \log_2(\text{depth}) \rceil$ ancestor pointers (jump table). Enables $O(\log D)$ LCA for cross-branch diff.


### 4.5 Three-Layer Storage

**Layer 0 — WAL (Write-Ahead Log):** Append-only, fsync per group commit (batching multiple changes). Each entry includes a monotonic `SeqNo` for deterministic replay ordering. Entry: `[seq_no, path_hash, parent_id, content_ref/delta_ref, trigger]`.

**Layer 1 — MemTable (hot path):** `BTreeMap<(SeqNo), PathChange>` — keyed by monotonic SeqNo (not wall clock timestamp, to handle NTP clock rollback). Range queries on SeqNo give $O(\log N + M)$ where $M$ = matching entries. Hot cache: `Arc<Rope>` for recently accessed full file contents.

**Layer 2 — Persistent (SQLite WAL mode + content-addressed store):**
- `version_nodes` table: indexed on `(seq_no)` and `(path_hash, seq_no DESC)`
- SQLite `PRAGMA journal_mode=WAL` for concurrent reader/writer
- Blob store: content-addressed, single-level sharding `hash[0:2]/content-hash` (upgrade to two-level only if blob count exceeds ~100K)
- Delta key: `SHA-256(parent_content_hash || child_content_hash)` — deltas are not purely content-addressed; they require both parent and child hashes to locate
- Small blobs (< 4KB): stored inline in SQLite (avoids filesystem inode overhead)
- Zstd level-1 compression on all blobs and deltas
- Refcounted: undo/redo of identical content shares a single blob

### 4.6 Bounded Delta Chain with Rope-Level Replay

Delta chains are capped at $D_{max} = 16$. The 17th version forces materialization as a full snapshot. **Intermediate delta nodes are retained** — undo to any historical version must work. Storage cost: $O(N/D_{max} \cdot \|full\| + N \cdot \|delta_{avg}\|)$, acknowledged in quota accounting.

**Delta replay uses Rope operations, not String concatenation.** Each delta is a sequence of (offset, delete_length, insert_rope) operations. Applying a delta to a Rope is $O(\log N + \|insert\|)$ per operation (tree surgery), not $O(\|full\_content\|)$ memory copy. Replaying $D_{max}$ deltas on a 10MB file costs $D_{max} \cdot O(\log N + \|delta_{avg}\|)$, not $D_{max} \cdot O(10\text{MB})$.

### 4.7 File Monitoring and Circuit Breaker

**Reuses worktree's existing file watcher** — no double-watching. Shadow snapshot subscribes to the same event stream that worktree already maintains, avoiding event ordering inconsistency.

```
worktree event stream (single subscription)
    │
    ├── ignore filter (evaluated BEFORE any processing):
    │     .gitignore (from worktree) + default list + .z3rmignore
    │     default: .git, node_modules, target, .next, __pycache__,
    │              *.o, *.obj, *.exe, *.out, *.elf, *.dll, *.so, *.dylib,
    │              *.pyc, *.class
    │     (no domain-specific patterns like test data — users add their own via .z3rmignore)
    │
    ├── write event → debounce 500ms → merge into one version
    ├── file close → force flush version
    ├── binary detection (not just size): ELF magic, PE magic, Mach-O magic
    │     detected binaries: store metadata only (no content)
    │
    └── FREQUENCY CIRCUIT BREAKER:
          if file F written > K times in 1s (default K=10):
            suspend snapshotting for F until 2s of idle
            log: "snapshot suspended for <F>: write frequency exceeded threshold"
            (prevents stress-test / cargo build / webpack --watch storms)
```

### 4.8 Undo / Decline — Crash-Safe Protocol

```
User selects Decline for src/foo.rs (currently at version N, restore to N-1):

  Step 1: Write WAL entry:
    [seq_no, path_hash(src/foo.rs), parent=N, trigger=Decline, content_ref=hash(N-1)]
    → fsync WAL

  Step 2: Write file to disk:
    restore src/foo.rs content to VersionNode(N-1)

  Step 3: Watcher sees the file change from Step 2.
    BUT: watcher checks content hash against pending WAL entries.
    The Decline WAL entry (Step 1) has content_ref = hash(N-1).
    The file now has content hash(N-1).
    → Match found → watcher SKIPS this event (it was caused by our own Decline).

  Step 4: MemTable updated with new VersionNode(N+1), trigger=Decline.

Crash between Step 1 and 2:
  WAL has Decline entry but file is still at version N.
  On recovery: replay WAL → see Decline entry → re-execute Step 2 (restore file).

Crash between Step 2 and 3:
  File is at N-1 content, WAL has Decline entry.
  Watcher fires (sees file changed) but matches pending Decline → skips.
  Recovery: WAL replay → Step 4 completes.
```

### 4.9 Quota Management — Age-Based Eviction

Default: per-project quota (500MB, configurable to unlimited).

**Eviction is age-based (FIFO by `seq_no`), not LRU.** True LRU would require updating `last_accessed_at` on every read — write-amplification on cold data during diff view navigation is wasteful. Age-based is simpler and sufficient: old versions nobody has looked at in days are the right candidates for eviction.

**Promote-to-full cost:** when evicting a full snapshot whose delta children are still reachable from a HEAD, the child must be promoted to full. This requires replaying the child's delta onto the parent's content and writing a new full snapshot blob. Cost: $O(\|content\|)$. This is acknowledged in the eviction complexity: $O(\text{evicted} \cdot \|content_{max}\|)$.

**Promote-to-full batching:** promotions are batched during a single GC pass to amortize I/O.

Git commit hook: after `git commit`, mark pre-commit deltas as `gc-eligible`. Next GC cycle prioritizes `gc-eligible` nodes. Orphan branches (not reachable from HEAD, past grace period) are also `gc-eligible`.
### 5.1 Design Principle

**All UI chrome is implemented as extensions.** The "bare" z3rm GUI has only: terminal pane rendering (GPUI), pane/tab layout engine (workspace), extension host (QuickJS runtime), settings pane.

Status bar, tab bar, session manager, layout manager, command palette, which-key hints — all extensions via QuickJS.

**Native chrome baseline:** Built-in chrome has a native GPUI implementation that serves as the primary implementation during Day 0. It runs alongside the extension host. When a QuickJS chrome extension activates successfully, it replaces the native version. If the extension crashes or is fuel-limited, native chrome reappears. This is not a "fallback" — it is the Day 0 baseline; the extension version is the enhancement.

**Core interaction independence (§15.7):** Core commands (split pane, switch pane, create/close tab, attach/detach, settings, kill session) are available through native keybindings independent of the extension host.


### 5.2 QuickJS Runtime — Resource Limits and Thread Isolation

QuickJS is bundled at compile time via the `rquickjs` crate. No external runtime download.

**Thread isolation:** The extension host runs on a **dedicated OS thread**, separate from the GPUI render thread. Extensions communicate with the UI via async channels (VDOM snapshots pushed to GPUI). JS code never blocks the render loop — a `while(true){}` in an extension freezes only that extension, not the terminal.

**Resource limits (enforced per extension):**
- **CPU:** QuickJS interrupt handler checks a fuel counter on every bytecode dispatch. When fuel is exhausted, execution yields. Budget: **50ms per second** (not per frame). This means an extension can consume up to 50ms of CPU per wall-clock second, checked via interrupt counter. An infinite loop (`while(true){}`) exhausts fuel rapidly and is killed after the extension exceeds its budget for 3 consecutive checks (~150ms). This does NOT affect the render loop — the extension runs on its own thread.
- **Memory:** `JS_SetMemoryLimit` enforced per JSContext. Default: 64MB per extension. Allocations beyond the limit throw `RangeError`. Note: memory limit may be partially effective when custom allocator features are enabled; this is a best-effort bound, not a hard wall.
- **IO rate limiting:** Filesystem and network API calls are rate-limited via a token bucket (configurable, default 100 ops/second per extension).

**Why QuickJS over WASM:**
- Developer experience: JavaScript/TypeScript is more accessible than WASM-targeting languages. Extensions can be developed, debugged, and hot-reloaded without a compile step.
- Ecosystem: existing JS/TS npm ecosystem for utilities, parsers, etc.
- **Trade-off acknowledged:** QuickJS is an interpreter — sustained hot-path performance is lower than wasmtime's Cranelift JIT. WASM has stronger memory isolation (hardware-enforced sandbox vs QuickJS's software isolation). rquickjs has known limitations in async environments (locking constraints). Cold start including manifest parse + JS file load + API binding init is realistically **5–20ms**, not sub-millisecond. The choice prioritizes developer ergonomics; the sandbox is a best-effort software boundary, not a security hard boundary.

### 5.3 Extension Project Structure

Reuses Zed's extension manifest format:

```toml
# extension.toml
[extension]
name = "z3rm-status-bar"
version = "0.1.0"

[capabilities]
terminal = true
mux = true
workspace = true
filesystem = "cwd"
settings = true

[resources]
memory_limit_mb = 64
cpu_budget_ms = 50
io_rate_limit = 100
```

### 5.4 Extension API and Rendering Pipeline

Extensions interact with z3rm through a typed JavaScript API:

```javascript
export function activate(context) {
    context.registerChromeView('status-bar', StatusBarView);
    context.mux.subscribe('pane:focus', (pane) => { ... });
    context.commands.register('my-ext.split-right', () => {
        context.mux.splitPane('right');
    });
    context.keymaps.bind('ctrl-b %', 'my-ext.split-right');
}
```

**Chrome Views return declarative Virtual DOM (JSON)** → z3rm's GPUI bridge maps to GPUI elements. Extensions never call GPUI directly.

**High-frequency widgets use a display-list pattern** (not a direct paint handle). For widgets requiring 30fps+ updates (CPU meter, clock), the extension returns a JSON display list instead of a full VDOM tree. The native side caches and diffs display lists, avoiding full VDOM reconciliation:

```javascript
// Extension returns a display list (JSON array of draw operations):
function renderCpuMeter(cpuPercent) {
    return [
        {"op": "fillRect", "x": 0, "y": 0, "w": cpuPercent, "h": 12, "color": "#ff0000"},
        {"op": "drawText", "text": `${cpuPercent}%`, "x": 4, "y": 10}
    ];
}
```

The native side diffs consecutive display lists and only repaints changed regions. This keeps JS→Rust FFI to one JSON serialization per update (smaller payload than full VDOM), without giving JS a direct GPU paint handle (which would break sandbox boundaries).

### 5.5 Built-in Extensions

| Extension | Function | Update Frequency |
|---|---|---|
| `z3rm-tab-bar` | Tab strip: titles, add/remove, drag-reorder | Low (event-driven) |
| `z3rm-status-bar` | Status line: session name, git branch, clock | Mixed (clock = display list; branch = VDOM) |
| `z3rm-session-manager` | Session list, switch, detach/reattach | Low (event-driven) |
| `z3rm-layout-manager` | Preset layout selection, save current layout | Low (event-driven) |
| `z3rm-command-palette` | Command palette | Low (on-demand) |
| `z3rm-which-key` | Keybinding hints | Low (on-demand) |

These extensions live in `extensions/` and use the exact same API as third-party extensions. Users can fork built-in extensions to customize chrome.

### 5.6 Permission Model

Extensions declare capabilities in `extension.toml`. First install shows a permission dialog (browser-extension style). Runtime capability violations throw. Capabilities include: `terminal`, `mux` (read/write), `filesystem` (cwd/home/none), `process_spawn`, `network`, `settings` (read/write). Resource limits (`[resources]` section) are enforced at runtime — exceeding them results in extension suspension with user notification.

## 6. Settings System

Retain Zed's settings engine (`settings`, `settings_macros`) and settings UI (`settings_ui`, `settings_profile_selector`). Rewrite configuration schema (`settings_json`, `settings_content`) for terminal/mux/extension configuration.

New schema covers: terminal config (font/shell/cwd/env), mux config (session/pane/keymap profiles), theme, extension config, shadow snapshot config.

## 7. Keymap Profiles

Built-in keymap profiles for classic multiplexer compatibility:

```json
// settings.json
{
    "keymap_profile": "tmux"
}
```

Profiles (`tmux.json`, `zellij.json`, `screen.json`) are keymap files bundled in `assets/keymaps/`. These cover a **compatible subset** of each multiplexer's keybindings — the common operations (prefix key, split, tab switch, copy mode, pane navigation). tmux's full three-state model (root/prefix/copy-mode-vi) with all subcommands is not a 1:1 mapping; the profiles target the ~80% most-used bindings. Users can fork profiles to add missing bindings. The keymap engine (Zed's existing system) supports complex key sequences, context-aware bindings, and modal tables.

## 8. Migration Strategy

### 8.1 `z3rm_todo` Macro

A proc-macro attribute that enforces migration completion. **"Fixing a hole" ≡ "deleting the `#[z3rm_todo]` attribute from that code."** There is no separate "macro cleanup" step — when you fix a hole, you remove the attribute. The count of remaining macros equals the count of remaining holes.

Switching is controlled by a **Cargo feature flag**, not a profile (proc-macros cannot see cargo profiles; they can only see `cfg(feature)`):

- **Without `z3rm-migration` feature (default):** Expands to `compile_error!` — any unfixed hole blocks compilation
- **With `--features z3rm-migration`:** Expands to `inventory::submit!` registration — compilation succeeds, build script reports remaining hole count

```rust
#[z3rm_todo(category = "removed-crate", desc = "workspace no longer depends on project::worktree")]
fn some_function() { ... }
```

Categories: `removed-crate`, `broken-ref`, `stub`, `disabled-feature`.

Build script outputs: per-category counts + total remaining holes.

### 8.2 Two-Pass Migration

**Pass 1 — Marking (scan, don't edit):**
1. Subagents scan: Rust source, `Cargo.toml` feature graph, `build.rs`, CI workflows, docs/README/scripts, keymaps/assets/settings schema, env var names, bundle IDs/installer paths
2. Mark every reference to deleted crates, every editor/agent/collab coupling point in retained crates, every rename location
3. Human review: confirm completeness before proceeding

**Pass 2 — Migration (edit, don't analyze):**
1. Fix marked holes one by one; deleting the `#[z3rm_todo]` attribute IS the fix
2. Milestone verification via `cargo check --features z3rm-migration`:
   - M0: Cargo.toml cleanup → check → count errors
   - M1: Fix all `removed-crate` holes → that category count = 0
   - M2: Fix all `broken-ref` holes → that category count = 0
   - M3: Fix all `stub` and `disabled-feature` holes → total count = 0
   - M4: `cargo check` WITHOUT `z3rm-migration` feature → clean compilation (no `compile_error!` triggers because all macros are already deleted) → migration complete

`.rs.old` discipline: `.rs.old` files are temporary local artifacts only. **They must never be committed.** Git history is the official backup. `.rs.old` files must be deleted before any commit.

### 8.3 Local Model Assistance

A local model (accessed via environment variables, credentials never committed) assists with bulk output analysis during migration. The primary orchestrator is always the main agent. The local model has limited concurrency (1) and is used for heavy-lifting analysis tasks.

### 8.4 Naming and Branding

**Layer 1 — User-visible names (one-time change):**
- `crates/zed` → `crates/z3rm`
- `default-members` updated
- `paths::APP_NAME` = `"Z3rm"`, `paths::APP_NAME_LOWERCASE` = `"z3rm"`
- Environment variable prefix `ZED_*` → `Z3RM_*`
- Bundle identifier updated
- `README.md`, `CONTRIBUTING.md` rewritten

**Layer 2 — Internal module names (gradual):**
- `mod zed` → `mod z3rm`
- `use zed::` → `use z3rm::`
- Variable names updated over time

This layered approach preserves cherry-pick compatibility with upstream.

## 9. Documentation Scaffold

```
docs/
├── architecture/
│   ├── overview.md
│   ├── crate-map.md
│   ├── mux-design.md
│   ├── shadow-snapshot-engine.md
│   ├── extension-system.md
│   └── adr/
│       ├── 0001-hard-fork-strategy.md
│       ├── 0002-mux-domain-unification.md
│       ├── 0003-udp-resilient-transport.md
│       ├── 0004-quickjs-over-wasm.md
│       ├── 0005-chrome-as-plugin.md
│       ├── 0006-shadow-snapshot-version-tree.md
│       ├── 0007-editor-readonly-preservation.md
│       ├── 0008-layered-licensing.md
│       └── ...
├── development/
│   ├── getting-started.md
│   ├── building-linux.md
│   ├── building-windows-wine.md
│   ├── migration-guide.md
│   └── local-model-usage.md
├── competitive-research/
│   ├── tmux.md
│   ├── zellij.md
│   ├── wezterm.md
│   ├── mosh.md
│   ├── ghostty.md
│   ├── kitty.md
│   └── herdr.md
```

**AGENTS.md / CLAUDE.md / .rules rewrite:**
- `AGENTS.md` — rewritten for z3rm (preserve GPUI guidelines, delete editor/agent specifics, add mux/terminal/extension development guidelines)
- `CLAUDE.md` — symlink to `AGENTS.md` (`ln -s AGENTS.md CLAUDE.md`)
- `.rules` — high-signal traps (preserve Rust + GPUI rules, delete Zed-specific rules, add z3rm-specific traps)

## 10. Licensing

**Per-crate licensing (not blanket):** Zed's codebase is "primarily GPL-3.0-or-later, with Apache-2.0 components where marked." The license audit must check each crate's actual `LICENSE` file.

| Category | License | Examples |
|---|---|---|
| GPUI framework crates | **Apache-2.0** | `gpui`, `gpui_macos`, `gpui_linux`, `gpui_windows`, `gpui_platform`, `gpui_wgpu`, `gpui_shared_string`, `gpui_macros`, `gpui_util`, `gpui_tokio` |
| Other retained Zed crates | GPL-3.0-or-later | `terminal`, `editor`, `workspace`, `project`, `settings`, `theme`, etc. (unless individually marked Apache) |
| Foundation new crates | Apache-2.0 | `z3rm`, `mux`, `mux_server`, `mux_protocol`, `z3rm_macros` |
| Post-foundation new crates | Apache-2.0 when created | `shadow_snapshot`, `quickjs_runtime`, `transport_resilient` |

The combined binary is GPL-3.0-or-later (due to linking with GPL crates). Source-level layered declaration allows others to reference GPUI and new crates under Apache-2.0 terms. **CI enforcement:** `cargo-about` / `REUSE.toml` audit blocks merge on license mismatch. New crates that copy code from GPL Zed files must carry GPL, not Apache. Each file retains its original SPDX identifier. Zed name/logo/trademark must not be used.

Apache-2.0 patent clauses are compatible with GPL-3.0 (not GPLv2). Since Zed uses GPL-3.0-or-later, this combination is valid.

## 11. Platform Considerations

### 11.1 Windows ConPTY

Windows uses ConPTY (not Unix `openpty`). The alacritty terminal engine already has ConPTY support, but ConPTY has known limitations: incomplete ANSI escape code support, resize behavior differences, and color passthrough issues. **Wine cannot test ConPTY** — Wine implements POSIX APIs, not the Windows Console API. Wine is used only for compilation verification. ConPTY behavioral testing requires real Windows CI runners.

CI strategy: GitHub Actions Windows runner for `cargo test` on `mux_server` and `terminal` crates.

### 11.2 Terminal Image Protocols

The retained terminal crate (alacritty-based) already has Sixel support. Kitty graphics protocol and iTerm2 OSC 1337 support should be added post-foundation. Not a blocking dependency.

### 11.3 Linux Wayland

GPUI's Linux backend (`gpui_linux`) supports both X11 and Wayland. Damage tracking is handled by GPUI's compositor integration. No additional work needed.

## 12. Terminal Product Details (Day 0)

These terminal features are **Day 0**. Most already exist in the retained alacritty/terminal_view crates — they need integration into the server-canonical mux model, not new implementation from scratch.

- **Copy mode / text selection** (mouse + keyboard) — selection coordinates in server grid space, synced via generation counter
- **Scrollback search** (regex) — server-side search over scrollback, results returned to client
- **OSC 52 clipboard** — server emulator parses, writes to server clipboard hub (§16.10)
- **Hyperlink detection** (OSC 8) — alacritty already has this; grid diff carries hyperlink info
- **Bracketed paste** — server emulator tracks mode; paste() RPC wraps content (§16.10)
- **Mouse reporting modes** (SGR, UTF-8) — alacritty already handles; mouse events route through send_input
- **Font ligatures and fallback chains** — GPUI already handles; no mux-specific work
- **Emoji width and CJK wide character handling** — alacritty Unicode width tables; verify correctness in grid diff
- **IME input** (critical — Chinese input) — GPUI input composition routed to server as synthetic PTY input via send_input
- **Shell integration** (OSC 7 cwd, OSC 133 prompt markers) — server emulator parses; cwd used for session metadata
- **Title updates** (OSC 0/1/2) — server emulator parses; TabTitleChanged notification (§3.4)
- **Pane zoom** — layout operation; server tracks zoomed pane; layout push to clients
- **Synchronized updates** (DEC-2026) — already in §3.3 (BSU/ESU timeout)
- **Resize semantics** (TIOCSWINSZ propagation) — server sends resize to PTY on pane resize RPC
- **Keybinding passthrough** (prefix key vs application shortcuts) — already in §16.5
- **Daemon version skew** — protocol version negotiation (§3.9); graceful error on mismatch

The only image protocol that is post-foundation: Kitty graphics protocol and iTerm2 OSC 1337 (Sixel is already in alacritty). These are enhancements, not blocking for a usable terminal.

## 13. Testing Strategy (Day 0)

All testing is Day 0. No exceptions. No testing is deferred.

**Unit tests:**
- **Shadow snapshot:** WAL replay, version tree CRUD, delta chain replay, GC invariants, crash-safe decline protocol
- **Mux protocol:** serialization round-trip for all message types, version negotiation logic
- **Grid sync:** generation counter logic, diff application, ring buffer overflow → full snapshot
- **Layout engine:** split tree operations, resize math, serialization/deserialization

**Integration test:**
daemon spawn → create session → spawn pane → type input → fetch grid update → verify content → split pane → detach → reattach → verify all panes rendered from authoritative snapshot → close session → verify daemon idle behavior

**Property-based testing:**
- Shadow snapshot crash recovery, WAL replay correctness, GC invariants (model checking)

**Protocol compatibility testing:**
- Mux protocol backward-compatibility (version negotiation, unknown field handling)

**Terminal emulation conformance:**
- vttest / esctest conformance suite against the retained alacritty terminal engine

**Extension sandbox testing:**
- Fuzz testing the QuickJS↔Rust FFI boundary

**Network fault injection:**
- Packet loss, latency, partition simulation for SSH transport

## 14. Known Limitations and Scope

**Foundation scope is comprehensive.** Day 0 includes everything in this spec except the items listed below. No testing, no terminal feature, no core subsystem is deferred.

**Only these are post-foundation:**
- Extension marketplace (local directory + CLI install is Day 0)
- Kitty graphics protocol / iTerm2 OSC 1337 (Sixel already in alacritty)
- Log viewer UI (file logs + status CLI + GPUI notifications are Day 0)

Everything else — including UDP resilient transport, all terminal product details (§12), all testing (§13), shadow snapshot, QuickJS, remote connection, clipboard, input routing — is Day 0.
**Editor crate pruning:** The editor crate's `read_only` mode is already battle-tested (Zed uses it for preview buffers). Pruning is surgical: delete editing-only and LSP-only modules (listed in §2.1), enforce `read_only = true` at construction, remove edit action handlers from `editor.rs`. This preserves the full tree-sitter highlighting, display map, folding, search, and diff rendering. The coupling with `project`/`buffer` is real but manageable: the file viewer constructs a `MultiBuffer` directly from a file path (the same path Zed's own file preview uses), without needing the project's LSP registry.

**Cherry-pick reality check:** After deleting 90 crates and rewriting files, cherry-picking upstream GPUI updates will be labor-intensive. GPUI is not a stable API; it evolves with Zed. The layered naming strategy helps but does not guarantee easy merges. This cost is accepted as a consequence of the hard fork decision.

**Keymap "compatible subset" risk:** A partial tmux keymap that violates muscle memory for the missing 20% may be worse than no keymap at all. Users who need exact tmux compatibility should use actual tmux inside z3rm. The built-in profiles target common operations only; documentation must be clear about the "compatible subset" limitation.

## 15. Hard Constraints and Performance SLOs

These are the load-bearing constraints that must hold for Day 0 to ship. They are not aspirational.

### 15.1 Authority Boundaries (who owns what)

1. **mux_server is the sole authority for pane/tab/layout state.** workspace is a projection and interaction shell — it holds no editable layout model of its own. All layout mutations go through MuxDomain.
2. **mux_server is the sole authority for terminal grid state.** The GUI client renders grid snapshots/diffs only.
3. **Foundation supports multi-client per session.** Multiple GUI windows can attach to the same session simultaneously. Control modes: `attach` (shared control, tmux default), `attach -d` (steal), `attach -r` (read-only). Layout uses min-fit resize (smallest client dimensions set pane sizes). Layout changes push to all attached clients. Input serializes to PTY by arrival order (interleaved output is user-caused, accepted).

### 15.2 Phase Boundaries

4. **Day 0 includes QuickJS extension host, shadow snapshot engine, and all foundation crates in §2.3.** Native chrome is the **primary implementation** (not fallback), running alongside the extension host. Chrome-as-extension is the target; native chrome is the Day 0 baseline that works even when extensions are loading or crashed. Shadow snapshot runs on the file's host machine (local or remote mux_server), providing fine-grained undo independent of git.
6. **Remote server auto-install.** Client SSHes to remote host, probes for `z3rm-server` via `command -v`. If not found: same-arch = scp local binary; different-arch = download from release server. Installed to `~/.z3rm-server/`. Version mismatch triggers reinstall (VS Code model). GUI-first: CLI is a convenience, not the primary interface.

### 15.3 Platform Abstraction

7. **Local IPC uses `interprocess` crate's local socket abstraction** (Unix domain socket on Unix, named pipe on Windows). No `UnixStream` in top-level types. Platform-specific code is behind this abstraction.
8. **Socket permissions:** Unix socket created with `0600` in `$XDG_RUNTIME_DIR/z3rm/`. Named pipe ACL restricts to current user SID. No same-machine cross-user session access.
9. **Windows ConPTY support** is available in alacritty's Windows backend (Zed already builds and runs on Windows with ConPTY). Real Windows CI runner required for behavioral testing; Wine for compilation only.

### 15.4 Notification and Reconnect Semantics

10. **Lifecycle events (`PaneAdded`, `PaneRemoved`, `SessionLayoutChanged`) use at-least-once delivery**, not at-most-once. Losing a `PaneRemoved` creates a zombie pane in the UI.
11. **Grid dirty signals (`PaneDirty`) use at-most-once delivery.** Missing a dirty signal is harmless — the next repaint polls `fetch_grid_update` and catches up.
12. **Attach/reattach returns a full authoritative session snapshot**: panes, tabs, layout tree, focused pane/tab, current generation per pane. Reconnect does not rely on missed push notifications; it reconciles from the authoritative snapshot.
13. **Generation increments on every render-affecting server-side pane state change** — not only PTY byte ingestion, but also cursor style, alternate screen switch, scroll offset, title update.
14. **DEC-2026 unpaired BSU timeout:** if server receives BSU but no matching ESU within 100ms, it force-flushes the generation bump. Prevents permanent display freeze from crashed TUI apps.
15. **Diff ring (64 entries) is a count of coalesced updates, not frames or time.** Under sustained high throughput, 64 updates may represent <1s of history. A client that falls behind gets a full snapshot. In remote high-latency scenarios this means more frequent full snapshots — acknowledged as a Day-0 limitation that UDP resilient transport would address.

### 15.5 Performance SLOs (Foundation)

These are minimum acceptable thresholds. "Benchmark first" means these numbers must be met before claiming foundation is done.

| Metric | SLO | Measurement |
|---|---|---|
| Local keystroke-to-glyph p95 | < 16ms (one frame at 60fps) | Type a char, measure time to glyph appearing on screen. The keystroke path is: key → `send_input` → server writes PTY → shell echoes → PTY output → emulator → generation bump → `PaneDirty` → client repaint → `fetch_grid_update` → render. |
| Local pane output steady-state | > 50 MB/s sustained throughput | `cat /dev/urandom > /dev/null` equivalent (large synthetic output). Measured as bytes processed by emulator per wall-clock second. |
| Cold start to first shell prompt | < 500ms | `z3rm` launch → daemon spawn → socket connect → spawn shell → first prompt visible. |
| Full snapshot resync (80x24 grid) | < 5ms | Time from `fetch_grid_update` returning `FullSnapshot` to render complete. |
| Reattach to interactive | < 200ms | From `attach()` call to all panes rendered from authoritative snapshot. |

If these SLOs are not met, the architecture (not just the implementation) must be revisited. Specifically, local keystroke latency > 16ms would indicate the IPC round-trip is fundamentally too slow and a local fast-path (shared memory for grid, not PTY bytes) must be reconsidered.

### 15.6 Server Single Point of Failure

**mux_server is the single point of failure for the entire system.** If it crashes, all PTYs receive SIGHUP and die, all grid state is lost, and all sessions end. The daemon holds: PTY fds, alacritty emulator instances, grid state, scrollback, layout metadata, protocol codec, diff ring. This concentration of responsibility is the direct cost of server-canonical design — it trades distributed resilience for state consistency. There is no "rare" about it: the server must be stable, because its crash kills everything.

Mitigation: layout metadata survives in SQLite (crash recovery restores layout + shells, not processes or grid). Process keepalive only protects against **client** crashes/disconnects, not server crashes. The server process must be rigorously tested and should not carry experimental subsystems.

### 15.7 Core Interaction Independence

**No core interaction capability may be reachable only through extension chrome.** Even if QuickJS is not started, crashed, or fuel-limited, users must be able to: split pane, switch pane, create/close tab, attach/detach session, open settings, kill server/session. These commands have native keybindings that do not depend on the extension host.

### 15.8 License Accuracy

**GPUI and related `gpui_*` crates are Apache-2.0, not GPL-3.0.** Zed's codebase is "primarily GPL-3.0-or-later, with Apache-2.0 components where marked." GPUI is explicitly Apache-2.0. The licensing table in §10 must reflect this per-crate reality, not blanket "inherited GPL." The `cargo-about` / `REUSE.toml` audit in CI must check each crate's actual `LICENSE` file, not assume by inheritance.

## 16. Resolved Decisions (Grill Session)

All decisions below were resolved through structured grilling and are Day 0 binding.

### 16.1 Daemon Lifecycle

- **Auto-spawn:** GUI startup checks for daemon; if not running, spawns `z3rm-server --daemonize`. If running, connects directly.
- **Socket:** Fixed path default (`$XDG_RUNTIME_DIR/z3rm/mux.sock` on Unix, `\\.\pipe\z3rm-mux` on Windows). `--session <name>` uses named socket. Connect timeout configurable (default 500ms).
- **Stale socket:** Connect timeout → do NOT delete socket → spawn new daemon → new daemon detects conflict → reports error or takes over.
- **Idle resource:** Zero-pane daemon enters passive idle mode (minimal CPU/memory).
- **keep_alive:** Default `true`. Daemon stays until explicit `z3rm-server kill`. Configurable `keep_alive_seconds` for auto-exit.
- **One daemon, multiple sessions:** Each session has independent cwd/env. `--session <name>` attaches to existing or creates new in running daemon.
- **Crash recovery:** Restore layout + shells at recorded cwd. Commands never auto-re-run.

### 16.2 Multi-Client

- **Shared control (tmux default):** All attached clients can input, resize, split.
- **Control modes:** `attach` (shared), `attach -d` (steal), `attach -r` (read-only).
- **Layout:** Min-fit resize (smallest client dimensions constrain pane sizes).
- **Layout propagation:** Any layout mutation → `SessionLayoutChanged` push to all attached clients.
- **Input serialization:** `send_input` calls arrive at server in order; PTY writes are serial. Interleaved output from concurrent typing is user-caused, accepted.

### 16.3 Grid Sync Protocol

- **Row-level diff:** `TerminalGridDiff = Vec<RowChange { row: usize, cells: Vec<Cell> }>`. Aligned with alacritty's internal `dirty_lines` damage tracking. Zero extra computation.
- **Generation counter:** Per-pane, increments on ALL render-affecting changes (PTY output, cursor style, alt-screen, scroll offset, title).
- **Push + pull:** `PaneDirty(PaneId)` push (at-most-once, lightweight wakeup) → client schedules repaint → `fetch_grid_update(pane, since_generation)` → returns `GridUpdate { from_gen, to_gen, Diff | FullSnapshot }`.
- **Ring buffer:** 64 coalesced updates per pane (not frames, not time). Client behind oldest entry → FullSnapshot.
- **Adaptive coalescing:** Interactive (0ms, keyboard active + <4KB/s) / Normal (2ms) / High-throughput (8-16ms, >100KB/s sustained).
- **DEC-2026:** Server parses BSU/ESU. Unpaired BSU timeout: 100ms → force-flush generation bump.

### 16.4 Scrollback

- **Per-client scroll** (default): each client has independent scroll offset. Users can switch to **session-wide sync scroll** (collaborative same-screen) via keybinding. Both modes are Day 0.
- **New output behavior:** Auto-jump to bottom on new PTY output when in scrollback. User can lock to stay in scrollback.
- **Fetch protocol:** `fetch_scrollback(pane, from_line, direction, count)` → `ScrollbackChunk { lines: Vec<Row>, total_lines: usize, scrollback_version }`.
- **Client cache:** Local cache of fetched scrollback lines. Cache invalidation: `scrollback_version` = u64 counter + Unix timestamp (overflow-safe). Version mismatch → clear cache → re-fetch.

### 16.5 Input Routing

- **Priority chain:** `IME composition → extension global shortcut → prefix mode → copy/scrollback mode → terminal application (PTY)`.
- **Prefix key:** Determined by keymap profile. Default profile: no prefix (global shortcuts only). `tmux` profile: `Ctrl-b`.
- **Nesting passthrough:** When terminal application enables any full-screen signal (alternate screen `ESC[?1049h`, bracketed paste `ESC[?2004h`, mouse tracking `ESC[?1000-1006h`, or any DECSET private mode), prefix key passes through to PTY.
- **Double-tap passthrough:** Press prefix key twice → send literal prefix key to PTY.
- **Agent CLI full-screen:** Claude Code / similar agents in full-screen mode → prefix passthrough. This is expected behavior, documented in user guide.

### 16.6 File Viewer, Diff, and Layout

- **Entry points:** (a) file tree sidebar click, (b) command palette, (c) terminal output path detection (clickable paths).
- **Default behavior:** Single center terminal pane + click → auto-split-right pane opens file viewer/diff.
- **Layout model:**
  - Left dock: stacked tabbar (each pane = one tab, vertically stacked)
  - Center: terminal panes with top tabbar
  - Right: normal panes (top tabbar) for file tree / diff view — supports split, drag-resize, multi-tab, same interaction model as terminal panes
  - Tabbar style (top vs stacked) is runtime-switchable, not hardcoded
- **File access:** Reuses Zed worktree/RemoteConnection abstraction. Backend connects to mux_protocol `read_file` / `list_dir` / `stat_file` RPCs for remote files.
- **Diff baseline:** Shadow snapshot version tree (Day 0, works with or without git).

### 16.7 Shadow Snapshot (Day 0, Full §4 Implementation)

- Runs on the file's host machine: local mux_server for local files, remote mux_server for remote files.
- Full implementation: version tree, WAL, SQLite, bounded delta chain ($D_{max}=16$), Rope-level delta replay, binary lifting LCA, frequency circuit breaker, crash-safe decline, quota GC with promote-to-full, Zstd compression, content-addressed blob store.
- Git commit hook integration and `.z3rmignore` are Day 0.

### 16.8 Remote Connection

- **GUI-first.** CLI is a convenience add-on, not the primary interface.
- **Auto-install:** Client probes remote via `command -v z3rm-server`. Not found → same-arch: scp local binary. Different-arch: download from release server. Version mismatch → reinstall. Installed to `~/.z3rm-server/`.
- **Extension sync:** On connect + on install. Client pushes extension manifests + source to server. Server loads server-side extensions.
- **Extension runtime declaration:** `extension.toml` `[runtime] side = "server" | "client" | "both"` + `sync = true | false`. Server-side extensions access remote PTY/grid/filesystem; chrome output returns to client via mux_protocol RPC. Client-side extensions render chrome UI directly via GPUI.

### 16.9 Layout System (Workspace Migration)

- **pane_group geometry logic migrates entirely to mux_server.** Split tree, resize math, size ratios — all server-side.
- **Client workspace = layout renderer + GPUI item container.** Receives layout snapshot → creates GPUI views at server-specified positions/sizes → forwards user interactions (drag, resize) as RPCs to server.
- **Client workspace holds no layout calculation logic.** It is a stateless layout renderer.

### 16.10 Clipboard

- **Server = relay hub.** Canonical clipboard space on mux_server, supports text / image / file path (with `origin_host` metadata).
- **OSC 52:** Server emulator parses OSC 52 → writes to server clipboard → pushes to clients.
- **Bracketed paste:** Server emulator tracks bracketed paste mode; `paste()` RPC content wrapped by server if needed.
- **Multi-client:** No automatic mirror. Only actual copy actions (OSC 52, user Ctrl-C/Ctrl-Shift-C) trigger sync. Prevents clipboard pollution.
- **Path forwarding:** Clipboard entries with file paths carry `origin_host`. Client opens path via local file system (local origin) or mux_protocol `fetch_file` (remote origin). Image paths are resolved on their origin machine.

### 16.11 Settings Hot Reload

- **Client-side settings** (font, theme, keymap, chrome config): watched and hot-reloaded by client.
- **Server-side settings** (scrollback limit, PTY behavior, keep_alive, quota config): watched and hot-reloaded by server. Local daemon watches same file. Remote daemon watches its own file on the remote machine.

### 16.12 Extension Installation

- **Local directory scan:** `~/.config/z3rm/extensions/` scanned on startup.
- **CLI install:** `z3rm extension install <path-or-git-url>` copies/clones to extensions directory.
- **No marketplace in Day 0.** Marketplace is post-foundation.

### 16.13 Testing (Day 0)

- **Unit tests:** shadow_snapshot (WAL replay, version tree CRUD, delta chain replay, GC invariants), mux_protocol (serialization round-trip, version negotiation), grid sync (generation counter logic, diff application).
- **End-to-end integration test:** daemon spawn → create session → spawn pane → type input → fetch grid → split pane → detach → reattach → verify state matches.

### 16.14 Logging and Diagnostics

- **File logs:** `~/.local/share/z3rm/logs/mux-server.log` (with rotation).
- **Status CLI:** `z3rm-server status` prints uptime, session count, pane count, memory usage.
- **GPUI notifications:** daemon issues push to client as toast/error notifications for critical problems.
