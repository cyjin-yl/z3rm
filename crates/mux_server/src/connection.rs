// §9 Connection 模块 — mux_protocol 消息分发、帧编码/解码、通知广播。
// 每个客户端连接一个 tokio task, 处理请求并推送通知。

use mux_protocol::proto::request::Body as RequestBody;
use prost::Message;
use mux_protocol::proto::fetch_grid_update_response::Update as FetchGridUpdateResponseUpdate;
use mux_protocol::proto::response::Body as ResponseBody;
use mux_protocol::proto::envelope::Payload as EnvelopePayload;
use mux_protocol::proto::*;
use sqlez::connection::Connection;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::sync::mpsc;

// §3.3 客户端角色 (Plan 33)
use crate::session::ClientRole;

/// §3.3 将 proto ClientRole 值映射为内部 ClientRole
pub fn proto_role_to_client_role(role: i32) -> ClientRole {
    match role {
        1 => ClientRole::ReadOnly,
        2 => ClientRole::ReadWrite,
        3 => ClientRole::Admin,
        _ => ClientRole::ReadWrite, // 未指定时默认为 ReadWrite
    }
}

/// §3.3 权限检查: 判断角色是否允许执行操作
pub fn check_permission(role: ClientRole, required: ClientRole) -> bool {
    match (role, required) {
        (ClientRole::Admin, _) => true,
        (ClientRole::ReadWrite, ClientRole::ReadWrite) |
        (ClientRole::ReadWrite, ClientRole::ReadOnly) => true,
        (ClientRole::ReadOnly, ClientRole::ReadOnly) => true,
        _ => false,
    }
}

/// 处理单个客户端连接 (§9)
pub async fn handle_connection(
    stream: UnixStream,
    sessions: Arc<parking_lot::RwLock<Vec<crate::session::Session>>>,
    db: Arc<parking_lot::Mutex<Connection>>,
    clipboard: Arc<crate::clipboard::ServerClipboard>,
) -> anyhow::Result<()> {
    let (reader, writer) = tokio::io::split(stream);

    let (notification_tx, mut notification_rx) =
        mpsc::unbounded_channel::<Notification>();

    // §3.3 客户端角色: 初始为 None, attach 后设置 (Plan 33)
    let client_role: Arc<parking_lot::Mutex<Option<ClientRole>>> =
        Arc::new(parking_lot::Mutex::new(None));

    let read_handle = tokio::spawn(async move {
        let mut reader = reader;
        loop {
            let envelope = read_envelope(&mut reader).await?;
            dispatch_envelope(&envelope, &sessions, &notification_tx, &db, &clipboard, &client_role).await?;
        }
        #[allow(unreachable_code)]
        Ok::<_, anyhow::Error>(())
    });

    // §9 通知推送循环: 向客户端推送 Notification
    let write_handle = tokio::spawn(async move {
        let mut writer = writer;
        while let Some(notification) = notification_rx.recv().await {
            let envelope = Envelope {
                version: Some(mux_protocol::PROTOCOL_VERSION.clone()),
                payload: Some(EnvelopePayload::Notification(notification)),
            };
            if let Ok(framed) = mux_protocol::frame(&envelope) {
                writer.write_all(&framed).await?;
                writer.flush().await?;
            }
        }
        Ok::<_, anyhow::Error>(())
    });

    let _ = tokio::join!(read_handle, write_handle);
    Ok(())
}

/// §9 从 socket 读取长度前缀帧, 解码 Envelope
async fn read_envelope(
    reader: &mut tokio::io::ReadHalf<UnixStream>,
) -> anyhow::Result<Envelope> {
    // 读取 varint 长度前缀 (§9)
    let mut len: u64 = 0;
    let mut shift: u32 = 0;

    loop {
        let byte = reader.read_u8().await?;
        len |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }

    // 读取 payload 数据
    let mut data = vec![0u8; len as usize];
    reader.read_exact(&mut data).await?;

    // 解码 Envelope
    let envelope = Envelope::decode_length_delimited(&data[..])?;
    Ok(envelope)
}

