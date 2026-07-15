# Plan 13: shadow_snapshot — Full Version Tree Engine

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Implement the complete shadow snapshot engine per spec §4. Runs inside mux_server on the file's host machine. Version tree, WAL, SQLite, bounded delta chain, Rope-level replay, binary lifting LCA, quota GC, crash-safe decline, frequency circuit breaker.

**Architecture:** Single-writer thread (watcher), WAL-first discipline, content-addressed blob store, age-based FIFO eviction. All operations keyed by monotonic SeqNo.

**Dependencies:** `rope`, `db` (SQLite), `worktree` (event stream), `zstd`, `sha2`, `blake3`, `parking_lot`.

---

### Task 1: Create crate skeleton

**Files:**
- Create: `crates/shadow_snapshot/Cargo.toml`
- Create: `crates/shadow_snapshot/src/shadow_snapshot.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "shadow_snapshot"
version = "0.1.0"
edition = "2024"
publish = false
license = "Apache-2.0"

[lib]
path = "src/shadow_snapshot.rs"

[dependencies]
rope = { workspace = true }
db = { workspace = true }
zstd = "0.13"
sha2 = { workspace = true }
blake3 = "1"
parking_lot = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
```

- [ ] **Step 2: Add to workspace Cargo.toml**

- [ ] **Step 3: Create module structure**

```rust
mod version_tree;
mod wal;
mod memtable;
mod storage;
mod delta_chain;
mod monitor;
mod decline;
mod quota;
mod lca;
```

---

### Task 2: Implement VersionNode + VersionTree

**Files:**
- Create: `crates/shadow_snapshot/src/version_tree.rs`

- [ ] **Step 1: Implement VersionNode struct (§4.4)**

```rust
use smallvec::SmallVec;

pub type VersionId = u64;
pub type SeqNo = u64;
pub type PathHash = [u8; 32]; // Blake3

pub struct VersionNode {
    pub version_id: VersionId,
    pub path_hash: PathHash,
    pub seq_no: SeqNo,
    pub timestamp_ns: u128, // informational only
    pub parent_id: Option<VersionId>,
    pub ancestors: SmallVec<[VersionId; 16]>, // binary lifting jump table
    pub full_content: Option<ContentHash>,    // materialized snapshot
    pub delta: Option<DeltaRef>,
    pub delta_depth: u8,
    pub trigger: SnapshotTrigger,
}

pub enum SnapshotTrigger {
    Write,
    Close,
    Debounce,
    Decline,
    Delete,
}

pub type ContentHash = [u8; 32]; // SHA-256

pub struct DeltaRef {
    pub hash: ContentHash, // SHA-256(parent_content || child_content)
    pub compressed_size: u64,
}
```

- [ ] **Step 2: Implement VersionTree with HEAD pointer + orphan tracking**

```rust
pub struct VersionTree {
    nodes: parking_lot::RwLock<HashMap<VersionId, Arc<VersionNode>>>,
    heads: parking_lot::RwLock<HashMap<PathHash, VersionId>>, // current HEAD per path
    orphans: parking_lot::RwLock<HashSet<VersionId>>,         // unreachable from any HEAD
}
```

Implement: add_node, advance_head, find_orphan_branches, mark_gc_eligible.

---

### Task 3: Implement WAL (Layer 0)

**Files:**
- Create: `crates/shadow_snapshot/src/wal.rs`

- [ ] **Step 1: Implement append-only WAL with SeqNo**

```rust
pub struct Wal {
    file: parking_lot::Mutex<std::fs::File>,
    path: PathBuf,
}

impl Wal {
    pub fn append(&self, entry: &WalEntry) -> Result<()>;
    pub fn replay(&self) -> Result<Vec<WalEntry>>;
    pub fn checkpoint(&self) -> Result<()>; // truncate after MemTable flush
}

pub struct WalEntry {
    pub seq_no: SeqNo,
    pub path_hash: PathHash,
    pub parent_id: Option<VersionId>,
    pub content_ref: Option<ContentHash>,
    pub delta_ref: Option<DeltaRef>,
    pub trigger: SnapshotTrigger,
}
```

- [ ] **Step 2: Implement group commit (fsync batching)**

Multiple changes in a debounce window → one fsync.

---

### Task 4: Implement MemTable (Layer 1)

**Files:**
- Create: `crates/shadow_snapshot/src/memtable.rs`

- [ ] **Step 1: Implement BTreeMap<SeqNo, PathChange> with hot cache**

```rust
pub struct MemTable {
    entries: parking_lot::RwLock<BTreeMap<SeqNo, PathChange>>,
    hot_cache: parking_lot::Mutex<LruCache<PathHash, Arc<Rope>>>,
}
```

- [ ] **Step 2: Implement range query (§4.2)**

`query_changed_files(t1: SeqNo, t2: SeqNo) -> HashSet<PathHash>`

---

### Task 5: Implement SQLite persistence (Layer 2)

**Files:**
- Create: `crates/shadow_snapshot/src/storage.rs`

- [ ] **Step 1: Implement SQLite schema + indexes**

