# Plan 30: Log Viewer UI

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Implement log viewer UI: GPUI-based log file browser with filtering, search, and real-time tail. Day 0 already has file logs + status CLI + GPUI notifications; this adds a dedicated UI.

**Dependencies:** `gpui`, `zlog`, `settings_ui`.

**Spec:** §16.12 Logging & Diagnostics (post-foundation)

---

### Task 1: Log file reading

**Files:**
- Create: `crates/z3rm/src/log_viewer.rs`

- [ ] **Step 1: Implement log file tail**

Read `~/.local/share/z3rm/logs/mux-server.log` with tail capability.

- [ ] **Step 2: Log entry parsing**

Parse timestamp, level, message from log lines.

---

### Task 2: GPUI log viewer

**Files:**
- Modify: `crates/z3rm/src/log_viewer.rs`

- [ ] **Step 1: Implement log list view**

Scrollable list of log entries with color-coded levels.

- [ ] **Step 2: Implement filtering**

Filter by level (ERROR, WARN, INFO, DEBUG).

- [ ] **Step 3: Implement search**

Search log entries by text.

- [ ] **Step 4: Real-time tail**

Auto-refresh when new log entries are written.

---

### Task 3: Command palette integration

- [ ] **Step 1: Add `z3rm: open log viewer` command**

---

### Task 4: Tests + Commit

- [ ] `cargo check` passes
- [ ] Commit + push