/// §9 分发 Envelope 到请求/通知处理器
async fn dispatch_envelope(
    envelope: &Envelope,
    sessions: &Arc<parking_lot::RwLock<Vec<crate::session::Session>>>,
    notification_tx: &mpsc::UnboundedSender<Notification>,
    _db: &Arc<parking_lot::Mutex<Connection>>,
    clipboard: &Arc<crate::clipboard::ServerClipboard>,
    client_role: &Arc<parking_lot::Mutex<Option<ClientRole>>>,
) -> anyhow::Result<()> {
    let payload = match &envelope.payload {
        Some(p) => p,
        None => return Ok(()),
    };

    match payload {
        EnvelopePayload::Request(req) => {
            let request_id = req.request_id;
            let response = dispatch_request(req, sessions, notification_tx, clipboard, client_role).await?;
            send_response(response, request_id).await?;
        }
        EnvelopePayload::Response(_) => {
            tracing::warn!("unexpected Response from client");
        }
        EnvelopePayload::Notification(_) => {
            tracing::warn!("unexpected Notification from client");
        }
    }

    Ok(())
}
/// §9 分发请求到具体处理器
async fn dispatch_request(
    req: &Request,
    sessions: &Arc<parking_lot::RwLock<Vec<crate::session::Session>>>,
    notification_tx: &mpsc::UnboundedSender<Notification>,
    clipboard: &Arc<crate::clipboard::ServerClipboard>,
    client_role: &Arc<parking_lot::Mutex<Option<ClientRole>>>,
) -> anyhow::Result<Response> {
    let request_id = req.request_id;

    let body = match &req.body {
        Some(b) => b,
        None => {
            return Ok(Response {
                request_id,
                body: Some(ResponseBody::Error("empty request body".to_string())),
            });
        }
    };

    // §3.3 获取当前客户端角色，未 attach 时默认为 ReadOnly (Plan 33)
    let role = client_role.lock().unwrap_or(ClientRole::ReadOnly);

    let resp_body = match body {
        // §3.3 无权限要求的操作
        RequestBody::CreateSession(r) => handle_create_session(r, sessions).await?,
        RequestBody::ListSessions(_) => handle_list_sessions(sessions).await?,
        RequestBody::Attach(r) => handle_attach(r, sessions, client_role).await?,
        RequestBody::Detach(_) => handle_detach(sessions).await?,
        RequestBody::FetchGridUpdate(r) => handle_fetch_grid_update(r, sessions).await?,
        RequestBody::FetchScrollback(r) => handle_fetch_scrollback(r, sessions).await?,
        RequestBody::SearchScrollback(r) => handle_search_scrollback(r, sessions).await?,
        RequestBody::GetClipboard(_) => handle_get_clipboard(clipboard).await?,

        // §3.3 Admin-only 操作 (Plan 33)
        RequestBody::KillSession(r) => {
            if check_permission(role, ClientRole::Admin) {
                handle_kill_session(r, sessions).await?
            } else {
                ResponseBody::Error("permission denied: admin required".to_string())
            }
        }
        RequestBody::RenameSession(_r) => {
            if check_permission(role, ClientRole::Admin) {
                ResponseBody::Error("rename_session not implemented yet".to_string())
            } else {
                ResponseBody::Error("permission denied: admin required".to_string())
            }
        }
        RequestBody::InstallExtension(_) => {
            if check_permission(role, ClientRole::Admin) {
                ResponseBody::Error("install_extension not implemented yet".to_string())
            } else {
                ResponseBody::Error("permission denied: admin required".to_string())
            }
        }
        RequestBody::NewWindow(r) => {
            if check_permission(role, ClientRole::Admin) {
                handle_new_window(r, sessions, notification_tx).await?
            } else {
                ResponseBody::Error("permission denied: admin required".to_string())
            }
        }

        // §3.3 需要 ReadWrite 的 pane 操作 (Plan 33)
        RequestBody::SpawnPane(r) => {
            if check_permission(role, ClientRole::ReadWrite) {
                handle_spawn_pane(r, sessions).await?
            } else {
                ResponseBody::Error("permission denied: read-write required".to_string())
            }
        }
        RequestBody::SplitPane(r) => {
            if check_permission(role, ClientRole::ReadWrite) {
                handle_split_pane(r, sessions).await?
            } else {
                ResponseBody::Error("permission denied: read-write required".to_string())
            }
        }
        RequestBody::ClosePane(r) => {
            if check_permission(role, ClientRole::ReadWrite) {
                handle_close_pane(r, sessions).await?
            } else {
                ResponseBody::Error("permission denied: read-write required".to_string())
            }
        }
        RequestBody::FocusPane(r) => {
            if check_permission(role, ClientRole::ReadWrite) {
                handle_focus_pane(r, sessions).await?
            } else {
                ResponseBody::Error("permission denied: read-write required".to_string())
            }
        }
        RequestBody::ResizePane(r) => {
            if check_permission(role, ClientRole::ReadWrite) {
                handle_resize_pane(r, sessions).await?
            } else {
                ResponseBody::Error("permission denied: read-write required".to_string())
            }
        }

        // §3.3 需要 ReadWrite 的输入操作 (Plan 33)
        RequestBody::SendInput(r) => {
            if check_permission(role, ClientRole::ReadWrite) {
                handle_send_input(r, sessions, clipboard, notification_tx).await?
            } else {
                ResponseBody::Error("permission denied: read-write required".to_string())
            }
        }
        RequestBody::Paste(r) => {
            if check_permission(role, ClientRole::ReadWrite) {
                handle_paste(r, sessions, clipboard, notification_tx).await?
            } else {
                ResponseBody::Error("permission denied: read-write required".to_string())
            }
        }
        RequestBody::SetClipboard(r) => {
            if check_permission(role, ClientRole::ReadWrite) {
                handle_set_clipboard(r, clipboard, notification_tx).await?
            } else {
                ResponseBody::Error("permission denied: read-write required".to_string())
            }
        }
        RequestBody::SetPaneTitle(_r) => {
            if check_permission(role, ClientRole::ReadWrite) {
                ResponseBody::Error("set_pane_title not implemented yet".to_string())
            } else {
                ResponseBody::Error("permission denied: read-write required".to_string())
            }
        }

        // §3.3 文件操作暂不限制 (Plan 33)
        RequestBody::ReadFile(_) => ResponseBody::Error("read_file not implemented yet".to_string()),
        RequestBody::ListDir(_) => ResponseBody::Error("list_dir not implemented yet".to_string()),
        RequestBody::StatFile(_) => ResponseBody::Error("stat_file not implemented yet".to_string()),
    };

    Ok(Response {
        request_id,
        body: Some(resp_body),
    })
}

