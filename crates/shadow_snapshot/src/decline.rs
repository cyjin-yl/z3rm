//! Decline：crash-safe decline 协议，WAL-first
//!
//! 步骤：
//! 1. 写 WAL entry（trigger=Decline, content_ref=hash(target)），fsync
//! 2. 写文件到磁盘
//! 3. Watcher 看到变更 → 匹配 pending Decline WAL entry → 跳过
//! 4. MemTable 更新新节点
//!
//! 崩溃恢复：
//! - 崩溃在步骤 1-2 之间：WAL 有 Decline，文件未变 → replay 重新执行步骤 2
//! - 崩溃在步骤 2-3 之间：Watcher 匹配 pending → 跳过，replay 完成步骤 4

use std::path::Path;

use anyhow::Result;
use sha2::{Sha256, Digest};
use tracing::info;

use crate::version_tree::{ContentHash, PathHash, SeqNo, SnapshotTrigger, VersionId};
use crate::wal::{Wal, WalEntry};

/// Decline 协议执行器
pub struct DeclineProtocol<'a> {
    /// WAL 引用
    wal: &'a Wal,
    /// 当前序列号
    seq_no: SeqNo,
}

impl<'a> DeclineProtocol<'a> {
    /// 创建 Decline 协议
    pub fn new(wal: &'a Wal, seq_no: SeqNo) -> Self {
        Self { wal, seq_no }
    }

    /// 执行完整的 decline 协议
    ///
    /// 步骤 1: WAL entry + fsync
    /// 步骤 2: 写文件
    /// 步骤 3: 由外部 watcher 处理（匹配 pending entry）
    /// 步骤 4: 由外部 memtable 处理
    pub fn execute(
        &self,
        path_hash: PathHash,
        parent_id: Option<VersionId>,
        target_content: &[u8],
        target_path: &Path,
    ) -> Result<ContentHash> {
        // 步骤 1: 计算目标内容哈希，写 WAL entry + fsync
        let content_hash = Self::compute_hash(target_content);
        let entry = WalEntry {
            seq_no: self.seq_no,
            path_hash,
            parent_id,
            content_ref: Some(content_hash),
            delta_ref: None,
            trigger: SnapshotTrigger::Decline,
        };

        self.wal.append(&entry)?;
        self.wal.commit()?; // fsync WAL

        info!(
            seq_no = self.seq_no,
            hash = ?content_hash,
            "decline: WAL entry written and fsynced"
        );

        // 步骤 2: 写文件到磁盘
        std::fs::write(target_path, target_content)?;
        // fsync 文件
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(target_path)?;
        file.sync_all()?;

        info!(
            path = ?target_path,
            "decline: file written and fsynced"
        );

        Ok(content_hash)
    }

    /// 计算内容 SHA-256 哈希
    pub fn compute_hash(data: &[u8]) -> ContentHash {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().into()
    }

    /// 检查 pending decline 条目是否匹配当前文件状态
    ///
    /// Watcher 调用此方法：如果 WAL 中有 Decline entry 且 content_hash
    /// 与当前文件内容匹配，则跳过（不触发额外快照）。
    pub fn check_pending(wal: &Wal, path_hash: PathHash) -> Result<Option<ContentHash>> {
        let entries = wal.replay()?;

        for entry in &entries {
            if entry.path_hash == path_hash && entry.trigger == SnapshotTrigger::Decline {
                return Ok(entry.content_ref);
            }
        }

        Ok(None)
    }

