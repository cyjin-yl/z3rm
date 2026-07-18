# Plan 33: Per-Client Identity and Permissions

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Implement per-client identity and permissions: each client (GUI window or CLI session) has an identity with permissions (read-only, read-write, admin). Server enforces permissions on pane operations.

**Dependencies:** `mux_server`, `mux_protocol`, `mux`.

**Spec:** §3.3 Mux Architecture (post-foundation)

---

### Task 1: Client identity

**Files:**
- Modify: `crates/mux_protocol/proto/mux.proto`

- [ ] **Step 1: Add client identity to AttachRequest**

```protobuf
message AttachRequest {
    string session_id = 1;
    AttachMode mode = 2;
    ClientIdentity identity = 3;
}

message ClientIdentity {
    string client_id = 1;  // UUID
    ClientRole role = 2;   // ReadOnly, ReadWrite, Admin
}

enum ClientRole {
    READ_ONLY = 0;
    READ_WRITE = 1;
    ADMIN = 2;
}
```

---

### Task 2: Permission enforcement

**Files:**
- Modify: `crates/mux_server/src/connection.rs`

- [ ] **Step 1: Check permissions on pane operations**

Read-only clients cannot: send input, resize, close pane, create pane.

- [ ] **Step 2: Admin-only operations**

Kill session, rename session, install extension require admin.

---

### Task 3: Permission-aware UI

**Files:**
- Modify: `crates/terminal_view/src/terminal_view.rs`

- [ ] **Step 1: Disable editing UI for read-only clients**

Hide input box, disable send-keys for read-only clients.

---

### Task 4: Tests + Commit

- [ ] `cargo check` passes
- [ ] Commit + push
