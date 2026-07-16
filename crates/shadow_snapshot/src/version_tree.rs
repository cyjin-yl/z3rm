//! 版本树：VersionNode + VersionTree + HEAD pointer + orphan tracking
//!
//! 每个节点记录一个文件的快照版本。通过 parent_id 形成链，
//! 通过 ancestors 实现 binary lifting LCA 查询。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use parking_lot::RwLock;
use smallvec::SmallVec;

/// 版本 ID，自增 u64
pub type VersionId = u64;

/// 全局单调序列号
pub type SeqNo = u64;

/// Blake3 文件路径哈希
pub type PathHash = [u8; 32];

/// SHA-256 内容哈希
pub type ContentHash = [u8; 32];

/// 快照触发原因
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotTrigger {
    /// 文件写入事件
    Write,
    /// 文件关闭事件
    Close,
    /// 防抖计时器触发
    Debounce,
    /// Decline 协议触发
    Decline,
    /// 文件删除事件
    Delete,
}

/// Delta 引用：记录压缩后的 delta 元信息
#[derive(Debug, Clone)]
pub struct DeltaRef {
    /// SHA-256(parent_content || child_content)
    pub hash: ContentHash,
    /// 压缩后大小
    pub compressed_size: u64,
}

/// 版本节点：版本树中的单个快照
#[derive(Debug, Clone)]
pub struct VersionNode {
    /// 版本唯一标识
    pub version_id: VersionId,
    /// 所属文件路径的 Blake3 哈希
    pub path_hash: PathHash,
    /// 全局单调序列号
    pub seq_no: SeqNo,
    /// 时间戳（纳秒），仅信息性
    pub timestamp_ns: u128,
    /// 父版本 ID（None 表示 root/full snapshot）
    pub parent_id: Option<VersionId>,
    /// Binary lifting 祖先跳表
    pub ancestors: SmallVec<[VersionId; 16]>,
    /// 完整快照内容哈希（materialized snapshot）
    pub full_content: Option<ContentHash>,
    /// 增量引用（相对于 parent 的 delta）
    pub delta: Option<DeltaRef>,
    /// Delta 链深度（0 = full snapshot）
    pub delta_depth: u8,
    /// 快照触发原因
    pub trigger: SnapshotTrigger,
}

/// 版本树：管理所有文件的版本节点
///
/// - `nodes`: 所有已知节点
/// - `heads`: 每个 path_hash 对应的当前 HEAD
/// - `orphans`: 不可达节点集合
pub struct VersionTree {
    nodes: RwLock<HashMap<VersionId, Arc<VersionNode>>>,
    /// 每个文件的当前 HEAD 版本
    heads: RwLock<HashMap<PathHash, VersionId>>,
    /// 不可达节点（gc 候选）
    orphans: RwLock<HashSet<VersionId>>,
    /// 版本 ID 分配器
    next_id: parking_lot::Mutex<u64>,
}

impl VersionTree {
    /// 创建空版本树
    pub fn new() -> Self {
        Self {
            nodes: RwLock::new(HashMap::new()),
            heads: RwLock::new(HashMap::new()),
            orphans: RwLock::new(HashSet::new()),
            next_id: parking_lot::Mutex::new(1),
        }
    }

    /// 获取下一个版本 ID
    fn alloc_id(&self) -> VersionId {
        let mut next = self.next_id.lock();
        let id = *next;
        *next += 1;
        id
    }

    /// 添加节点到版本树
    ///
    /// 如果 path_hash 已有 HEAD，新节点作为其子节点。
    /// 旧 HEAD 下的不可达节点标记为 orphan。
    pub fn add_node(&self, node: Arc<VersionNode>) {
        let path_hash = node.path_hash;

        // 先读当前 head，标记旧 head 为 orphan
        let old_head = {
            let heads = self.heads.read();
            heads.get(&path_hash).copied()
        };

        if let Some(old_id) = old_head {
            // 只有当新节点的 parent 不是旧 HEAD 时才标记 orphan（分支情况）
            if node.parent_id != Some(old_id) {
                let mut orphans = self.orphans.write();
                orphans.insert(old_id);
            }
        }

        // 注册新节点
        {
            let mut nodes = self.nodes.write();
            nodes.insert(node.version_id, node.clone());
        }

        // 更新 HEAD
        {
            let mut heads = self.heads.write();
            heads.insert(path_hash, node.version_id);
        }
    }

