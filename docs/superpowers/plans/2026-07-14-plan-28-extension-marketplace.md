# Plan 28: Extension Marketplace

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Implement extension marketplace: hosted extension registry with search, install, update, and version management. Built on top of Day 0 CLI install (`z3rm extension install`).

**Dependencies:** `extension_host`, `extension_cli`, `http_client`.

**Spec:** §16.11 Extension System (post-foundation)

---

### Task 1: Marketplace registry format

**Files:**
- Create: `crates/extension_host/src/marketplace.rs`

- [ ] **Step 1: Define ExtensionManifest registry format**

```rust
pub struct MarketplaceEntry {
    pub id: String,
    pub name: String,
    pub version: semver::Version,
    pub description: String,
    pub author: String,
    pub repository: Option<String>,
    pub download_url: String,
    pub checksum: String, // sha256
}
```

- [ ] **Step 2: Implement registry fetch**

Fetch from `https://extensions.z3rm.dev/registry.json` (or configurable URL).

---

### Task 2: CLI marketplace commands

**Files:**
- Modify: `crates/z3rm/src/cli/marketplace.rs`

- [ ] **Step 1: Implement `z3rm extension search <query>`**

Search marketplace registry, print results table.

- [ ] **Step 2: Implement `z3rm extension install <id>` (from marketplace)**

Download from marketplace URL, verify checksum, install to extensions directory.

- [ ] **Step 3: Implement `z3rm extension update`**

Check all installed extensions for updates, prompt user.

- [ ] **Step 4: Implement `z3rm extension list`**

List installed extensions with versions.

---

### Task 3: GPUI marketplace UI

**Files:**
- Create: `crates/extensions_ui/src/marketplace.rs`

- [ ] **Step 1: Implement marketplace browser**

Search, browse, install from GPUI settings panel.

---

### Task 4: Tests + Commit

- [ ] `cargo check` passes
- [ ] Commit + push
