# Plan 9: mux_protocol — Wire Protocol Crate

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Define the prost/protobuf wire protocol between z3rm client and mux_server. Versioned from day one. Covers: session lifecycle, pane lifecycle, grid sync (generation counter), scrollback fetch, file fetch, clipboard, extension chrome RPC.

**Architecture:** prost-based protobuf messages over framed binary (length-prefixed). Protocol version in every message header. All types in `mux_protocol` crate, shared by both client and server.

**Tech Stack:** prost, prost-build, prost-types.

---

### Task 1: Create crate skeleton

**Files:**
- Create: `crates/mux_protocol/Cargo.toml`
- Create: `crates/mux_protocol/src/mux_protocol.rs`
- Create: `crates/mux_protocol/build.rs`
- Create: `crates/mux_protocol/proto/mux.proto`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "mux_protocol"
version = "0.1.0"
edition = "2024"
publish = false
license = "Apache-2.0"

[lib]
path = "src/mux_protocol.rs"

[dependencies]
prost = { workspace = true }
prost-types = { workspace = true }

[build-dependencies]
prost-build = { workspace = true }
```

- [ ] **Step 2: Create build.rs**

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::compile_protos(&["proto/mux.proto"], &["proto/"])?;
    Ok(())
}
```

- [ ] **Step 3: Create the protobuf definition**

This is the wire protocol contract. Every message has a protocol version. Field numbers are never reused.

