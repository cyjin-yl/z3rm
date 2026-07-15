# Plan 3: AGENTS.md & .rules Rewrite

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Rewrite AGENTS.md for z3rm (preserve GPUI guidelines, delete Zed editor/agent specifics, add mux/terminal/extension guidelines). Create CLAUDE.md as symlink to AGENTS.md. Update .rules.

**Architecture:** AGENTS.md is the single source of truth for agent guidelines. CLAUDE.md is a symlink. .rules contains high-signal traps only.

---

### Task 1: Rewrite AGENTS.md

**Files:**
- Overwrite: `AGENTS.md`

- [ ] **Step 1: Write new AGENTS.md**

The new AGENTS.md must preserve:
- Rust coding guidelines (correctness, no unwrap, error handling, no mod.rs, full words, variable shadowing)
- GPUI guidelines (Context, Window, Entities, Concurrency, Elements, Input events, Actions, Notify, Entity events)
- Timer guidelines

The new AGENTS.md must delete:
- Zed editor-specific patterns (vim, snippets, editor actions)
- Sentry/crash investigation (§ references Zed's sentry prompts)
- Zed PR template specifics that reference editor features

The new AGENTS.md must add:
- Mux architecture guidelines (server-canonical, grid sync, generation counter)
- `#[z3rm_todo]` macro usage
- Extension system guidelines (QuickJS, manifest format, runtime.side)
- Shadow snapshot constraints (single-writer thread, SeqNo ordering, WAL discipline)
- Two-pass migration process reference

- [ ] **Step 2: Create CLAUDE.md as symlink**

```bash
ln -sf AGENTS.md CLAUDE.md
```

- [ ] **Step 3: Commit**

```bash
git add AGENTS.md CLAUDE.md
git commit -m "Rewrite AGENTS.md for z3rm, symlink CLAUDE.md"
```

### Task 2: Update .rules

**Files:**
- Overwrite: `.rules`

- [ ] **Step 1: Write new .rules**

Preserve:
- Rust coding guidelines (identical to AGENTS.md section)
- GPUI guidelines (identical)
- Timer guidelines

Delete:
- Zed Sentry integration
- Zed-specific .rules hygiene references (script/clippy path, crash prompts)
- PR template specifics

Add:
- `#[z3rm_todo]` usage rule: every hole must be marked, fix = delete attribute
- Server-canonical rule: client never parses PTY bytes, never holds layout authority
- Shadow snapshot rule: WAL append before any file write, SeqNo is monotonic
- Extension rule: core commands must work without extension host
- `.rs.old` files must never be committed

- [ ] **Step 2: Commit**

```bash
git add .rules
git commit -m "Update .rules for z3rm"
```

### Task 3: Update docs/.rules

**Files:**
- Modify: `docs/.rules`

- [ ] **Step 1: Update docs rules for z3rm documentation conventions**

Remove any Zed-specific documentation rules. Add z3rm documentation structure rules.

- [ ] **Step 2: Commit**

```bash
git add docs/.rules
git commit -m "Update docs/.rules for z3rm"
```

### Task 4: Remove Zed-specific factory prompts

**Files:**
- Remove or update: `.factory/prompts/crash/investigate.md`
- Remove or update: `.factory/prompts/crash/fix.md`

- [ ] **Step 1: Remove Zed crash investigation prompts**

These reference Zed's Sentry integration which is not applicable to z3rm.

```bash
rm -rf .factory/prompts/crash
```

- [ ] **Step 2: Commit**

```bash
git add -A
git commit -m "Remove Zed-specific factory crash prompts"
```