    /// 前进 HEAD：为新版本创建节点并设为当前 HEAD
    ///
    /// 返回新节点的 version_id
    pub fn advance_head(
        &self,
        path_hash: PathHash,
        seq_no: SeqNo,
        timestamp_ns: u128,
        parent_id: Option<VersionId>,
        full_content: Option<ContentHash>,
        delta: Option<DeltaRef>,
        delta_depth: u8,
        trigger: SnapshotTrigger,
    ) -> VersionId {
        let version_id = self.alloc_id();

        // 计算 binary lifting 祖先表
        let ancestors = self.build_ancestor_table(version_id, parent_id);

        let node = Arc::new(VersionNode {
            version_id,
            path_hash,
            seq_no,
            timestamp_ns,
            parent_id,
            ancestors,
            full_content,
            delta,
            delta_depth,
            trigger,
        });

        self.add_node(node);
        version_id
    }

    /// 为节点构建 binary lifting 祖先跳表
    ///
    /// ancestors[k] = 向上跳 2^k 步的祖先 ID
    fn build_ancestor_table(&self, _version_id: VersionId, parent_id: Option<VersionId>) -> SmallVec<[VersionId; 16]> {
        let mut table: SmallVec<[VersionId; 16]> = SmallVec::new();

        if let Some(parent) = parent_id {
            table.push(parent);
            // 读父节点的祖先表
            let nodes = self.nodes.read();
            if let Some(parent_node) = nodes.get(&parent) {
                // ancestors[k] = parent.ancestors[k-1]
                for k in 1..16 {
                    if k - 1 < parent_node.ancestors.len() {
                        let jump_ancestor = parent_node.ancestors[k - 1];
                        table.push(jump_ancestor);
                    } else {
                        break;
                    }
                }
            }
        }

        table
    }

    /// 查找不可达分支的根节点
    ///
    /// 从所有 HEAD 出发 BFS/DFS 标记可达节点，
    /// 未标记的即为 orphan。
    pub fn find_orphan_branches(&self) -> HashSet<VersionId> {
        let nodes = self.nodes.read();
        let heads = self.heads.read();

        let mut reachable = HashSet::new();
        let mut stack = Vec::new();

        // 从所有 HEAD 开始 DFS
        for &head_id in heads.values() {
            stack.push(head_id);
        }

        while let Some(id) = stack.pop() {
            if reachable.insert(id) {
                if let Some(node) = &nodes.get(&id) {
                    if let Some(parent) = node.parent_id {
                        stack.push(parent);
                    }
                }
            }
        }

        // 所有不在 reachable 中的节点
        let mut orphans = HashSet::new();
        for &id in nodes.keys() {
            if !reachable.contains(&id) {
                orphans.insert(id);
            }
        }

        orphans
    }

    /// 标记一批节点为 GC 候选
    pub fn mark_gc_eligible(&self, version_ids: &[VersionId]) {
        let mut orphans = self.orphans.write();
        for id in version_ids {
            orphans.insert(*id);
        }
    }

    /// 获取指定版本节点
    pub fn get_node(&self, version_id: VersionId) -> Option<Arc<VersionNode>> {
        let nodes = self.nodes.read();
        nodes.get(&version_id).cloned()
    }

    /// 获取指定文件的当前 HEAD
    pub fn get_head(&self, path_hash: &PathHash) -> Option<VersionId> {
        let heads = self.heads.read();
        heads.get(path_hash).copied()
    }

    /// 获取不可达节点集合
    pub fn get_orphans(&self) -> HashSet<VersionId> {
        let orphans = self.orphans.read();
        orphans.clone()
    }

    /// 获取节点总数
    pub fn node_count(&self) -> usize {
        self.nodes.read().len()
    }

    /// 迭代所有节点（快照）
    pub fn iter_nodes(&self) -> Vec<(VersionId, Arc<VersionNode>)> {
        let nodes = self.nodes.read();
        nodes.iter().map(|(k, v)| (*k, v.clone())).collect()
    }

    /// 获取所有 HEAD 映射（快照）
    pub fn iter_heads(&self) -> HashMap<PathHash, VersionId> {
        let heads = self.heads.read();
        heads.clone()
    }
}

impl Default for VersionTree {
    fn default() -> Self {
        Self::new()
    }
}
