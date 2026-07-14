# Plan 5-8: Two-Pass Migration

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Pass 1 scans the entire codebase and marks every broken reference with `#[zerminal_todo]`. Pass 2 fixes all holes category by category until count = 0.

**These plans are execution-time plans** — the specific holes, file paths, and fixes depend on what Pass 1's static analysis discovers. They cannot be pre-written with exact code. Instead, they define the process and acceptance criteria.

---

## Plan 5: Pass 1 — Static Analysis Scan

**Goal:** Scan every remaining crate, mark every broken reference.

### Process

1. **Scan scope:** Rust source (`.rs`), `Cargo.toml` (feature graph + deps), `build.rs`, CI workflows, keymaps/assets/settings schema, env var names, bundle IDs
2. **For each broken reference:** add `#[zerminal_todo("category", "description")]` to the enclosing item (function, struct, module, use statement)
3. **Categories:**
   - `removed-crate`: references a deleted crate's path/module
   - `broken-ref`: references a pruned module/function within a retained crate
   - `stub`: temporary placeholder code
   - `disabled-feature`: code for a feature that's disabled but retained

### Execution Method

Dispatch subagents (one at a time due to concurrency limit) to scan each remaining crate. Each subagent:
1. Runs `cargo check --features zerminal-migration -p <crate> 2>&1` 
2. Parses compiler errors for unresolved imports, missing crates, etc.
3. Adds `#[zerminal_todo]` markers at each error site
4. Reports: crate name, hole count, hole list

### Acceptance Criteria

- `cargo check --features zerminal-migration` produces zero compiler errors (all holes are marked, code compiles with marks)
- Hole count report shows total count > 0 with per-category breakdown
- Every hole has a descriptive `#[zerminal_todo]` with category and description

---

## Plan 6: Pass 2 — Fix `removed-crate` Holes

**Goal:** Fix every `removed-crate` hole. Fixing = deleting the `#[zerminal_todo]` attribute AND resolving the reference.

### Fix Strategies (per reference type)

- **`use deleted_crate::...`** → delete the import, delete or stub dependent code
- **`DeletedType` in function signatures** → replace with `#[zerminal_todo]` stub or remove the function
- **Feature flag conditional** → remove the `#[cfg(feature = "...")]` branch entirely
- **Trait impl for deleted type** → delete the impl

### Acceptance Criteria

- `removed-crate` category count = 0
- `cargo check --features zerminal-migration` still compiles

---

## Plan 7: Pass 2 — Fix `broken-ref` Holes

**Goal:** Fix every `broken-ref` hole in retained crates.

### Key Broken-Ref Targets (from spec §2.1)

- `workspace` → remove editor/project/buffer coupling
- `project` → remove buffer/language registry/LSP/index/task
- `editor` → remove editing modules (see §2.1 surgical list)
- `git_ui` → remove commit/diff editing
- `search` → rework to ripgrep-on-worktree
- `settings_json` / `settings_content` → rewrite schema
- `extension_host` → replace node_runtime with quickjs_runtime

### Acceptance Criteria

- `broken-ref` category count = 0
- `cargo check --features zerminal-migration` still compiles

---

## Plan 8: Pass 2 — Fix `stub` and `disabled-feature` Holes

**Goal:** Clean up all remaining holes. Total count = 0.

### Acceptance Criteria

- Total hole count = 0
- `cargo check --features zerminal-migration` compiles cleanly with zero holes
- `cargo check` (WITHOUT feature) compiles cleanly (no `compile_error!` triggers)
- Migration complete. Proceed to new crate implementation.