/// §3.10 创建会话
async fn handle_create_session(
    req: &CreateSessionRequest,
    sessions: &Arc<parking_lot::RwLock<Vec<crate::session::Session>>>,
) -> anyhow::Result<ResponseBody> {
    let id = nanoid::nanoid!();
    let session = crate::session::Session::new(id.clone(), req.name.clone(), req.cwd.clone());
    sessions.write().push(session);

    // §16.12 记录 session 创建事件
    zlog::info!("session created: id={} name={} cwd={}", id, req.name, req.cwd);

    Ok(ResponseBody::Session(SessionInfo {
        id,
        name: req.name.clone(),
        cwd: req.cwd.clone(),
        created_timestamp: 0,
        attached_clients: 0,
    }))
}

/// §3.10 列出所有会话
async fn handle_list_sessions(
    sessions: &Arc<parking_lot::RwLock<Vec<crate::session::Session>>>,
) -> anyhow::Result<ResponseBody> {
    let sessions_r = sessions.read();
    let infos: Vec<SessionInfo> = sessions_r
        .iter()
        .map(|s| SessionInfo {
            id: s.id.clone(),
            name: s.name.clone(),
            cwd: s.cwd.clone(),
            created_timestamp: s.created_timestamp,
            attached_clients: s.attached_client_count(),
        })
        .collect();

    Ok(ResponseBody::Sessions(ListSessionsResponse { sessions: infos }))
}

