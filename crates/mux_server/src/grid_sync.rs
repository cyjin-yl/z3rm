// §3.3 Grid Sync 模块 — generation counter、diff ring、grid snapshot。
// 实现 pull-based grid 同步: 客户端基于 generation 拉取 diff 或全量快照。

use std::collections::VecDeque;

/// 网格行级差异 (§3.3 GridDiff)
#[derive(Clone, Debug, Default)]
pub struct GridDiff {
    /// 变更的行列表
    pub rows: Vec<RowChange>,
}

/// 单行变更 (§3.3 RowChange)
#[derive(Clone, Debug)]
pub struct RowChange {
    /// 行号 (从 0 开始)
    pub row: u32,
    /// 单元格列表
    pub cells: Vec<Cell>,
}

/// 单元格 (§3.3 Cell)
#[derive(Clone, Debug, Default)]
pub struct Cell {
    /// 字符
    pub character: String,
    /// 样式标志
    pub style: CellStyle,
    /// 前景色 0xRRGGBB
    pub foreground: u32,
    /// 背景色 0xRRGGBB
    pub background: u32,
}

/// 单元格样式 (§3.3 CellStyle)
#[derive(Clone, Copy, Debug, Default)]
pub struct CellStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub dim: bool,
    pub reverse: bool,
}

/// 光标状态 (§3.3 CursorState)
#[derive(Clone, Copy, Debug)]
pub struct CursorState {
    pub col: u32,
    pub row: u32,
    pub style: CursorShape,
    pub visible: bool,
}

/// 光标形状 (§3.3)
#[derive(Clone, Copy, Debug, Default)]
pub enum CursorShape {
    #[default]
    Block,
    Bar,
    Underline,
}

/// 完整网格快照 (§3.3 FullGridSnapshot)
#[derive(Clone, Debug)]
pub struct FullGridSnapshot {
    /// 列数
    pub cols: u32,
    /// 行数
    pub rows: u32,
    /// 扁平单元格数组 (row-major: cells[row * cols + col])
    pub cells: Vec<Cell>,
    /// 光标状态
    pub cursor: CursorState,
    /// 是否使用 alternate screen
    pub alternate_screen: bool,
}

/// Grid diff ring (§3.3 默认 64 entries)
pub struct GridDiffRing {
    /// 环形缓冲区
    entries: VecDeque<DiffEntry>,
    /// 容量
    capacity: usize,
}

/// Diff 条目: generation + diff
#[derive(Clone, Debug)]
struct DiffEntry {
    generation: u64,
    diff: GridDiff,
}

/// Grid 更新结果 (§3.3 FetchGridUpdateResponse)
#[derive(Debug)]
pub enum GridUpdate {
    /// 增量 diff (§3.3 GridDiff)
    Diff {
        from_generation: u64,
        to_generation: u64,
        diff: GridDiff,
    },
    /// 全量快照 (§3.3 FullGridSnapshot)
    FullSnapshot {
        to_generation: u64,
        snapshot: FullGridSnapshot,
    },
    /// 无变化 (since_generation == current)
    NoChange(u64),
}

// === §16.9 Scrollback Buffer ===

/// 回滚缓冲区 (§16.9) — 存储 alacritty 历史行。
/// 每行保存为 RowChange, 按时间倒序排列 (最新行在末尾)。
#[derive(Clone, Debug)]
pub struct ScrollbackBuffer {
    /// 历史行列表 (从旧到新)
    pub rows: Vec<RowChange>,
    /// 容量上限 (默认 10_000 行)
    capacity: usize,
}

/// 回滚版本 (§16.9) — counter + timestamp 对, 用于缓存失效检测。
/// Counter 在环形缓冲区 wrap 时递增, timestamp 为 Unix 秒。
#[derive(Clone, Copy, Debug, Default)]
pub struct ScrollbackVersion {
    /// 环形缓冲区 wrap 计数器
    pub counter: u64,
    /// Unix 时间戳 (秒)
    pub timestamp: u64,
}

