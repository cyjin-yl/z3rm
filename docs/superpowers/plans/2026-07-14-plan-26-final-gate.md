# Plan 26: Final Compilation Gate

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Verify the complete foundation compiles cleanly WITHOUT the `z3rm-migration` feature. All `#[z3rm_todo]` macros deleted. All tests pass. This is the final Day 0 gate.

---

### Task 1: Verify zero migration holes

- [ ] **Step 1: Run with migration feature**

Run: `cargo check --features z3rm-migration`
Expected: PASS with output "z3rm: no migration holes remaining."

- [ ] **Step 2: Run WITHOUT migration feature**

Run: `cargo check`
Expected: PASS — no `compile_error!` triggers (all macros deleted).

If any `compile_error!` fires: there is a `#[z3rm_todo]` still in the codebase. Find and fix it.

- [ ] **Step 3: Search for any remaining macro attributes**

Run: `grep -rn '#\[z3rm_todo' crates/`
Expected: zero results.

---

### Task 2: Run all tests

- [ ] **Step 1: Run full test suite**

Run: `cargo test --workspace`
Expected: ALL PASS.

- [ ] **Step 2: Run property-based tests**

Run: `cargo test -p shadow_snapshot -- --ignored` (if property tests are marked ignored for long runtime)
Expected: ALL PASS.

- [ ] **Step 3: Run terminal conformance tests**

Run: `cargo test -p terminal --test vttest`
Expected: ALL PASS.

---

### Task 3: Performance SLO verification

- [ ] **Step 1: Local keystroke-to-glyph latency**

Measure: type a character, measure time to glyph appearing. p95 must be < 16ms.

- [ ] **Step 2: Local pane output throughput**

Measure: `cat /dev/urandom | base64 | head -c 500M` equivalent. Must sustain > 50 MB/s.

- [ ] **Step 3: Cold start time**

Measure: `time z3rm` to first shell prompt. Must be < 500ms.

- [ ] **Step 4: Reattach time**

Measure: detach → reattach. Must be < 200ms to interactive.

---

### Task 4: Smoke test on all platforms

- [ ] **Step 1: Linux smoke test**

Run z3rm → shell → type → split → close → reopen → reattach. All working.

- [ ] **Step 2: Windows CI runner smoke test**

GitHub Actions Windows runner. `cargo test -p mux_server -p terminal`. Verify ConPTY works.

- [ ] **Step 3: macOS smoke test (if available)**

---

### Task 5: Commit + tag

- [ ] **Step 1: Final commit**

```bash
git add -A
git commit -m "Foundation complete: Day 0 — all crates, tests, SLOs verified"
git tag v0.1.0-foundation
```

Foundation migration complete. z3rm is a working terminal + multiplexer.