/// §3.10 结束会话
async fn handle_kill_session(
    req: &KillSessionRequest,
    sessions: &Arc<parking_lot::RwLock<Vec<crate::session::Session>>>,
) -> anyhow::Result<ResponseBody> {
    let mut sessions_w = sessions.write();
    let idx = sessions_w.iter().position(|s| s.id == req.id);
    if let Some(idx) = idx {
        sessions_w.remove(idx);
        // §16.12 记录 session 销毁事件
        zlog::info!("session killed: id={}", req.id);
    } else {
        zlog::warn!("kill session not found: id={}", req.id);
    }
    Ok(ResponseBody::Error(String::new()))
}

/// §3.10 连接会话
async fn handle_attach(
    req: &AttachRequest,
    sessions: &Arc<parking_lot::RwLock<Vec<crate::session::Session>>>,
    client_role: &Arc<parking_lot::Mutex<Option<ClientRole>>>,
) -> anyhow::Result<ResponseBody> {
    let mut sessions_w = sessions.write();
    let session = sessions_w
        .iter_mut()
        .find(|s| s.id == req.session_id)
        .ok_or_else(|| anyhow::anyhow!("session not found: {}", req.session_id))?;

    // §16.12 记录客户端 attach 事件
    zlog::info!("client attached: session={} mode={:?}", req.session_id, req.mode);

    // §3.3 解析客户端身份 (Plan 33)
    let client_id = if let Some(identity) = &req.identity {
        if !identity.client_id.is_empty() {
            identity.client_id.clone()
        } else {
            format!("client-{}", std::process::id())
        }
    } else {
        format!("client-{}", std::process::id())
    };

    // §3.3 从 ClientIdentity 提取角色 (Plan 33)
    let role = if let Some(identity) = &req.identity {
        proto_role_to_client_role(identity.role)
    } else {
        ClientRole::ReadWrite // 无 identity 时默认 ReadWrite
    };

    // §3.3 将角色写入连接级状态 (Plan 33)
    *client_role.lock() = Some(role);

    let mode = match req.mode {
        0 => crate::session::AttachMode::Shared,
        1 => crate::session::AttachMode::Shared,
        2 => crate::session::AttachMode::Steal,
        3 => crate::session::AttachMode::ReadOnly,
        _ => crate::session::AttachMode::Shared,
    };
    session.add_attached_client(client_id, mode, role);

    // §3.3 将窗口 ID 注册到会话 (Plan 32)
    if !req.window_id.is_empty() {
        session.add_window(req.window_id.clone());
    }

    Ok(ResponseBody::Attach(AttachResponse {
        snapshot: Some(SessionSnapshot {
            session_id: session.id.clone(),
            focused_pane_id: session.focused_pane.clone().unwrap_or_default(),
            focused_tab_id: session.focused_tab.clone().unwrap_or_default(),
            tabs: Vec::new(),
            layout: Some(LayoutTree { root: None }),
        }),
    }))
}

/// §3.10 断开连接
async fn handle_detach(
    _sessions: &Arc<parking_lot::RwLock<Vec<crate::session::Session>>>,
) -> anyhow::Result<ResponseBody> {
    Ok(ResponseBody::Error(String::new()))
}