```protobuf
syntax = "proto3";
package z3rm.mux;

// Protocol version — bumped on breaking changes.
// Minor additions use new field numbers (forward-compatible).
// Major version bumps require session restart.
message ProtocolVersion {
    uint32 major = 1;
    uint32 minor = 2;
}

// Every message envelope wraps a protocol version.
message Envelope {
    ProtocolVersion version = 1;
    oneof payload {
        Request request = 2;
        Response response = 3;
        Notification notification = 4;
    }
}

// === Session Lifecycle ===

message SessionInfo {
    string id = 1;
    string name = 2;
    string cwd = 3;
    uint64 created_timestamp = 4;
    uint32 attached_clients = 5;
}

message CreateSessionRequest {
    string name = 1;
    string cwd = 2;
}

message ListSessionsRequest {}

message ListSessionsResponse {
    repeated SessionInfo sessions = 1;
}

message KillSessionRequest {
    string id = 1;
}

message AttachRequest {
    string session_id = 1;
    AttachMode mode = 2;
    
    enum AttachMode {
        ATTACH_MODE_UNSPECIFIED = 0;
        SHARED = 1;
        STEAL = 2;
        READ_ONLY = 3;
    }
}

message AttachResponse {
    SessionSnapshot snapshot = 1;
}

// Full authoritative snapshot returned on attach/reattach.
message SessionSnapshot {
    repeated TabInfo tabs = 1;
    LayoutTree layout = 2;
    string focused_pane_id = 3;
    string focused_tab_id = 4;
    string session_id = 5;
}

message TabInfo {
    string id = 1;
    string title = 2;
    repeated PaneInfo panes = 3;
}

message PaneInfo {
    string id = 1;
    string cwd = 2;
    string title = 3;
    string command = 4;
    uint64 generation = 5;  // current grid generation
    TerminalSize size = 6;
    bool is_alive = 7;
}

message TerminalSize {
    uint32 cols = 1;
    uint32 rows = 2;
}

message DetachRequest {}

// === Layout Tree ===

message LayoutTree {
    LayoutNode root = 1;
}

message LayoutNode {
    string id = 1;
    oneof node {
        PaneLeaf pane = 2;
        SplitNode split = 3;
    }
}

message PaneLeaf {
    string pane_id = 1;
}

message SplitNode {
    SplitDirection direction = 1;
    repeated LayoutNode children = 2;
    repeated float ratios = 3;  // size ratios for each child
    
    enum SplitDirection {
        SPLIT_DIRECTION_UNSPECIFIED = 0;
        LEFT_RIGHT = 1;
        TOP_BOTTOM = 2;
    }
}

// === Pane Lifecycle ===

message SpawnPaneRequest {
    string session_id = 1;
    string tab_id = 2;
    TerminalSize size = 3;
    optional ShellCommand command = 4;
    optional string cwd = 5;
}

message ShellCommand {
    string program = 1;
    repeated string args = 2;
    map<string, string> env = 3;
}

message SplitPaneRequest {
    string pane_id = 1;
    LayoutNode.SplitNode.SplitDirection direction = 2;
}

message ClosePaneRequest {
    string pane_id = 1;
}

message FocusPaneRequest {
    string pane_id = 1;
}

message ResizePaneRequest {
    string pane_id = 1;
    uint32 cols = 2;
    uint32 rows = 3;
}

// === Input ===

message SendInputRequest {
    string pane_id = 1;
    bytes data = 2;
}

message PasteRequest {
    string pane_id = 1;
    string text = 2;
}

// === Grid Sync (generation counter, pull-based) ===

message FetchGridUpdateRequest {
    string pane_id = 1;
    uint64 since_generation = 2;
}

message FetchGridUpdateResponse {
    uint64 from_generation = 1;
    uint64 to_generation = 2;
    oneof update {
        GridDiff diff = 3;
        FullGridSnapshot full_snapshot = 4;
    }
}

message GridDiff {
    repeated RowChange rows = 1;
}

message RowChange {
    uint32 row = 1;
    repeated Cell cells = 2;
}

message Cell {
    string char = 1;
    CellStyle style = 2;
    uint32 foreground = 3;  // 0xRRGGBB
    uint32 background = 4;  // 0xRRGGBB
}

message CellStyle {
    bool bold = 1;
    bool italic = 2;
    bool underline = 3;
    bool strikethrough = 4;
    bool dim = 5;
    bool reverse = 6;
}

message FullGridSnapshot {
    uint32 cols = 1;
    uint32 rows = 2;
    repeated Cell cells = 3;  // flat row-major: cells[row * cols + col]
    CursorState cursor = 4;
    bool alternate_screen = 5;
}

message CursorState {
    uint32 col = 1;
    uint32 row = 2;
    CursorStyle style = 3;
    bool visible = 4;
    
    enum CursorStyle {
        CURSOR_STYLE_UNSPECIFIED = 0;
        BLOCK = 1;
        BAR = 2;
        UNDERLINE = 3;
    }
}

// === Scrollback ===

message FetchScrollbackRequest {
    string pane_id = 1;
    uint32 from_line = 2;
    uint32 direction = 3;  // 0 = up, 1 = down
    uint32 count = 4;
}

message FetchScrollbackResponse {
    repeated RowChange lines = 1;
    uint32 total_lines = 2;
    uint64 scrollback_version = 3;  // counter + timestamp for cache invalidation
}

// === File Fetch (for remote file viewer) ===

message ReadFileRequest {
    string path = 1;
    optional uint32 offset_line = 2;
    optional uint32 max_lines = 3;
}

message ReadFileResponse {
    bytes content = 1;
    bool is_binary = 2;
    string encoding = 3;  // "utf-8", "binary", etc.
}

message ListDirRequest {
    string path = 1;
}

message DirEntry {
    string name = 1;
    bool is_dir = 2;
    uint64 size = 3;
    bool is_modified = 4;  // shadow snapshot detected change
}

message ListDirResponse {
    repeated DirEntry entries = 1;
}

message StatFileRequest {
    string path = 1;
}

message StatFileResponse {
    bool exists = 1;
    uint64 size = 2;
    bool is_dir = 3;
    uint64 modified_timestamp = 4;
}

// === Clipboard (server relay hub) ===

message ClipboardEntry {
    ClipboardContentType content_type = 1;
    bytes data = 2;
    string origin_host = 3;  // which machine this came from
    
    enum ClipboardContentType {
        CONTENT_TYPE_UNSPECIFIED = 0;
        TEXT = 1;
        IMAGE_PNG = 2;
        FILE_PATH = 3;
    }
}

message SetClipboardRequest {
    ClipboardEntry entry = 1;
}

message GetClipboardRequest {}

message GetClipboardResponse {
    ClipboardEntry entry = 1;
}

// === Extension Chrome RPC ===

message ExtensionChromeUpdate {
    string extension_id = 1;
    string view_id = 2;
    bytes vdom_payload = 3;  // serialized VDOM JSON
}

// === Notifications (server → client push) ===

message Notification {
    oneof event {
        PaneDirty pane_dirty = 1;
        PaneAdded pane_added = 2;
        PaneRemoved pane_removed = 3;
        PaneFocused pane_focused = 4;
        TabTitleChanged tab_title_changed = 5;
        SessionLayoutChanged session_layout_changed = 6;
        ClipboardChanged clipboard_changed = 7;
        ExtensionChromeUpdate extension_chrome = 8;
    }
}

message PaneDirty {
    string pane_id = 1;
}

message PaneAdded {
    string pane_id = 1;
    string tab_id = 2;
}

message PaneRemoved {
    string pane_id = 1;
    int32 exit_code = 2;
}

message PaneFocused {
    string pane_id = 1;
}

message TabTitleChanged {
    string tab_id = 1;
    string title = 2;
}

message SessionLayoutChanged {
    LayoutTree layout = 1;
}

message ClipboardChanged {}

// === Request/Response envelope ===

message Request {
    uint64 request_id = 1;
    oneof body {
        CreateSessionRequest create_session = 2;
        ListSessionsRequest list_sessions = 3;
        KillSessionRequest kill_session = 4;
        AttachRequest attach = 5;
        DetachRequest detach = 6;
        SpawnPaneRequest spawn_pane = 7;
        SplitPaneRequest split_pane = 8;
        ClosePaneRequest close_pane = 9;
        FocusPaneRequest focus_pane = 10;
        ResizePaneRequest resize_pane = 11;
        SendInputRequest send_input = 12;
        PasteRequest paste = 13;
        FetchGridUpdateRequest fetch_grid_update = 14;
        FetchScrollbackRequest fetch_scrollback = 15;
        ReadFileRequest read_file = 16;
        ListDirRequest list_dir = 17;
        StatFileRequest stat_file = 18;
        SetClipboardRequest set_clipboard = 19;
        GetClipboardRequest get_clipboard = 20;
    }
}

message Response {
    uint64 request_id = 1;
    oneof body {
        string error = 2;  // non-empty = error
        SessionInfo session = 3;
        ListSessionsResponse sessions = 4;
        AttachResponse attach = 5;
        string pane_id = 6;
        FetchGridUpdateResponse grid_update = 7;
        FetchScrollbackResponse scrollback = 8;
        ReadFileResponse file_content = 9;
        ListDirResponse dir_listing = 10;
        StatFileResponse file_stat = 11;
        GetClipboardResponse clipboard = 12;
    }
}
```

