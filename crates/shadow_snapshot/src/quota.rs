//! Quota GC：age-based FIFO eviction + promote-to-full
//!
//! - 按 seq_no 顺序 evict 最老的节点
//! - 当 full snapshot 的 delta children 全部被 GC 后，promote-to-full
//! - Orphan branch pruning：grace period（默认 24h）后 GC 候选
//! - Git commit hook：commit 后标记 pre-commit deltas 为 gc-eligible

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use tracing::{info, warn};

use crate::version_tree::{SeqNo, VersionId, VersionTree};

/// 孤儿分支 grace period（默认 24h）
const DEFAULT_GRACE_PERIOD: Duration = Duration::from_secs(24 * 3600);

/// 配额管理器
pub struct QuotaManager {
    /// 最大存储空间（字节）
    max_bytes: u64,
    /// 当前使用空间
    used_bytes: parking_lot::Mutex<u64>,
    /// Grace period（interior mutability）
    grace_period: parking_lot::Mutex<Duration>,
    /// 孤儿节点标记时间
    orphan_since: parking_lot::Mutex<HashMap<VersionId, Instant>>,
    /// GC 候选集合
    gc_eligible: parking_lot::Mutex<HashSet<VersionId>>,
}

impl QuotaManager {
    /// 创建配额管理器
    pub fn new(max_bytes: u64) -> Self {
        Self {
            max_bytes,
            used_bytes: parking_lot::Mutex::new(0),
            grace_period: parking_lot::Mutex::new(DEFAULT_GRACE_PERIOD),
            orphan_since: parking_lot::Mutex::new(HashMap::new()),
            gc_eligible: parking_lot::Mutex::new(HashSet::new()),
        }
    }

    /// 设置 grace period
    pub fn set_grace_period(&self, period: Duration) {
        *self.grace_period.lock() = period;
    }

    /// 检查是否超过配额
    pub fn is_over_quota(&self) -> bool {
        *self.used_bytes.lock() > self.max_bytes
    }

    /// 执行 GC：age-based FIFO eviction
    ///
    /// 按 seq_no 从小到大 evict 最老的节点，直到回到配额内。
    /// 保留当前 HEAD 链上的所有节点。
    pub fn run_gc(&self, tree: &VersionTree) -> u64 {
        let mut freed = 0u64;
        let head_ids = self.collect_head_ids(tree);

        // 按 seq_no 排序所有非 HEAD 节点
        let mut candidates = Vec::new();

        for (id, node) in tree.iter_nodes() {
            if !head_ids.contains(&id) && !node.full_content.is_some() {
                // 优先 GC delta 节点（非 full snapshot）
                candidates.push((node.seq_no, id, node.delta_depth));
            }
        }

        // 按 seq_no 排序（FIFO），delta_depth 高的优先
        candidates.sort_by(|a, b| a.0.cmp(&b.0).then(b.2.cmp(&a.2)));

        let used = *self.used_bytes.lock();
        let to_free = used.saturating_sub(self.max_bytes);

        for (seq_no, id, _depth) in &candidates {
            if freed >= to_free {
                break;
            }

            // 标记为 GC 候选
            {
                let mut eligible = self.gc_eligible.lock();
                eligible.insert(*id);
            }

            // 估算释放空间（delta 大小）
            freed += self.estimate_node_size(tree, *id);

            info!(version_id = *id, seq_no = *seq_no, "gc: evicting node");
        }

        *self.used_bytes.lock() = used.saturating_sub(freed);

        // 将 GC 候选标记到 version tree
        let eligible = self.gc_eligible.lock().clone();
        if !eligible.is_empty() {
            let ids: Vec<VersionId> = eligible.into_iter().collect();
            tree.mark_gc_eligible(&ids);
        }

        freed
    }

    /// 收集所有 HEAD 链上的节点 ID（不可 GC）
    fn collect_head_ids(&self, tree: &VersionTree) -> HashSet<VersionId> {
        let mut head_ids = HashSet::new();
        let orphans = tree.get_orphans();

        let heads = tree.iter_heads();
        for &head_id in heads.values() {
            let mut stack = vec![head_id];
            while let Some(id) = stack.pop() {
                if head_ids.insert(id) {
                    if let Some(node) = tree.get_node(id) {
                        if let Some(parent) = node.parent_id {
                            // 不跟随 orphan 节点的祖先链
                            if !orphans.contains(&parent) {
                                stack.push(parent);
                            }
                        }
                    }
                }
            }
        }

        head_ids
    }