/// §3.3 在现有会话中创建新窗口 (Plan 32)
async fn handle_new_window(
    req: &NewWindowRequest,
    sessions: &Arc<parking_lot::RwLock<Vec<crate::session::Session>>>,
    notification_tx: &mpsc::UnboundedSender<Notification>,
) -> anyhow::Result<ResponseBody> {
    let mut sessions_w = sessions.write();
    let session = sessions_w
        .iter_mut()
        .find(|s| s.id == req.session_id)
        .ok_or_else(|| anyhow::anyhow!("session not found: {}", req.session_id))?;

    // §3.3 生成新窗口 ID
    let window_id = format!("win-{}", nanoid::nanoid!());

    // §3.3 将窗口添加到会话
    session.add_window(window_id.clone());

    // §16.12 记录新窗口创建事件
    zlog::info!(
        "new window created: session={} window={}",
        req.session_id,
        window_id
    );

    // §3.3 广播 WindowAdded 通知到所有已连接窗口
    let notify = Notification {
        event: Some(
            mux_protocol::proto::notification::Event::WindowAdded(
                mux_protocol::WindowAdded {
                    window_id: window_id.clone(),
                    session_id: req.session_id.clone(),
                },
            ),
        ),
    };
    let _ = notification_tx.send(notify);

    // §3.3 返回新窗口信息与会话快照
    Ok(ResponseBody::NewWindow(NewWindowResponse {
        window_id,
        snapshot: Some(SessionSnapshot {
            session_id: session.id.clone(),
            focused_pane_id: session.focused_pane.clone().unwrap_or_default(),
            focused_tab_id: session.focused_tab.clone().unwrap_or_default(),
            tabs: Vec::new(),
            layout: Some(LayoutTree { root: None }),
        }),
    }))
}

/// §3.10 创建 pane
async fn handle_spawn_pane(
    _req: &SpawnPaneRequest,
    _sessions: &Arc<parking_lot::RwLock<Vec<crate::session::Session>>>,
) -> anyhow::Result<ResponseBody> {
    let pane_id = nanoid::nanoid!();
    // §16.12 记录 pane 创建事件
    zlog::info!("pane spawned: id={}", pane_id);
    Ok(ResponseBody::PaneId(pane_id))
}

/// §3.10 分割 pane
async fn handle_split_pane(
    req: &SplitPaneRequest,
    sessions: &Arc<parking_lot::RwLock<Vec<crate::session::Session>>>,
) -> anyhow::Result<ResponseBody> {
    let direction = match req.direction {
        1 => crate::layout::SplitDirection::LeftRight,
        2 => crate::layout::SplitDirection::TopBottom,
        _ => crate::layout::SplitDirection::LeftRight,
    };
    let new_pane_id = nanoid::nanoid!();

    let mut sessions_w = sessions.write();
    for session in sessions_w.iter_mut() {
        if session.layout.root.find_pane(&req.pane_id).is_some() {
            session
                .layout
                .split(&req.pane_id, new_pane_id.clone(), direction)?;
        }
    }

    Ok(ResponseBody::PaneId(new_pane_id))
}

/// §3.10 关闭 pane
async fn handle_close_pane(
    req: &ClosePaneRequest,
    sessions: &Arc<parking_lot::RwLock<Vec<crate::session::Session>>>,
) -> anyhow::Result<ResponseBody> {
    let mut sessions_w = sessions.write();
    for session in sessions_w.iter_mut() {
        let _ = session.layout.remove_pane(&req.pane_id);
    }
    Ok(ResponseBody::Error(String::new()))
}

/// §3.10 聚焦 pane
async fn handle_focus_pane(
    req: &FocusPaneRequest,
    sessions: &Arc<parking_lot::RwLock<Vec<crate::session::Session>>>,
) -> anyhow::Result<ResponseBody> {
    let mut sessions_w = sessions.write();
    for session in sessions_w.iter_mut() {
        if session.layout.root.find_pane(&req.pane_id).is_some() {
            session.set_focused_pane(req.pane_id.clone());
        }
    }
    Ok(ResponseBody::Error(String::new()))
}

