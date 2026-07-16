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