    /// 估算节点占用的空间
    fn estimate_node_size(&self, tree: &VersionTree, id: VersionId) -> u64 {
        if let Some(node) = tree.get_node(id) {
            // 估算：delta 节点 ≈ compressed_size，full snapshot ≈ 更大
            if let Some(delta) = &node.delta {
                delta.compressed_size
            } else if node.full_content.is_some() {
                4096 // 估算 full snapshot 大小
            } else {
                0
            }
        } else {
            0
        }
    }

    /// Promote-to-full：批量处理可提升的 full snapshot
    ///
    /// 当 full snapshot 的所有 delta children 都被 GC 后，
    /// 可以将该 full snapshot 提升为新的 base，释放旧的 delta 引用。
    pub fn batch_promote(&self, tree: &VersionTree) -> usize {
        let mut promoted = 0;

        let full_snapshots: Vec<_> = tree
            .iter_nodes()
            .into_iter()
            .filter(|(_, n)| n.full_content.is_some())
            .collect();

        for (snapshot_id, _snapshot) in &full_snapshots {
            // 检查所有 delta children 是否已被 GC
            let mut all_children_gc = true;

            for (child_id, child) in tree.iter_nodes() {
                if child.parent_id == Some(*snapshot_id) && child.delta.is_some() {
                    let eligible = self.gc_eligible.lock();
                    if !eligible.contains(&child_id) {
                        all_children_gc = false;
                        break;
                    }
                }
            }

            if all_children_gc {
                // 所有 delta children 已被 GC，可以 promote
                info!(version_id = *snapshot_id, "gc: promoting full snapshot");
                promoted += 1;
            }
        }

        promoted
    }

    /// 标记孤儿分支为 GC 候选
    ///
    /// 孤儿分支在 grace period 后变为 GC 候选。
    pub fn prune_orphan_branches(&self, tree: &VersionTree, now: Instant) {
        let orphans = tree.get_orphans();

        let mut orphan_since = self.orphan_since.lock();
        for id in &orphans {
            orphan_since.entry(*id).or_insert(now);
        }

        // 检查哪些孤儿已超过 grace period
        let grace = *self.grace_period.lock();
        let mut to_gc = Vec::new();
        for (id, since) in orphan_since.iter() {
            if now.duration_since(*since) >= grace {
                to_gc.push(*id);
            }
        }

        // 标记为 GC 候选
        if !to_gc.is_empty() {
            let mut eligible = self.gc_eligible.lock();
            for id in &to_gc {
                eligible.insert(*id);
            }

            // 从 orphan_since 中移除
            for id in &to_gc {
                orphan_since.remove(id);
            }

            tree.mark_gc_eligible(&to_gc);
            warn!(count = to_gc.len(), "gc: orphan branches pruned");
        }
    }

    /// Git commit hook：标记 pre-commit deltas 为 GC 候选
    ///
    /// git commit 后，commit 之前的所有 delta 变为可 GC（
    /// 因为 commit 已持久化到 git history，shadow snapshot 不再需要它们）。
    pub fn on_git_commit(&self, tree: &VersionTree, commit_seq: SeqNo) {
        let mut to_gc = Vec::new();
        for (id, node) in tree.iter_nodes() {
            if node.seq_no < commit_seq && node.delta.is_some() {
                to_gc.push(id);
            }
        }

        if !to_gc.is_empty() {
            let mut eligible = self.gc_eligible.lock();
            for id in &to_gc {
                eligible.insert(*id);
            }

            tree.mark_gc_eligible(&to_gc);
            info!(count = to_gc.len(), seq = commit_seq, "gc: git commit hook");
        }
    }

    /// 获取 GC 候选数量
    pub fn gc_eligible_count(&self) -> usize {
        self.gc_eligible.lock().len()
    }