```sql
PRAGMA journal_mode=WAL;

CREATE TABLE version_nodes (
    version_id INTEGER PRIMARY KEY,
    path_hash BLOB NOT NULL,
    seq_no INTEGER NOT NULL,
    parent_id INTEGER,
    full_content_hash BLOB,
    delta_hash BLOB,
    delta_depth INTEGER NOT NULL,
    trigger TEXT NOT NULL,
    FOREIGN KEY (parent_id) REFERENCES version_nodes(version_id)
);

CREATE INDEX idx_seq ON version_nodes(seq_no);
CREATE INDEX idx_path_seq ON version_nodes(path_hash, seq_no DESC);
```

- [ ] **Step 2: Implement content-addressed blob store**

Sharded by `hash[0:2]/content-hash`. Small blobs (< 4KB) inline in SQLite. Zstd level-1 compression. Refcounted.

---

### Task 6: Implement bounded delta chain + Rope replay

**Files:**
- Create: `crates/shadow_snapshot/src/delta_chain.rs`

- [ ] **Step 1: Implement delta application on Rope**

Each delta is `Vec<(offset: usize, delete_len: usize, insert: Arc<Rope>)>`. Apply on Rope = $O(\log N + \|insert\|)$ per operation.

- [ ] **Step 2: Implement D_MAX=16 materialization**

When delta_depth reaches 16, next version forces full snapshot materialization.

- [ ] **Step 3: Implement content reconstruction**

Walk from version V back to nearest full snapshot (≤ D_MAX steps), apply deltas forward.

---

### Task 7: Implement binary lifting LCA

**Files:**
- Create: `crates/shadow_snapshot/src/lca.rs`

- [ ] **Step 1: Implement ancestor jump table**

Each node stores $\lceil \log_2(\text{depth}) \rceil$ ancestor pointers.

- [ ] **Step 2: Implement LCA query**

$O(\log D)$ query for cross-branch diff.

---

### Task 8: Implement file monitoring + circuit breaker

**Files:**
- Create: `crates/shadow_snapshot/src/monitor.rs`

- [ ] **Step 1: Subscribe to worktree event stream**

Single subscription — no double-watching.

- [ ] **Step 2: Implement ignore filter**

Default list + `.z3rmignore` + `.gitignore` from worktree.

- [ ] **Step 3: Implement binary magic detection**

ELF magic (`\x7fELF`), PE magic (`MZ`), Mach-O magic (`\xfe\xed\xfa\xce`).

- [ ] **Step 4: Implement frequency circuit breaker**

K writes/sec → suspend snapshotting for that file until 2s idle.

---

### Task 9: Implement crash-safe decline protocol

**Files:**
- Create: `crates/shadow_snapshot/src/decline.rs`

- [ ] **Step 1: Implement WAL-first decline**

Step 1: Write WAL entry with trigger=Decline + content_ref=hash(target). fsync.
Step 2: Write file to disk.
Step 3: Watcher sees change → matches pending Decline WAL entry by content hash → skips.
Step 4: MemTable updated with new node.

- [ ] **Step 2: Implement crash recovery for all interleavings**

Crash between step 1-2: WAL has Decline, file unchanged → replay re-executes step 2.
Crash between step 2-3: Watcher matches pending → skips. Replay completes step 4.

---

### Task 10: Implement quota GC

**Files:**
- Create: `crates/shadow_snapshot/src/quota.rs`

- [ ] **Step 1: Implement age-based FIFO eviction**

Evict oldest nodes by seq_no. Promote-to-full when full snapshot's delta children are live.

- [ ] **Step 2: Implement promote-to-full batching**

Batch promotions in a single GC pass.

- [ ] **Step 3: Implement orphan branch pruning**

Branches unreachable from HEAD after grace period (24h default) → gc-eligible.

- [ ] **Step 4: Implement git commit hook**

After git commit, mark pre-commit deltas as gc-eligible.

---

### Task 11: Unit tests

- [ ] **Step 1: WAL replay correctness tests**

Write N entries → crash → replay → verify exact reconstruction.

- [ ] **Step 2: Version tree CRUD tests**

Add nodes, advance HEAD, find orphans, diff cross-branch.

- [ ] **Step 3: Delta chain replay tests**

Create D_MAX+1 versions, verify reconstruction from any version, verify materialization.

- [ ] **Step 4: GC invariant tests**

Evict nodes, verify HEAD branches remain reconstructable, verify blob refcount.

- [ ] **Step 5: Decline crash safety tests**

All crash interleavings, verify correct recovery.

- [ ] **Step 6: Property-based tests**

Model checking: random sequence of writes/declines/crashes, verify invariant that any version is reconstructable.

- [ ] **Step 7: Run tests**

Run: `cargo test -p shadow_snapshot`
Expected: ALL PASS

- [ ] **Step 8: Commit**

```bash
git add crates/shadow_snapshot Cargo.toml
git commit -m "Add shadow_snapshot: version tree, WAL, SQLite, delta chain, LCA, quota GC, crash-safe decline"
```
