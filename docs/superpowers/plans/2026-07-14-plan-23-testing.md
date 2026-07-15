# Plan 23: Testing & Verification

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Implement ALL testing for Day 0. No testing is deferred. Unit tests, integration test, property-based testing, protocol compatibility, terminal emulation conformance, extension sandbox fuzzing, network fault injection.

**Dependencies:** All foundation crates.

---

### Task 1: Shadow snapshot unit + property tests

**Files:**
- Create: `crates/shadow_snapshot/tests/property_tests.rs`

- [ ] **Step 1: WAL replay correctness (deterministic)**

Write N entries with random data → simulate crash at each position → verify replay produces identical state.

- [ ] **Step 2: Property-based: random write/decline/crash sequences**

Use `proptest` crate. Generate random sequence of operations (write file, decline version, crash, recover). Invariant: every version reachable from HEAD is reconstructable.

- [ ] **Step 3: GC invariant verification**

After GC: every HEAD-reachable version reconstructable. No dangling parent_id references. Blob refcount = number of referencing nodes.

---

### Task 2: Mux protocol tests

**Files:**
- Create: `crates/mux_protocol/tests/compat.rs`

- [ ] **Step 1: Serialization round-trip for ALL message types**

Enumerate every message type in the proto. Encode → decode → verify field equality.

- [ ] **Step 2: Version negotiation test**

Client with protocol v1.0 connects to server with v1.1 → verify handshake succeeds (forward compat). Client v2.0 connects to server v1.0 → verify clean error.

- [ ] **Step 3: Unknown field preservation test**

Encode message with extra field (future version) → decode with current version → verify extra field is preserved, not dropped.

---

### Task 3: Grid sync tests

**Files:**
- Create: `crates/mux_server/tests/grid_sync.rs`

- [ ] **Step 1: Generation counter logic**

Write PTY bytes → verify generation incremented. No bytes → no increment. Cursor style change → increment.

- [ ] **Step 2: Diff ring operations**

Fill ring to capacity (64) → next diff pushes oldest out → client requesting old generation gets FullSnapshot.

- [ ] **Step 3: Diff application correctness**

Apply a GridDiff to a FullGridSnapshot → verify result matches actual terminal grid state.

---

### Task 4: Layout engine tests

- [ ] **Step 1: Split tree invariants**

After any sequence of split/close/resize: no orphan panes, ratios sum to 1.0 per split, all panes reachable from root.

- [ ] **Step 2: Serialization round-trip**

Layout tree → serialize (tmux checksummed format) → deserialize → verify identical structure.

---

### Task 5: End-to-end integration test

**Files:**
- Create: `crates/z3rm/tests/e2e.rs`

- [ ] **Step 1: Full session lifecycle test**

```
1. Spawn daemon (z3rm-server)
2. Create session "test"
3. Spawn pane with shell
4. Send input "echo hello"
5. Fetch grid update → verify "hello" appears in grid
6. Split pane right
7. Verify both panes in layout snapshot
8. Focus pane 2
9. Detach
10. Verify daemon still alive (status check)
11. Reattach
12. Verify both panes rendered from snapshot
13. Close session
14. Verify daemon idle (no panes)
15. Kill daemon
```

---

### Task 6: Terminal emulation conformance

**Files:**
- Create: `crates/terminal/tests/vttest.rs`

- [ ] **Step 1: Run vttest / esctest conformance suite**

Port or integrate the standard VT terminal test suite. Verify alacritty engine passes: cursor movement, screen modes, color handling, tab stops, insert/delete line, scroll regions, etc.

---

### Task 7: Extension sandbox fuzzing

**Files:**
- Create: `crates/quickjs_runtime/tests/fuzz.rs`

- [ ] **Step 1: VDOM bridge fuzz**

Feed malformed/edge-case JSON to the VDOM bridge → verify graceful error (Result::Err), not panic.

- [ ] **Step 2: QuickJS resource exhaustion fuzz**

Load extensions that: infinite loop, excessive memory allocation, rapid IO calls → verify resource limits enforced without host crash.

---

### Task 8: Network fault injection (SSH transport)

**Files:**
- Create: `crates/mux/tests/network_fault.rs`

- [ ] **Step 1: Simulated packet loss / latency / partition**

Use `tokio::io` wrappers that inject delays, drop bytes, or close connections. Verify: client shows reconnect dialog, server detects detach, reattach recovers.

---

### Task 9: Transport resilient (UDP) tests

- [ ] **Step 1: UDP AEAD packet loss test**

- [ ] **Step 2: Stateless roaming test (change source IP)**

- [ ] **Step 3: RTT estimation accuracy test**

---

### Task 10: Commit

```bash
git add crates/*/tests/
git commit -m "Add comprehensive Day 0 testing: unit, integration, property, conformance, fuzz, fault injection"
```
