# Plan 18: File Viewer & Diff Review

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Wire up the pruned read-only editor crate as a file viewer with diff review. Entry points: file tree click, command palette, terminal path detection. Default: auto-split-right. Remote files supported via mux_protocol worktree abstraction.

**Architecture:** Editor crate constructs MultiBuffer from file path (no LSP needed). Diff view uses buffer_diff. Worktree abstraction backed by mux_protocol RPCs for remote files.

**Dependencies:** `editor` (pruned), `multi_buffer`, `buffer_diff`, `project`, `worktree`, `project_panel`, `mux`, `shadow_snapshot`.

---

### Task 1: Wire file tree click to editor readonly

**Files:**
- Modify: `crates/project_panel/src/project_panel.rs`

- [ ] **Step 1: On file click → create editor in readonly mode**

```rust
fn open_file_readonly(path: &Path, cx: &mut Context<Self>) {
    let buffer = MultiBuffer::from_file(path, cx);
    let editor = Editor::new_readonly(buffer, cx);
    // Auto-split-right if only center terminal pane exists
    workspace.open_pane_right(editor, cx);
}
```

---

### Task 2: Wire terminal path detection

**Files:**
- Modify: `crates/terminal_view/src/terminal_view.rs`

- [ ] **Step 1: Reuse Zed's terminal hyperlink detection**

Zed's terminal_view already has hyperlink detection (OSC 8, regex-based path detection). Retain this. On click → open file viewer.

---

### Task 3: Wire command palette entry

- [ ] **Step 1: Add `file::openDiff` command**

Opens file picker → select file → diff view (current vs shadow snapshot previous version).

---

### Task 4: Remote file access via worktree abstraction

**Files:**
- Modify: `crates/worktree/src/worktree.rs` (or new remote backend)

- [ ] **Step 1: Implement RemoteWorktreeBackend**

```rust
impl WorktreeBackend for RemoteWorktreeBackend {
    fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        self.domain.read_file(path).wait()  // via mux_protocol RPC
    }
    fn list_dir(&self, path: &Path) -> Result<Vec<DirEntry>> {
        self.domain.list_dir(path).wait()
    }
}
```

- [ ] **Step 2: Auto-detect local vs remote**

If session is connected to remote mux_server (SSH transport) → use RemoteWorktreeBackend. If local → use existing LocalWorktreeBackend.

---

### Task 5: Diff review with accept/decline

**Files:**
- Create: `crates/z3rm/src/diff_review.rs` (or in editor crate)

- [ ] **Step 1: Implement diff view (side-by-side using split.rs)**

Left pane: previous version (shadow snapshot). Right pane: current file content. Red/green highlighting via buffer_diff.

- [ ] **Step 2: Implement Accept button**

Accept = dismiss diff view. File stays at current version.

- [ ] **Step 3: Implement Decline button**

Decline = trigger shadow_snapshot decline protocol (§4.8). File reverts to previous version. Diff view updates to show reverted state.

---

### Task 6: Tests + Commit

- [ ] **Step 1: File viewer opens and renders with syntax highlighting**

- [ ] **Step 2: Diff view shows correct changes**

- [ ] **Step 3: Decline reverts file content**

- [ ] **Step 4: Remote file read works via mux_protocol**

- [ ] **Step 5: Commit**

```bash
git add crates/editor/ crates/project_panel/ crates/terminal_view/ crates/z3rm/src/diff_review.rs
git commit -m "Wire file viewer, diff review, remote file access via mux_protocol"
```