impl ScrollbackVersion {
    /// 创建新版本 (§16.9)
    pub fn new() -> Self {
        Self {
            counter: 1,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// 递增 counter, 更新 timestamp (§16.9 ring wrap)
    pub fn bump(&mut self) {
        self.counter += 1;
        self.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// 将版本编码为单个 u64 (counter << 32 | timestamp)
    pub fn encode(&self) -> u64 {
        (self.counter << 32) | (self.timestamp & 0xFFFFFFFF)
    }

    /// 从编码值解码版本
    pub fn decode(encoded: u64) -> Self {
        Self {
            counter: (encoded >> 32) as u64,
            timestamp: (encoded & 0xFFFFFFFF) as u64,
        }
    }
}

impl ScrollbackBuffer {
    /// 创建回滚缓冲区 (§16.9 默认 10_000 行)
    pub fn new(capacity: usize) -> Self {
        Self {
            rows: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// 追加一行到缓冲区 (§16.9)
    pub fn push_row(&mut self, row: RowChange) {
        self.rows.push(row);
        // 超出容量时移除最早的历史行
        while self.rows.len() > self.capacity {
            self.rows.remove(0);
        }
    }

    /// 获取总行数 (§16.9)
    pub fn total_lines(&self) -> u32 {
        self.rows.len() as u32
    }

    /// 获取指定范围的行 (§16.9 fetch_scrollback)
    /// from_line: 起始行号 (0 = 最早的历史行)
    /// count: 要获取的行数
    /// direction: 0 = 向上 (from_line 往旧方向), 1 = 向下 (from_line 往新方向)
    pub fn fetch_lines(&self, from_line: u32, count: u32, direction: u32) -> Vec<RowChange> {
        if self.rows.is_empty() {
            return Vec::new();
        }

        let total = self.rows.len();
        let from = from_line as usize;
        let count = count as usize;

        match direction {
            0 => {
                // §16.9 向上: 从 from_line 往旧方向 (行号减小)
                let start = if from + 1 >= count { from + 1 - count } else { 0 };
                self.rows[start..=from]
                    .iter()
                    .cloned()
                    .collect()
            }
            _ => {
                // §16.9 向下: 从 from_line 往新方向 (行号增大)
                let end = std::cmp::min(from + count, total);
                if from >= total {
                    return Vec::new();
                }
                self.rows[from..end]
                    .iter()
                    .cloned()
                    .collect()
            }
        }
    }

    /// §16.9 正则搜索回滚缓冲区
    /// 返回匹配行号列表 + 对应的 RowChange
    pub fn search(
        &self,
        regex: &str,
        from_line: u32,
        direction: u32,
        max_results: u32,
    ) -> Vec<(u32, RowChange)> {
        if self.rows.is_empty() {
            return Vec::new();
        }

        // 编译正则表达式 (§16.9)
        let re = match regex::Regex::new(regex) {
            Ok(re) => re,
            Err(_) => return Vec::new(),
        };

        // 构建搜索顺序
        let total = self.rows.len();
        let from = from_line as usize;
        let max = max_results as usize;

        let indices: Vec<usize> = match direction {
            0 => {
                // §16.9 向上搜索: 从 from_line 往 0
                if from >= total {
                    (0..total).rev().collect()
                } else {
                    (0..=from).rev().collect()
                }
            }
            _ => {
                // §16.9 向下搜索: 从 from_line 往末尾
                if from >= total {
                    Vec::new()
                } else {
                    (from..total).collect()
                }
            }
        };

        let mut results = Vec::new();
        for idx in indices {
            if results.len() >= max {
                break;
            }
            // 将行内容拼接为字符串, 用正则匹配 (§16.9)
            let text = self.rows[idx]
                .cells
                .iter()
                .map(|c| c.character.as_str())
                .collect::<String>();
            if re.is_match(&text) {
                results.push((idx as u32, self.rows[idx].clone()));
            }
        }
        results
    }

    /// 检查缓冲区是否已满, 需要 bump version (§16.9 wrap detection)
    pub fn is_full(&self) -> bool {
        self.rows.len() >= self.capacity
    }

    /// 清空缓冲区 (§16.9)
    pub fn clear(&mut self) {
        self.rows.clear();
    }
}

impl GridDiffRing {
    /// 创建 diff ring (§3.3 默认 64 entries)
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// 推送新的 diff (§3.3)
    pub fn push(&mut self, generation: u64, diff: GridDiff) {
        self.entries.push_back(DiffEntry { generation, diff });
        while self.entries.len() > self.capacity {
            self.entries.pop_front();
        }
    }

    /// §3.3 fetch_grid_update: 根据 since_generation 返回 diff 或全量快照
    pub fn fetch_update(&self, since_generation: u64, pane: &crate::pane::Pane) -> GridUpdate {
        let current = pane.get_generation();
        if since_generation == current {
            return GridUpdate::NoChange(current);
        }

        if since_generation > current {
            return GridUpdate::NoChange(current);
        }

        if let Some(oldest) = self.entries.front() {
            if since_generation < oldest.generation {
                return GridUpdate::FullSnapshot {
                    to_generation: current,
                    snapshot: pane.get_full_snapshot(),
                };
            }
        }

        let mut merged_diff = GridDiff::default();
        for entry in &self.entries {
            if entry.generation > since_generation {
                for row_change in &entry.diff.rows {
                    let pos = merged_diff
                        .rows
                        .iter()
                        .position(|r| r.row == row_change.row);
                    if let Some(idx) = pos {
                        merged_diff.rows[idx].cells = row_change.cells.clone();
                    } else {
                        merged_diff.rows.push(row_change.clone());
                    }
                }
            }
        }

        GridUpdate::Diff {
            from_generation: since_generation,
            to_generation: current,
            diff: merged_diff,
        }
    }

    /// 获取当前条目数
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// 构建空快照 (桩实现)
pub fn build_empty_snapshot(cols: u32, rows: u32) -> FullGridSnapshot {
    let cell_count = cols as usize * rows as usize;
    FullGridSnapshot {
        cols,
        rows,
        cells: vec![Cell::default(); cell_count],
        cursor: CursorState {
            col: 0,
            row: 0,
            style: CursorShape::Block,
            visible: true,
        },
        alternate_screen: false,
    }
}