/// §3.10 调整 pane 尺寸
async fn handle_resize_pane(
    req: &ResizePaneRequest,
    sessions: &Arc<parking_lot::RwLock<Vec<crate::session::Session>>>,
) -> anyhow::Result<ResponseBody> {
    let sessions_r = sessions.read();
    for session in sessions_r.iter() {
        let panes = session.panes.clone();
        if let Some(_pane) = panes.read().get(&req.pane_id) {
            // §3.10 ResizePaneRequest: 通知 pane resize
        }
    }
    Ok(ResponseBody::Error(String::new()))
}

/// §3.10 发送输入 + §16.6 OSC 52 剪贴板拦截
async fn handle_send_input(
    req: &SendInputRequest,
    sessions: &Arc<parking_lot::RwLock<Vec<crate::session::Session>>>,
    clipboard: &Arc<crate::clipboard::ServerClipboard>,
    notification_tx: &mpsc::UnboundedSender<Notification>,
) -> anyhow::Result<ResponseBody> {
    // §16.6 解析 OSC 52 序列: ESC ] 52 ; c ; <base64> BEL/ST
    let mut osc52_parser = crate::clipboard::Osc52Parser::new();
    if let Some(base64_content) = osc52_parser.feed(&req.data) {
        // §16.6 OSC 52 触发剪贴板更新并通知所有客户端
        let origin_host = std::env::var("HOSTNAME")
            .unwrap_or_else(|_| "z3rm-server".to_string());
        clipboard.set_from_osc52(&base64_content, origin_host, notification_tx)?;
        // OSC 52 序列已被消费, 不转发到 PTY
        return Ok(ResponseBody::Error(String::new()));
    }

    // §16.6 检查 bracketed paste 模式切换序列
    // ESC [ ? 2004 h (enable) / ESC [ ? 2004 l (disable)
    const BRACKETED_PASTE_ENABLE: &[u8] = &[0x1B, b'[', b'?', b'2', b'0', b'0', b'4', b'h'];
    const BRACKETED_PASTE_DISABLE: &[u8] = &[0x1B, b'[', b'?', b'2', b'0', b'0', b'4', b'l'];
    if req.data == BRACKETED_PASTE_ENABLE {
        // §16.6 启用 bracketed paste
        let sessions_r = sessions.read();
        for session in sessions_r.iter() {
            let panes = session.panes.clone();
            if let Some(pane) = panes.read().get(&req.pane_id) {
                pane.set_bracketed_paste_mode(true);
            }
        }
        return Ok(ResponseBody::Error(String::new()));
    }
    if req.data == BRACKETED_PASTE_DISABLE {
        // §16.6 禁用 bracketed paste
        let sessions_r = sessions.read();
        for session in sessions_r.iter() {
            let panes = session.panes.clone();
            if let Some(pane) = panes.read().get(&req.pane_id) {
                pane.set_bracketed_paste_mode(false);
            }
        }
        return Ok(ResponseBody::Error(String::new()));
    }

    // §3.10 普通输入: 转发到 PTY
    let sessions_r = sessions.read();
    for session in sessions_r.iter() {
        let panes = session.panes.clone();
        if let Some(pane) = panes.read().get(&req.pane_id) {
            pane.write_input(&req.data)?;
        }
    }
    Ok(ResponseBody::Error(String::new()))
}

/// §3.10 粘贴文本 + §16.6 bracketed paste 包裹
async fn handle_paste(
    req: &PasteRequest,
    sessions: &Arc<parking_lot::RwLock<Vec<crate::session::Session>>>,
    _clipboard: &Arc<crate::clipboard::ServerClipboard>,
    _notification_tx: &mpsc::UnboundedSender<Notification>,
) -> anyhow::Result<ResponseBody> {
    let sessions_r = sessions.read();
    for session in sessions_r.iter() {
        let panes = session.panes.clone();
        if let Some(pane) = panes.read().get(&req.pane_id) {
            // §16.6 如果 bracketed paste 模式激活, 包裹内容
            let text = crate::clipboard::wrap_bracketed_paste(
                &req.text,
                pane.is_bracketed_paste_active(),
            );
            pane.paste(&text)?;
        }
    }
    Ok(ResponseBody::Error(String::new()))
}