- [ ] **Step 4: Create lib.rs**

```rust
pub mod gen {
    include!(concat!(env!("OUT_DIR"), "/z3rm.mux.rs"));
}

pub use gen::*;

/// Current protocol version.
pub const PROTOCOL_VERSION: gen::ProtocolVersion = gen::ProtocolVersion {
    major: 1,
    minor: 0,
};

/// Frame a message as length-prefixed binary.
pub fn frame(msg: &Envelope) -> Result<Vec<u8>, prost::EncodeError> {
    let mut buf = Vec::with_capacity(msg.encoded_len() + 4);
    (msg.encoded_len() as u32).encode(&mut buf)?;
    msg.encode(&mut buf)?;
    Ok(buf)
}

/// Decode a framed message. Returns (message, bytes_consumed).
pub fn unframe(buf: &[u8]) -> Result<(Envelope, usize), prost::DecodeError> {
    if buf.len() < 4 {
        return Err(prost::DecodeError::new("buffer too short for frame header"));
    }
    let mut cur = std::io::Cursor::new(&buf[..4]);
    let len = u32::decode(&mut cur)? as usize;
    if buf.len() < 4 + len {
        return Err(prost::DecodeError::new("incomplete frame"));
    }
    let msg = Envelope::decode(&buf[4..4 + len])?;
    Ok((msg, 4 + len))
}
```

Add `prost` encoding trait to imports:
```rust
use prost::Message;
```

- [ ] **Step 5: Add to workspace**

Add `"crates/mux_protocol"` to workspace members. Add `mux_protocol = { path = "crates/mux_protocol" }` to workspace.dependencies.

- [ ] **Step 6: Verify compilation**

Run: `cargo check -p mux_protocol`
Expected: PASS

- [ ] **Step 7: Write unit test — serialization round-trip**

**Files:**
- Create: `crates/mux_protocol/tests/round_trip.rs`

```rust
use mux_protocol::*;
use prost::Message;

#[test]
fn test_grid_diff_round_trip() {
    let diff = GridDiff {
        rows: vec![RowChange {
            row: 5,
            cells: vec![Cell {
                char: "H".into(),
                style: Some(CellStyle {
                    bold: true,
                    ..Default::default()
                }).into(),
                foreground: 0xFFFFFF,
                background: 0x000000,
            }],
        }],
    };
    
    let mut buf = Vec::new();
    diff.encode(&mut buf).unwrap();
    let decoded = GridDiff::decode(buf.as_slice()).unwrap();
    assert_eq!(decoded.rows.len(), 1);
    assert_eq!(decoded.rows[0].row, 5);
}

#[test]
fn test_frame_unframe_round_trip() {
    let env = Envelope {
        version: Some(PROTOCOL_VERSION.into()),
        payload: Some(gen::envelope::Payload::Notification(Notification {
            event: Some(gen::notification::Event::PaneDirty(PaneDirty {
                pane_id: "w1:p1".into(),
            })),
        })),
    };
    
    let framed = frame(&env).unwrap();
    let (decoded, consumed) = unframe(&framed).unwrap();
    assert_eq!(consumed, framed.len());
    assert!(matches!(decoded.payload, Some(gen::envelope::Payload::Notification(_))));
}

#[test]
fn test_full_snapshot_serialization() {
    let snap = FullGridSnapshot {
        cols: 80,
        rows: 24,
        cells: vec![Cell {
            char: " ".into(),
            style: None,
            foreground: 0,
            background: 0,
        }; 80 * 24],
        cursor: Some(CursorState {
            col: 0,
            row: 0,
            style: gen::cursor_state::CursorStyle::Block.into(),
            visible: true,
        }).into(),
        alternate_screen: false,
    };
    
    let mut buf = Vec::new();
    snap.encode(&mut buf).unwrap();
    let decoded = FullGridSnapshot::decode(buf.as_slice()).unwrap();
    assert_eq!(decoded.cols, 80);
    assert_eq!(decoded.rows, 24);
    assert_eq!(decoded.cells.len(), 80 * 24);
}
```

- [ ] **Step 8: Run tests**

Run: `cargo test -p mux_protocol`
Expected: 3 tests PASS

- [ ] **Step 9: Commit**

```bash
git add crates/mux_protocol Cargo.toml
git commit -m "Add mux_protocol crate with prost wire protocol"
```