    /// 清除 GC 候选（实际删除后调用）
    pub fn clear_gc_eligible(&self, ids: &[VersionId]) {
        let mut eligible = self.gc_eligible.lock();
        for id in ids {
            eligible.remove(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version_tree::{SnapshotTrigger, VersionNode};
    use std::sync::Arc;

    fn make_node(seq_no: SeqNo, id: VersionId, parent: Option<VersionId>, full: bool) -> VersionNode {
        VersionNode {
            version_id: id,
            path_hash: [0x11; 32],
            seq_no,
            timestamp_ns: 0,
            parent_id: parent,
            ancestors: Default::default(),
            full_content: full.then_some([0xCC; 32]),
            delta: (!full).then_some(crate::version_tree::DeltaRef {
                hash: [0xDD; 32],
                compressed_size: 100,
            }),
            delta_depth: if full { 0 } else { 1 },
            trigger: SnapshotTrigger::Write,
        }
    }

    #[test]
    fn test_gc_fifo_eviction() {
        let tree = VersionTree::new();
        let quota = QuotaManager::new(100);

        // 节点 1: path A, full snapshot → HEAD A = 1
        let node1 = VersionNode {
            version_id: 1,
            path_hash: [0xAA; 32],
            seq_no: 1,
            timestamp_ns: 0,
            parent_id: None,
            ancestors: Default::default(),
            full_content: Some([0xCC; 32]),
            delta: None,
            delta_depth: 0,
            trigger: SnapshotTrigger::Write,
        };
        tree.add_node(Arc::new(node1));

        // 节点 2: path A, delta, parent=1 → HEAD A = 2 (node1 不 orphan)
        let node2 = VersionNode {
            version_id: 2,
            path_hash: [0xAA; 32],
            seq_no: 2,
            timestamp_ns: 0,
            parent_id: Some(1),
            ancestors: Default::default(),
            full_content: None,
            delta: Some(crate::version_tree::DeltaRef {
                hash: [0xDD; 32],
                compressed_size: 100,
            }),
            delta_depth: 1,
            trigger: SnapshotTrigger::Write,
        };
        tree.add_node(Arc::new(node2));

        // 节点 3: path A, delta, parent=None (分支!) → HEAD A = 3, node2 变 orphan
        let node3 = VersionNode {
            version_id: 3,
            path_hash: [0xAA; 32],
            seq_no: 3,
            timestamp_ns: 0,
            parent_id: None,
            ancestors: Default::default(),
            full_content: None,
            delta: Some(crate::version_tree::DeltaRef {
                hash: [0xDD; 32],
                compressed_size: 200,
            }),
            delta_depth: 1,
            trigger: SnapshotTrigger::Write,
        };
        tree.add_node(Arc::new(node3));

        *quota.used_bytes.lock() = 400;

        let freed = quota.run_gc(&tree);
        assert!(freed > 0, "expected to free orphan node2");
    }

    #[test]
    fn test_gc_preserves_head_chain() {
        let tree = VersionTree::new();

        // HEAD chain: 1 → 2 → 3
        tree.add_node(Arc::new(make_node(1, 1, None, true)));
        tree.add_node(Arc::new(make_node(2, 2, Some(1), false)));
        tree.add_node(Arc::new(make_node(3, 3, Some(2), false)));

        let head_ids = QuotaManager::new(0).collect_head_ids(&tree);

        // 所有三个节点都在 HEAD 链上
        assert!(head_ids.contains(&1));
        assert!(head_ids.contains(&2));
        assert!(head_ids.contains(&3));
    }

    #[test]
    fn test_orphan_pruning() {
        let tree = VersionTree::new();
        let quota = QuotaManager::new(1000);
        quota.set_grace_period(Duration::from_millis(1));

        // 添加节点然后替换 HEAD（创建 orphan）
        tree.add_node(Arc::new(make_node(1, 1, None, true)));
        tree.add_node(Arc::new(make_node(2, 2, None, true)));

        // 手动标记 orphan 为过去时间
        let past = Instant::now() - Duration::from_secs(1);
        {
            let mut orphan_since = quota.orphan_since.lock();
            orphan_since.insert(1, past);
        }

        let now = Instant::now();
        quota.prune_orphan_branches(&tree, now);

        // Orphan 1 已超过 grace period，应被标记
        assert!(quota.gc_eligible_count() > 0);
    }

    #[test]
    fn test_git_commit_hook() {
        let tree = VersionTree::new();
        let quota = QuotaManager::new(1000);

        tree.add_node(Arc::new(make_node(1, 1, None, true)));
        tree.add_node(Arc::new(make_node(2, 2, Some(1), false)));
        tree.add_node(Arc::new(make_node(3, 3, Some(2), false)));

        // Git commit at seq 2: 标记 seq < 2 的 delta 为 GC
        quota.on_git_commit(&tree, 2);

        assert_eq!(quota.gc_eligible_count(), 0); // seq 1 是 full snapshot，非 delta
    }
}
