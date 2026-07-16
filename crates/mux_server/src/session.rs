// §3.10 Session 模块 — 会话生命周期、标签页、附加客户端。
// 每个 session 包含多个 tab，每个 tab 包含多个 pane。

use crate::layout::LayoutTree;
use crate::pane::Pane;
use std::collections::HashMap;
use std::sync::Arc;

/// 会话状态 (§3.2)
#[derive(Clone)]
pub struct Session {
    /// 会话唯一 ID
    pub id: String,
    /// 会话名称 (§3.10 SessionInfo.name)
    pub name: String,
    /// 工作目录 (§3.10 SessionInfo.cwd)
    pub cwd: String,
    /// 创建时间戳 (Unix 毫秒)
    pub created_timestamp: u64,
    /// 标签页集合: tab_id → Tab
    pub tabs: HashMap<String, Tab>,
    /// 布局树 (§3.10 LayoutTree)
    pub layout: LayoutTree,
    /// 当前焦点 pane 的 ID
    pub focused_pane: Option<String>,
    /// 当前焦点 tab 的 ID
    pub focused_tab: Option<String>,
    /// 已附加的客户端列表
    pub attached_clients: Arc<parking_lot::RwLock<Vec<AttachedClient>>>,
    /// Pane 注册表: pane_id → Pane
    pub panes: Arc<parking_lot::RwLock<HashMap<String, Pane>>>,
}

/// 标签页 (§3.10 TabInfo)
#[derive(Clone, Debug)]
pub struct Tab {
    /// 标签 ID
    pub id: String,
    /// 标签标题 (§3.10 TabInfo.title)
    pub title: String,
    /// Pane ID 列表
    pub pane_ids: Vec<String>,
}

/// 附加客户端 (§3.10 AttachRequest)
#[derive(Clone, Debug)]
pub struct AttachedClient {
    /// 客户端唯一 ID
    pub client_id: String,
    /// 连接模式: shared / steal / read_only
    pub mode: AttachMode,
}

/// 连接模式 (§3.10 AttachRequest.AttachMode)
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AttachMode {
    /// 共享模式: 多个客户端可同时连接
    Shared,
    /// 抢占模式: 断开其他客户端
    Steal,
    /// 只读模式: 只能读取，不能写入
    ReadOnly,
}

impl Session {
    /// 创建新 session (§3.2)
    pub fn new(id: String, name: String, cwd: String) -> Self {
        Self {
            id,
            name,
            cwd,
            created_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            tabs: HashMap::new(),
            layout: LayoutTree::empty(),
            focused_pane: None,
            focused_tab: None,
            attached_clients: Arc::new(parking_lot::RwLock::new(Vec::new())),
            panes: Arc::new(parking_lot::RwLock::new(HashMap::new())),
        }
    }

    pub fn add_tab(&mut self, id: String, title: String) {
        let tab = Tab {
            id: id.clone(),
            title,
            pane_ids: Vec::new(),
        };
        self.tabs.insert(id, tab);
    }

    /// 获取焦点 pane 的 ID
    pub fn get_focused_pane(&self) -> Option<&str> {
        self.focused_pane.as_deref()
    }

    /// 设置焦点 pane (§3.10 FocusPaneRequest)
    pub fn set_focused_pane(&mut self, pane_id: String) {
        self.focused_pane = Some(pane_id);
    }

    /// 添加附加客户端 (§3.10 AttachRequest)
    pub fn add_attached_client(&mut self, client_id: String, mode: AttachMode) {
        let clients = self.attached_clients.clone();
        clients.write().push(AttachedClient { client_id, mode });
    }

    /// 移除附加客户端 (§3.10 DetachRequest)
    pub fn remove_attached_client(&mut self, client_id: &str) {
        let clients = self.attached_clients.clone();
        let mut clients_w = clients.write();
        clients_w.retain(|c| c.client_id != client_id);
    }

    /// 附加客户端数量 (§3.10 SessionInfo.attached_clients)
    pub fn attached_client_count(&self) -> u32 {
        self.attached_clients.read().len() as u32
    }

    /// 检查 session 是否为空 (§3.7 idle behavior)
    pub fn is_empty(&self) -> bool {
        self.panes.read().is_empty()
    }
}
