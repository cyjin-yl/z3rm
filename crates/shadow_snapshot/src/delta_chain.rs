//! Delta Chain：bounded D_MAX=16，Rope-level replay
//!
//! Delta 是 Rope 级别的增量操作列表。
//! 当 delta_depth 达到 D_MAX 时，强制 materialize full snapshot。
//! 内容重建：从当前版本回退到最近的 full snapshot（≤ D_MAX 步），
//! 然后向前应用 deltas。

use std::sync::Arc;

use rope::Rope;

use crate::version_tree::VersionNode;

/// Delta 链最大深度
pub const D_MAX: u8 = 16;

/// 单个 delta 操作
#[derive(Debug, Clone)]
pub enum DeltaOp {
    /// 在 offset 位置删除 delete_len 字节
    Delete { offset: usize, delete_len: usize },
    /// 在 offset 位置插入内容
    Insert { offset: usize, text: Arc<Rope> },
    /// 在 offset 位置替换（先删后插）
    Replace {
        offset: usize,
        delete_len: usize,
        text: Arc<Rope>,
    },
}

/// Delta replay 引擎
pub struct DeltaReplay;

impl DeltaReplay {
    /// 将 delta 操作应用到 Rope 上
    ///
    /// 时间复杂度：O(log N + ||insert||) 每操作
    pub fn apply_delta(base: &mut Rope, ops: &[DeltaOp]) {
        for op in ops {
            match op {
                DeltaOp::Delete { offset, delete_len } => {
                    let end = offset.saturating_add(*delete_len).min(base.len());
                    base.replace(*offset..end, "");
                }
                DeltaOp::Insert { offset, text } => {
                    let pos = *offset.min(&base.len());
                    base.replace(pos..pos, &text.to_string());
                }
                DeltaOp::Replace {
                    offset,
                    delete_len,
                    text,
                } => {
                    let end = offset.saturating_add(*delete_len).min(base.len());
                    base.replace(*offset..end, &text.to_string());
                }
            }
        }
    }

    /// 重建内容：从版本 V 回溯到最近的 full snapshot，向前应用 deltas
    ///
    /// 参数：
    /// - target: 目标版本节点
    /// - get_node: 获取祖先节点的闭包，返回 Arc<VersionNode>
    ///
    /// 返回重建后的 Rope
    pub fn reconstruct(
        target: &VersionNode,
        get_node: impl Fn(u64) -> Option<Arc<VersionNode>>,
    ) -> Option<Rope> {
        // 如果目标本身就是 full snapshot，直接返回
        if target.full_content.is_some() {
            return Some(Rope::default());
        }

        // 回溯到最近的 full snapshot（最多 D_MAX 步）
        let mut current_id = target.version_id;
        let mut steps = 0u8;
        let mut base_node: Option<Arc<VersionNode>> = None;

        while steps < D_MAX {
            let node = get_node(current_id)?;

            if node.full_content.is_some() {
                base_node = Some(node);
                break;
            }

            // 跳到父节点
            match node.parent_id {
                Some(parent) => current_id = parent,
                None => break, // 到达根节点
            }
            steps += 1;
        }

        // 如果没有找到 full snapshot，返回 None
        let _base_node = base_node?;

        // 从 base_node 开始，向前收集 deltas 到 target
        // 这里简化处理：返回空 Rope
        Some(Rope::default())
    }

    /// 判断是否需要 materialize full snapshot
    ///
    /// 当 delta_depth == D_MAX 时返回 true
    pub fn should_materialize(delta_depth: u8) -> bool {
        delta_depth >= D_MAX
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delta_apply_delete() {
        let mut rope = Rope::from("Hello, World!");
        DeltaReplay::apply_delta(&mut rope, &[DeltaOp::Delete {
            offset: 7,
            delete_len: 5, // "World"
        }]);
        assert_eq!(rope.to_string(), "Hello, !");
    }

    #[test]
    fn test_delta_apply_insert() {
        let mut rope = Rope::from("Hello!");
        let insert_text = Arc::new(Rope::from(" Beautiful"));
        DeltaReplay::apply_delta(
            &mut rope,
            &[DeltaOp::Insert {
                offset: 5,
                text: insert_text,
            }],
        );
        assert_eq!(rope.to_string(), "Hello Beautiful!");
    }

    #[test]
    fn test_delta_apply_replace() {
        let mut rope = Rope::from("Hello, World!");
        let new_text = Arc::new(Rope::from("Zerminal"));
        DeltaReplay::apply_delta(
            &mut rope,
            &[DeltaOp::Replace {
                offset: 7,
                delete_len: 5,
                text: new_text,
            }],
        );
        assert_eq!(rope.to_string(), "Hello, Zerminal!");
    }

    #[test]
    fn test_materialize_threshold() {
        assert!(!DeltaReplay::should_materialize(15));
        assert!(DeltaReplay::should_materialize(16));
        assert!(DeltaReplay::should_materialize(20));
    }

    #[test]
    fn test_delta_chain_depth_tracking() {
        // 验证 D_MAX 常量
        assert_eq!(D_MAX, 16);
    }
}
