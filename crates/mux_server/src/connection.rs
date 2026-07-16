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

/// 处理单个客户端连接 (§9)
pub async fn handle_connection(
    stream: UnixStream,
    sessions: Arc<parking_lot::RwLock<Vec<crate::session::Session>>>,
    db: Arc<parking_lot::Mutex<Connection>>,
) -> anyhow::Result<()> {
    let (reader, writer) = tokio::io::split(stream);

    let (notification_tx, mut notification_rx) =
        mpsc::unbounded_channel::<Notification>();

    let read_handle = tokio::spawn(async move {
        let mut reader = reader;
        loop {
            let envelope = read_envelope(&mut reader).await?;
            dispatch_envelope(&envelope, &sessions, &notification_tx, &db).await?;
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
) -> anyhow::Result<()> {
    let payload = match &envelope.payload {
        Some(p) => p,
        None => return Ok(()),
    };

    match payload {
        EnvelopePayload::Request(req) => {
            let request_id = req.request_id;
            let response = dispatch_request(req, sessions, notification_tx).await?;
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
    _notification_tx: &mpsc::UnboundedSender<Notification>,
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

    let resp_body = match body {
        RequestBody::CreateSession(r) => handle_create_session(r, sessions).await?,
        RequestBody::ListSessions(_) => handle_list_sessions(sessions).await?,
        RequestBody::KillSession(r) => handle_kill_session(r, sessions).await?,
        RequestBody::Attach(r) => handle_attach(r, sessions).await?,
        RequestBody::Detach(_) => handle_detach(sessions).await?,
        RequestBody::SpawnPane(r) => handle_spawn_pane(r, sessions).await?,
        RequestBody::SplitPane(r) => handle_split_pane(r, sessions).await?,
        RequestBody::ClosePane(r) => handle_close_pane(r, sessions).await?,
        RequestBody::FocusPane(r) => handle_focus_pane(r, sessions).await?,
        RequestBody::ResizePane(r) => handle_resize_pane(r, sessions).await?,
        RequestBody::SendInput(r) => handle_send_input(r, sessions).await?,
        RequestBody::Paste(r) => handle_paste(r, sessions).await?,
        RequestBody::FetchGridUpdate(r) => handle_fetch_grid_update(r, sessions).await?,
        RequestBody::FetchScrollback(_) => {
            return Ok(Response {
                request_id,
                body: Some(ResponseBody::Error(
                    "fetch_scrollback not implemented yet".to_string(),
                )),
            });
        }
        RequestBody::ReadFile(_) => {
            return Ok(Response {
                request_id,
                body: Some(ResponseBody::Error("read_file not implemented yet".to_string())),
            });
        }
        RequestBody::ListDir(_) => {
            return Ok(Response {
                request_id,
                body: Some(ResponseBody::Error("list_dir not implemented yet".to_string())),
            });
        }
        RequestBody::StatFile(_) => {
            return Ok(Response {
                request_id,
                body: Some(ResponseBody::Error("stat_file not implemented yet".to_string())),
            });
        }
        RequestBody::SetClipboard(_) => {
            return Ok(Response {
                request_id,
                body: Some(ResponseBody::Error(
                    "set_clipboard not implemented yet".to_string(),
                )),
            });
        }
        RequestBody::GetClipboard(_) => {
            return Ok(Response {
                request_id,
                body: Some(ResponseBody::Error(
                    "get_clipboard not implemented yet".to_string(),
                )),
            });
        }
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
    }
    Ok(ResponseBody::Error(String::new()))
}

/// §3.10 连接会话
async fn handle_attach(
    req: &AttachRequest,
    sessions: &Arc<parking_lot::RwLock<Vec<crate::session::Session>>>,
) -> anyhow::Result<ResponseBody> {
    let sessions_r = sessions.read();
    let session = sessions_r
        .iter()
        .find(|s| s.id == req.session_id)
        .ok_or_else(|| anyhow::anyhow!("session not found: {}", req.session_id))?;

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

/// §3.10 创建 pane
async fn handle_spawn_pane(
    _req: &SpawnPaneRequest,
    _sessions: &Arc<parking_lot::RwLock<Vec<crate::session::Session>>>,
) -> anyhow::Result<ResponseBody> {
    let pane_id = nanoid::nanoid!();
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

/// §3.10 发送输入
async fn handle_send_input(
    req: &SendInputRequest,
    sessions: &Arc<parking_lot::RwLock<Vec<crate::session::Session>>>,
) -> anyhow::Result<ResponseBody> {
    let sessions_r = sessions.read();
    for session in sessions_r.iter() {
        let panes = session.panes.clone();
        if let Some(pane) = panes.read().get(&req.pane_id) {
            pane.write_input(&req.data)?;
        }
    }
    Ok(ResponseBody::Error(String::new()))
}

/// §3.10 粘贴文本
async fn handle_paste(
    req: &PasteRequest,
    sessions: &Arc<parking_lot::RwLock<Vec<crate::session::Session>>>,
) -> anyhow::Result<ResponseBody> {
    let sessions_r = sessions.read();
    for session in sessions_r.iter() {
        let panes = session.panes.clone();
        if let Some(pane) = panes.read().get(&req.pane_id) {
            pane.paste(&req.text)?;
        }
    }
    Ok(ResponseBody::Error(String::new()))
}

/// §3.3 获取 grid 更新
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
