# Plan 17: Keymap Profiles

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Implement keymap profiles (default, tmux, zellij, screen) as bundled keymap files. Default profile uses global shortcuts (no prefix). tmux/zellij/screen profiles provide a compatible subset of bindings.

**Architecture:** Zed's keymap engine (complex key sequences, context-aware bindings, modal tables) is retained. Profiles are JSON keymap files in `assets/keymaps/`. User selects via `mux.keymap_profile` setting.

**Dependencies:** `settings`, `assets/keymaps/`.

---

### Task 1: Create default keymap profile

**Files:**
- Create: `assets/keymaps/default.json`

- [ ] **Step 1: Define global shortcuts (no prefix)**

```json
[
    {"context": ["Terminal"], "bindings": {
        "ctrl-shift-t": "pane::newTab",
        "ctrl-shift-w": "pane::closeTab",
        "ctrl-shift-%": "pane::splitRight",
        "ctrl-shift-\"": "pane::splitDown",
        "ctrl-shift-left": "pane::focusLeft",
        "ctrl-shift-right": "pane::focusRight",
        "ctrl-shift-up": "pane::focusUp",
        "ctrl-shift-down": "pane::focusDown",
        "ctrl-shift-d": "mux::detach",
        "ctrl-shift-z": "pane::zoomToggle"
    }}
]
```

---

### Task 2: Create tmux keymap profile

**Files:**
- Create: `assets/keymaps/tmux.json`

- [ ] **Step 1: Define prefix key (Ctrl-b) + prefix-mode bindings**

```json
[
    {"context": ["Terminal"], "bindings": {
        "ctrl-b": ["mux::enterPrefixMode", {"timeout": 500}]
    }},
    {"context": ["Terminal", "PrefixMode"], "bindings": {
        "c": "pane::newTab",
        "%": "pane::splitRight",
        "\"": "pane::splitDown",
        "n": "pane::nextTab",
        "p": "pane::prevTab",
        "d": "mux::detach",
        "0": "pane::focusPane0",
        "1": "pane::focusPane1",
        "left": "pane::focusLeft",
        "right": "pane::focusRight",
        "up": "pane::focusUp",
        "down": "pane::focusDown",
        "ctrl-b": "terminal::sendLiteral Ctrl-b"
    }}
]
```

- [ ] **Step 2: Document the compatible subset**

Cover: prefix, split, tab switch, pane navigation, detach, pane numbering. ~80% of common tmux operations. Document explicitly that this is NOT 1:1 tmux compatibility.

---

### Task 3: Create zellij keymap profile

**Files:**
- Create: `assets/keymaps/zellij.json`

- [ ] **Step 1: Define zellij-style bindings (Alt-prefix)**

Cover zellij's common operations: Alt-n (new tab), Alt-h/j/k/l (pane nav), Alt-d (detach), Alt-+ (split).

---

### Task 4: Create screen keymap profile

**Files:**
- Create: `assets/keymaps/screen.json`

- [ ] **Step 1: Define screen-style bindings (Ctrl-a prefix)**

---

### Task 5: Implement prefix mode

**Files:**
- Modify: `crates/z3rm/src/input.rs` (or new module)

- [ ] **Step 1: Implement prefix mode state machine**

User presses prefix key → enter PrefixMode context → next key matched against prefix bindings → execute or passthrough (double-tap = literal).

- [ ] **Step 2: Implement full-screen app passthrough detection**

When terminal application enables alt screen / bracketed paste / mouse tracking / DECSET → prefix key passthrough to PTY.

---

### Task 6: Profile switching

- [ ] **Step 1: Implement setting-driven profile loading**

`mux.keymap_profile = "tmux"` → load `assets/keymaps/tmux.json` as the active keymap.

- [ ] **Step 2: Runtime switching**

Change setting → reload keymap without restart.

---

### Task 7: Tests + Commit

- [ ] **Step 1: Prefix mode state machine test**

- [ ] **Step 2: Full-screen passthrough detection test**

- [ ] **Step 3: Commit**

```bash
git add assets/keymaps/ crates/z3rm/src/input.rs
git commit -m "Add keymap profiles: default, tmux, zellij, screen + prefix mode"
```
