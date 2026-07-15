# Plan 21: Input Routing — Priority Chain

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Implement the input routing priority chain: IME → extension → prefix mode → copy/scrollback mode → terminal application (PTY). Full-screen app passthrough. Double-tap literal. Agent CLI passthrough.

**Dependencies:** `gpui`, `mux`, keymap profiles (Plan 17).

---

### Task 1: Implement priority chain dispatcher

**Files:**
- Create: `crates/z3rm/src/input.rs`

- [ ] **Step 1: Implement key event handler**

```rust
fn handle_key_event(key: &KeyEvent, cx: &mut Context<TerminalView>) -> KeyDispatchResult {
    // 1. IME composing?
    if cx.is_ime_composing() {
        return KeyDispatchResult::RouteToIme;
    }
    
    // 2. Extension global shortcut?
    if let Some(action) = extension_host::match_global_shortcut(key) {
        return KeyDispatchResult::ExecuteAction(action);
    }
    
    // 3. Prefix mode active?
    if prefix_mode::is_active() {
        return prefix_mode::handle_key(key);
    }
    
    // 4. Full-screen app passthrough?
    if terminal_app::is_full_screen_active(domain) {
        // Check double-tap prefix passthrough
        if is_double_tap_prefix(key) {
            return KeyDispatchResult::SendLiteral(key.to_byte());
        }
        return KeyDispatchResult::SendToPty(key.to_bytes());
    }
    
    // 5. Copy/scrollback mode?
    if copy_mode::is_active() {
        return copy_mode::handle_key(key);
    }
    
    // 6. Terminal application
    KeyDispatchResult::SendToPty(key.to_bytes())
}
```

---

### Task 2: Full-screen app detection

**Files:**
- Create: `crates/z3rm/src/terminal_app.rs`

- [ ] **Step 1: Detect alt screen, bracketed paste, mouse tracking, DECSET modes**

Query server for current pane mode state (included in `PaneInfo` or via a `GetPaneModes` RPC).

```rust
fn is_full_screen_active(domain: &MuxDomain, pane: &str) -> bool {
    let modes = domain.get_pane_modes(pane);
    modes.alt_screen || modes.bracketed_paste || modes.mouse_tracking || modes.any_decset
}
```

---

### Task 3: Prefix mode state machine

**Files:**
- Create: `crates/z3rm/src/prefix_mode.rs`

- [ ] **Step 1: Implement enter/exit prefix mode**

On prefix key press → enter PrefixMode context. Set timeout (configurable, default 500ms). On timeout → exit prefix mode without action.

- [ ] **Step 2: Match next key against prefix bindings**

If match → execute action → exit prefix mode.
If double-tap prefix → send literal prefix to PTY → exit prefix mode.
If no match → send prefix key + this key to PTY → exit prefix mode.

---

### Task 4: IME integration

**Files:**
- Modify: `crates/terminal_view/src/terminal_view.rs`

- [ ] **Step 1: Route GPUI IME composition to PTY**

GPUI provides IME composition events. When IME commits text → send committed text to PTY via `send_input`. During composition → don't send partial input.

---

### Task 5: Tests + Commit

- [ ] **Step 1: Priority chain test — each layer intercepts correctly**

- [ ] **Step 2: Full-screen passthrough test**

- [ ] **Step 3: Double-tap literal test**

- [ ] **Step 4: IME composition test (requires IME framework)**

- [ ] **Step 5: Commit**

```bash
git add crates/z3rm/src/input.rs crates/z3rm/src/prefix_mode.rs crates/z3rm/src/terminal_app.rs
git commit -m "Add input routing: priority chain, prefix mode, full-screen passthrough, IME"
```
