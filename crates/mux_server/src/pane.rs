// §3.1 Pane 模块 — PTY + alacritty 终端模拟器封装 + grid diff ring。
// mux_server 作为 server-canonical 拥有 PTY fd、VT 终端、grid 状态 (§3.1)。

use crate::grid_sync::GridDiffRing;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

/// Pane 结构 — 封装 PTY + alacritty 终端 + grid diff ring (§3.3)
pub struct Pane {
    /// Pane 唯一 ID (§3.10 PaneInfo.id)
    pub id: String,
    /// 工作目录 (§3.10 PaneInfo.cwd)
    pub cwd: String,
    /// 标题 (§3.10 PaneInfo.title)
    pub title: Arc<parking_lot::RwLock<String>>,
    /// Shell 命令 (§3.10 PaneInfo.command)
    pub command: Option<String>,
    /// alacritty 终端实例引用 (§3.1 server-canonical)
    pub terminal: Arc<parking_lot::Mutex<AlacrittyTerminal>>,
    /// 网格生成计数器 (§3.3)
    pub generation: AtomicU64,
    /// Grid diff ring (§3.3 默认 64 entries)
    pub grid_diff_ring: Arc<parking_lot::RwLock<GridDiffRing>>,
    /// 进程存活状态 (§3.5)
    pub alive: AtomicBool,
    /// 终端尺寸
    pub cols: u32,
    pub rows: u32,
    /// §16.6 Bracketed paste 模式 (ESC [ ? 2004 h / l)
    pub bracketed_paste_mode: AtomicBool,
}

/// alacritty 终端包装器
/// §3.1: mux_server 拥有 alacritty Term, 客户端只渲染 grid diff
pub struct AlacrittyTerminal {
    #[allow(dead_code)]
    inner: Box<dyn std::any::Any + Send + Sync>,
}

impl AlacrittyTerminal {
    fn new() -> Self {
        Self {
            inner: Box::new(()),
        }
    }
}

/// Shell 命令结构 (§3.10 ShellCommand)
#[derive(Clone, Debug, Default)]
pub struct ShellCommand {
    /// 可执行文件名或路径
    pub program: String,
    /// 命令行参数
    pub args: Vec<String>,
    /// 环境变量
    pub env: std::collections::HashMap<String, String>,
}

impl Pane {
    /// 创建新 pane (§3.10 SpawnPaneRequest)
    pub fn spawn(
        id: String,
        cwd: String,
        cols: u32,
        rows: u32,
        command: Option<ShellCommand>,
    ) -> Self {
        let command_str = command
            .as_ref()
            .map(|c| format!("{} {}", c.program, c.args.join(" ")));

        Self {
            id,
            cwd,
            title: Arc::new(parking_lot::RwLock::new(String::new())),
            command: command_str,
            terminal: Arc::new(parking_lot::Mutex::new(AlacrittyTerminal::new())),
            generation: AtomicU64::new(0),
            grid_diff_ring: Arc::new(parking_lot::RwLock::new(GridDiffRing::new(64))),
            alive: AtomicBool::new(true),
            cols,
            rows,
            bracketed_paste_mode: AtomicBool::new(false),
        }
    }

    /// 获取当前 generation (§3.3)
    pub fn get_generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    /// 增加 generation (§3.3 grid sync)
    pub fn bump_generation(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    /// §3.10 向 Pane 发送输入 (§3.10 SendInputRequest)
    pub fn write_input(&self, _data: &[u8]) -> anyhow::Result<()> {
        // §3.1: 实际实现中写入 PTY
        // 当前为桩实现
        Ok(())
    }

    /// §3.10 向 Pane 粘贴文本 (§3.10 PasteRequest)
    pub fn paste(&self, _text: &str) -> anyhow::Result<()> {
        // §3.1: 实际实现中写入 PTY
        Ok(())
    }

    /// §3.3 获取 grid diff (FetchGridUpdateRequest)
    pub fn fetch_grid_update(&self, since_generation: u64) -> crate::grid_sync::GridUpdate {
        self.grid_diff_ring
            .read()
            .fetch_update(since_generation, self)
    }

    /// §3.3 获取完整 grid 快照 (FullGridSnapshot)
    pub fn get_full_snapshot(&self) -> crate::grid_sync::FullGridSnapshot {
        crate::grid_sync::build_empty_snapshot(self.cols, self.rows)
    }

    /// §3.10 调整 pane 尺寸 (ResizePaneRequest)
    pub fn resize(&mut self, cols: u32, rows: u32) {
        self.cols = cols;
        self.rows = rows;
        // §3.1: 实际实现中通知 PTY TIOCSWINSZ
        self.bump_generation();
    }

    /// §3.5 进程保活: 检查进程是否存活
    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }

    /// §3.5 设置进程存活状态
    pub fn set_alive(&self, alive: bool) {
        self.alive.store(alive, Ordering::SeqCst);
    }

    /// §3.10 更新标题 (TabTitleChanged)
    pub fn set_title(&self, title: String) {
        *self.title.write() = title;
    }

    /// §3.10 获取标题
    pub fn get_title(&self) -> String {
        self.title.read().clone()
    }

    /// §16.6 检查 bracketed paste 模式是否激活
    pub fn is_bracketed_paste_active(&self) -> bool {
        self.bracketed_paste_mode.load(Ordering::SeqCst)
    }

    /// §16.6 设置 bracketed paste 模式
    pub fn set_bracketed_paste_mode(&self, active: bool) {
        self.bracketed_paste_mode.store(active, Ordering::SeqCst);
    }
}

impl Drop for Pane {
    fn drop(&mut self) {
        // §3.5 进程保活: pane 关闭时标记为不存活
        // 完整实现中此处会向 PTY 子进程发送 SIGHUP
        self.alive.store(false, Ordering::SeqCst);
    }
}