/// §16.6 设置剪贴板
async fn handle_set_clipboard(
    req: &SetClipboardRequest,
    clipboard: &Arc<crate::clipboard::ServerClipboard>,
    notification_tx: &mpsc::UnboundedSender<Notification>,
) -> anyhow::Result<ResponseBody> {
    // §16.6 从 proto 消息转换并设置剪贴板
    let entry = match &req.entry {
        Some(proto_entry) => crate::clipboard::ClipboardEntry::from_proto(proto_entry),
        None => {
            return Ok(ResponseBody::Error("empty clipboard entry".to_string()));
        }
    };
    clipboard.set_clipboard(entry, notification_tx);
    Ok(ResponseBody::Error(String::new()))
}

/// §16.6 获取剪贴板
async fn handle_get_clipboard(
    clipboard: &Arc<crate::clipboard::ServerClipboard>,
) -> anyhow::Result<ResponseBody> {
    let entry = clipboard.get_clipboard();
    match entry {
        Some(entry) => {
            let proto_entry = entry.to_proto();
            Ok(ResponseBody::Clipboard(GetClipboardResponse {
                entry: Some(proto_entry),
            }))
        }
        None => {
            Ok(ResponseBody::Clipboard(GetClipboardResponse {
                entry: Some(mux_protocol::proto::ClipboardEntry {
                    content_type: mux_protocol::proto::clipboard_entry::ClipboardContentType::Text as i32,
                    data: Vec::new(),
                    origin_host: String::new(),
                }),
            }))
        }
    }
}

/// §3.3 获取 grid 更新
/// §16.9 获取回滚缓冲区历史行
async fn handle_fetch_scrollback(
    req: &FetchScrollbackRequest,
    sessions: &Arc<parking_lot::RwLock<Vec<crate::session::Session>>>,
) -> anyhow::Result<ResponseBody> {
    let sessions_r = sessions.read();
    for session in sessions_r.iter() {
        let panes = session.panes.clone();
        if let Some(pane) = panes.read().get(&req.pane_id) {
            let (lines, total, version) = pane.fetch_scrollback(
                req.from_line,
                req.direction,
                req.count,
            );
            let resp = FetchScrollbackResponse {
                lines: lines
                    .into_iter()
                    .map(|r| RowChange {
                        row: r.row,
                        cells: r.cells
                            .into_iter()
                            .map(|c| Cell {
                                char: c.character,
                                style: Some(CellStyle {
                                    bold: c.style.bold,
                                    italic: c.style.italic,
                                    underline: c.style.underline,
                                    strikethrough: c.style.strikethrough,
                                    dim: c.style.dim,
                                    reverse: c.style.reverse,
                                }),
                                foreground: c.foreground,
                                background: c.background,
                            })
                            .collect(),
                    })
                    .collect(),
                total_lines: total,
                scrollback_version: version,
            };
            return Ok(ResponseBody::Scrollback(resp));
        }
    }
    Ok(ResponseBody::Error("pane not found".to_string()))
}

/// §16.9 搜索回滚缓冲区
async fn handle_search_scrollback(
    req: &SearchScrollbackRequest,
    sessions: &Arc<parking_lot::RwLock<Vec<crate::session::Session>>>,
) -> anyhow::Result<ResponseBody> {
    let sessions_r = sessions.read();
    for session in sessions_r.iter() {
        let panes = session.panes.clone();
        if let Some(pane) = panes.read().get(&req.pane_id) {
            let (matches, version) = pane.search_scrollback(
                &req.regex,
                req.from_line,
                req.direction,
                req.max_results,
            );
            let resp = SearchScrollbackResponse {
                matches: matches
                    .into_iter()
                    .map(|(line_num, row)| SearchMatch {
                        line_number: line_num,
                        context: row.cells
                            .into_iter()
                            .map(|c| Cell {
                                char: c.character,
                                style: Some(CellStyle {
                                    bold: c.style.bold,
                                    italic: c.style.italic,
                                    underline: c.style.underline,
                                    strikethrough: c.style.strikethrough,
                                    dim: c.style.dim,
                                    reverse: c.style.reverse,
                                }),
                                foreground: c.foreground,
                                background: c.background,
                            })
                            .collect(),
                    })
                    .collect(),
                scrollback_version: version,
            };
            return Ok(ResponseBody::SearchScrollback(resp));
        }
    }
    Ok(ResponseBody::Error("pane not found".to_string()))
}