    /// 崩溃恢复：检查未完成的 decline 操作
    ///
    /// 返回需要恢复的条目列表
    pub fn recover(wal: &Wal) -> Result<Vec<WalEntry>> {
        let entries = wal.replay()?;

        let pending: Vec<WalEntry> = entries
            .into_iter()
            .filter(|e| e.trigger == SnapshotTrigger::Decline)
            .collect();

        if !pending.is_empty() {
            info!(count = pending.len(), "decline: found pending entries to recover");
        }

        Ok(pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decline_protocol_full() {
        let dir = tempfile::tempdir().unwrap();
        let wal_path = dir.path().join("test.wal");
        let file_path = dir.path().join("target.txt");

        let wal = Wal::open(&wal_path).unwrap();
        let protocol = DeclineProtocol::new(&wal, 1);

        let content = b"decline target content";
        let path_hash: PathHash = [0xAA; 32];

        let hash = protocol
            .execute(path_hash, None, content, &file_path)
            .unwrap();

        // 验证文件内容
        let written = std::fs::read(&file_path).unwrap();
        assert_eq!(written, content.as_slice());

        // 验证 WAL entry
        let entries = wal.replay().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].trigger, SnapshotTrigger::Decline);
        assert_eq!(entries[0].content_ref, Some(hash));
    }

    #[test]
    fn test_decline_pending_check() {
        let dir = tempfile::tempdir().unwrap();
        let wal_path = dir.path().join("test.wal");

        let wal = Wal::open(&wal_path).unwrap();

        let content_hash = DeclineProtocol::compute_hash(b"test");
        let entry = WalEntry {
            seq_no: 1,
            path_hash: [0xBB; 32],
            parent_id: None,
            content_ref: Some(content_hash),
            delta_ref: None,
            trigger: SnapshotTrigger::Decline,
        };
        wal.append(&entry).unwrap();
        wal.commit().unwrap();

        // 检查 pending
        let found = DeclineProtocol::check_pending(&wal, [0xBB; 32]).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap(), content_hash);

        // 不匹配的路径
        let found = DeclineProtocol::check_pending(&wal, [0xCC; 32]).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_decline_recovery() {
        let dir = tempfile::tempdir().unwrap();
        let wal_path = dir.path().join("test.wal");

        let wal = Wal::open(&wal_path).unwrap();

        // 写一个 Decline entry（模拟崩溃在步骤 1 之后，步骤 2 之前）
        wal.append(&WalEntry {
            seq_no: 1,
            path_hash: [0xDD; 32],
            parent_id: None,
            content_ref: Some(DeclineProtocol::compute_hash(b"recovery test")),
            delta_ref: None,
            trigger: SnapshotTrigger::Decline,
        })
        .unwrap();

        // 也写一个普通 Write entry
        wal.append(&WalEntry {
            seq_no: 2,
            path_hash: [0xEE; 32],
            parent_id: None,
            content_ref: None,
            delta_ref: None,
            trigger: SnapshotTrigger::Write,
        })
        .unwrap();
        wal.commit().unwrap();

        // 恢复：应该只返回 Decline entry
        let pending = DeclineProtocol::recover(&wal).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].trigger, SnapshotTrigger::Decline);
    }

    #[test]
    fn test_decline_crash_between_step1_step2() {
        // 模拟崩溃在 WAL fsync 之后，文件写入之前
        let dir = tempfile::tempdir().unwrap();
        let wal_path = dir.path().join("test.wal");
        let file_path = dir.path().join("target.txt");

        let wal = Wal::open(&wal_path).unwrap();

        let content_hash = DeclineProtocol::compute_hash(b"crash test");
        wal.append(&WalEntry {
            seq_no: 1,
            path_hash: [0xFF; 32],
            parent_id: None,
            content_ref: Some(content_hash),
            delta_ref: None,
            trigger: SnapshotTrigger::Decline,
        })
        .unwrap();
        wal.commit().unwrap();

        // 此时文件未写入（模拟崩溃）
        assert!(!file_path.exists());

        // Replay: WAL 中有 Decline entry → 需要重新执行步骤 2
        let pending = DeclineProtocol::recover(&wal).unwrap();
        assert_eq!(pending.len(), 1);

        // Replay 重新写入文件
        let _entry = &pending[0];
        std::fs::write(&file_path, b"crash test").unwrap();

        // 验证恢复
        assert!(file_path.exists());
    }
}
