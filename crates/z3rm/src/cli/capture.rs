// capture-pane 实现: 从 server 拉取 grid 并转为文本
// 来源: spec §3.10 — capture-pane -p 输出 pane 可见内容

use anyhow::{Context, Result};
use mux::MuxDomain;
use mux_protocol::proto::fetch_grid_update_response::Update as GridUpdateKind;

/// 捕获 pane 的可见网格内容，转换为文本。
///
/// - `pane_id`: pane 的唯一标识
/// - `scrollback_lines`: 可选的历史行数。负值表示包含 scrollback。
/// - `preserve_ansi`: 是否保留 ANSI 颜色码
pub async fn capture_pane(
    domain: &MuxDomain,
    pane_id: &str,
    scrollback_lines: Option<i32>,
    _preserve_ansi: bool,
) -> Result<String> {
    let mut output = String::new();

    // 如果需要 scrollback, 先拉取历史行
    if let Some(n) = scrollback_lines.filter(|n| *n < 0) {
        let count = (-n) as u32;
        match domain.fetch_scrollback(pane_id, 0, 1, count).await {
            Ok(scrollback) => {
                for row in &scrollback.lines {
                    let mut line = String::new();
                    for cell in &row.cells {
                        line.push_str(&cell.char);
                    }
                    output.push_str(&line);
                    output.push('\n');
                }
            }
            Err(e) => {
                // scrollback 拉取失败时继续, 只输出可见网格
                eprintln!("warning: could not fetch scrollback: {}", e);
            }
        }
    }

    // 拉取完整网格快照
    let grid = domain
        .fetch_grid_update(pane_id, 0)
        .await
        .context("failed to fetch grid update")?;

    // 将 grid cells 转为文本行
    if let Some(update) = &grid.update {
        match update {
            GridUpdateKind::FullSnapshot(snapshot) => {
                for row in 0..snapshot.rows {
                    let mut line = String::new();
                    for col in 0..snapshot.cols {
                        let idx = (row * snapshot.cols + col) as usize;
                        if idx < snapshot.cells.len() {
                            let cell = &snapshot.cells[idx];
                            line.push_str(&cell.char);
                        }
                    }
                    output.push_str(&line);
                    output.push('\n');
                }
            }
            GridUpdateKind::Diff(diff) => {
                // 增量 diff 模式: 按行输出变更
                for row_change in &diff.rows {
                    let mut line = String::new();
                    for cell in &row_change.cells {
                        line.push_str(&cell.char);
                    }
                    output.push_str(&line);
                    output.push('\n');
                }
            }
        }
    }

    Ok(output)
}