async fn handle_fetch_grid_update(
    req: &FetchGridUpdateRequest,
    sessions: &Arc<parking_lot::RwLock<Vec<crate::session::Session>>>,
) -> anyhow::Result<ResponseBody> {
    let sessions_r = sessions.read();
    for session in sessions_r.iter() {
        let panes = session.panes.clone();
        if let Some(pane) = panes.read().get(&req.pane_id) {
            let update = pane.fetch_grid_update(req.since_generation);
            let resp = match update {
                crate::grid_sync::GridUpdate::Diff {
                    from_generation,
                    to_generation,
                    diff,
                } => FetchGridUpdateResponse {
                    from_generation,
                    to_generation,
                    update: Some(
                        FetchGridUpdateResponseUpdate::Diff(GridDiff {
                            rows: diff
                                .rows
                                .into_iter()
                                .map(|r| RowChange {
                                    row: r.row,
                                    cells: r.cells.into_iter().map(|c| Cell {
                                        char: c.character,
                                        style: Some(CellStyle {
                                            bold: c.style.bold,
                                            italic: c.style.italic,
                                            underline: c.style.underline,
                                            strikethrough: c.style.strikethrough,
                                            dim: c.style.dim,
                                            reverse: c.style.reverse,
                                        }),
                                        foreground: c.foreground,
                                        background: c.background,
                                    })
                                    .collect(),
                                })
                                .collect(),
                        }),
                    ),
                },
                crate::grid_sync::GridUpdate::FullSnapshot {
                    to_generation,
                    snapshot,
                } => FetchGridUpdateResponse {
                    from_generation: 0,
                    to_generation,
                    update: Some(FetchGridUpdateResponseUpdate::FullSnapshot(
                        FullGridSnapshot {
                            cols: snapshot.cols,
                            rows: snapshot.rows,
                            cells: snapshot
                                .cells
                                .into_iter()
                                .map(|c| Cell {
                                    char: c.character,
                                    style: Some(CellStyle {
                                        bold: c.style.bold,
                                        italic: c.style.italic,
                                        underline: c.style.underline,
                                        strikethrough: c.style.strikethrough,
                                        dim: c.style.dim,
                                        reverse: c.style.reverse,
                                    }),
                                    foreground: c.foreground,
                                    background: c.background,
                                })
                                .collect(),
                            cursor: Some(CursorState {
                                col: snapshot.cursor.col,
                                row: snapshot.cursor.row,
                                style: match snapshot.cursor.style {
                                    crate::grid_sync::CursorShape::Block => 1,
                                    crate::grid_sync::CursorShape::Bar => 2,
                                    crate::grid_sync::CursorShape::Underline => 3,
                                },
                                visible: snapshot.cursor.visible,
                            }),
                            alternate_screen: snapshot.alternate_screen,
                        },
                    )),
                },
                crate::grid_sync::GridUpdate::NoChange(current_gen) => FetchGridUpdateResponse {
                    from_generation: current_gen,
                    to_generation: current_gen,
                    update: None,
                },
            };
            return Ok(ResponseBody::GridUpdate(resp));
        }
    }
    Ok(ResponseBody::Error("pane not found".to_string()))
}

/// §9 发送响应回客户端 (stub)
async fn send_response(_response: Response, _request_id: u64) -> anyhow::Result<()> {
    // §9: 实际实现中通过 writer 发送
    Ok(())
}
