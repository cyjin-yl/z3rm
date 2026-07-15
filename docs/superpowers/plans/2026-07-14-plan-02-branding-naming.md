# Plan 2: Branding & Naming (Layer 1 — User-Visible)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Rename all user-visible "Zed" references to "Z3rm": binary name, APP_NAME, config/data directory names, environment variable prefixes, README.

**Architecture:** Layer 1 only — user-visible names. Internal `mod zed` / `use zed::` module names are Layer 2 (gradual, post-migration). This preserves cherry-pick compatibility.

**Tech Stack:** Rust, Cargo.toml, paths crate.

---

### Task 1: Rename main entry crate

**Files:**
- Rename: `crates/zed/` → `crates/z3rm/`
- Modify: `Cargo.toml` (workspace root)
- Modify: `crates/z3rm/Cargo.toml`

- [ ] **Step 1: Rename the crate directory**

```bash
git mv crates/zed crates/z3rm
```

- [ ] **Step 2: Update crate Cargo.toml**

In `crates/z3rm/Cargo.toml`, change:
```toml
[package]
name = "z3rm"
```

Also update `[lib]` path if needed and the `[[bin]]` section:
```toml
[[bin]]
name = "z3rm"
path = "src/main.rs"
```

- [ ] **Step 3: Update workspace Cargo.toml**

In root `Cargo.toml`:
- Change `"crates/zed"` to `"crates/z3rm"` in `members`
- Change `default-members = ["crates/zed"]` to `default-members = ["crates/z3rm"]`
- Change `zed = { path = "crates/zed" }` to `z3rm = { path = "crates/z3rm" }` in `[workspace.dependencies]`

- [ ] **Step 4: Update all workspace references to the zed crate**

Search for all `zed = { path = "crates/zed" }` or `path = "../zed"` references across all Cargo.toml files and update them.

Run: `grep -r 'crates/zed' --include='Cargo.toml' .` to find all references.

Replace each with `crates/z3rm`.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "Rename zed crate to z3rm"
```

### Task 2: Update paths crate

**Files:**
- Modify: `crates/paths/src/paths.rs`

- [ ] **Step 1: Find APP_NAME constants**

Run: `grep -n 'APP_NAME' crates/paths/src/paths.rs`

- [ ] **Step 2: Update constants**

```rust
pub const APP_NAME: &str = "Z3rm";
pub const APP_NAME_LOWERCASE: &str = "z3rm";
```

- [ ] **Step 3: Verify the const assertion in main.rs still holds**

The assertion checks `APP_NAME_LOWERCASE` matches `CARGO_BIN_NAME`. After rename, both should be `"z3rm"`.

Run: `grep -n 'APP_NAME_LOWERCASE' crates/z3rm/src/main.rs`

The assertion should now pass because both are `"z3rm"`.

- [ ] **Step 4: Commit**

```bash
git add crates/paths/src/paths.rs
git commit -m "Update APP_NAME to Z3rm"
```

### Task 3: Update environment variable prefixes

**Files:**
- Modify: `crates/zed_env_vars/src/*.rs` (search for ZED_ prefix)
- Search all crates for `ZED_` env var prefixes

- [ ] **Step 1: Find all ZED_ environment variables**

Run: `grep -rn 'ZED_' --include='*.rs' crates/ | grep -v test`

- [ ] **Step 2: Rename each ZED_ prefix to Z3RM_**

For each occurrence, change `ZED_` to `Z3RM_`. Use `#[z3rm_todo("broken-ref", "rename ZED_ env var")]` on any that have complex migration dependencies.

Note: The `zed_env_vars` crate itself will be renamed later (Layer 2). For now, just change the env var string constants inside it.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "Rename ZED_ environment variables to Z3RM_"
```

### Task 4: Rewrite README

**Files:**
- Overwrite: `README.md`

- [ ] **Step 1: Write new README**

```markdown
# Z3rm

A high-performance GPU-rendered terminal with a built-in multiplexer, read-only file viewer with diff review, and QuickJS extension system.

Forked from [Zed](https://github.com/zed-industries/zed). All editor, AI, and collaboration features removed. The retained core: GPUI rendering engine, terminal emulation (alacritty-based), workspace pane management, theme/settings infrastructure, and a slimmed read-only editor for file/diff viewing.

## Features

- **GPU-rendered terminal** — powered by GPUI
- **Built-in multiplexer** — tmux-class session management with detach/reattach
- **Server-canonical architecture** — mux_server owns PTY + terminal state; GUI client renders grid
- **File viewer & diff review** — read-only editor with syntax highlighting for CLI agent workflows
- **Shadow snapshot engine** — fine-grained filesystem versioning for undo/decline
- **QuickJS extension system** — all UI chrome implemented as extensions
- **Remote sessions** — SSH tunnel support with auto server installation

## Building

- [Building for Linux](./docs/development/building-linux.md)
- [Building for Windows](./docs/development/building-windows.md)

## License

Z3rm source code is licensed under GPL-3.0-or-later (inherited from Zed) with Apache-2.0 components where marked. New z3rm crates are Apache-2.0.
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "Rewrite README for z3rm"
```

### Task 5: Update CONTRIBUTING.md

**Files:**
- Overwrite: `CONTRIBUTING.md`

- [ ] **Step 1: Rewrite**

Write a new CONTRIBUTING.md for z3rm. Remove all Zed-specific content (hiring, Zed cloud, Zed discussion links). Keep the general contribution guidelines (code style, PR format, testing).

- [ ] **Step 2: Commit**

```bash
git add CONTRIBUTING.md
git commit -m "Rewrite CONTRIBUTING.md for z3rm"
```

### Task 6: Update bundle identifier and platform metadata

**Files:**
- Search for `dev.zed` or `Zed` in bundle/metadata files
- Modify: `assets/` icon names, plist files, `.desktop` files, etc.

- [ ] **Step 1: Find all bundle identifiers**

Run: `grep -rn 'dev.zed\|dev\.zed\|Zed Industries' --include='*.plist' --include='*.desktop' --include='*.json' --include='*.rc' .`

- [ ] **Step 2: Update each**

Change `dev.zed.Zed` to `dev.z3rm.Z3rm`. Change `Zed Industries` to the appropriate entity.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "Update bundle identifiers and platform metadata"
```
