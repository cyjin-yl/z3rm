//! # mux
//!
//! z3rm mux client crate: connects to mux_server via local socket (or SSH),
//! sends RPC requests, receives notifications, and provides grid sync.
//!
//! 协议版本化（§3.10），基于长度前缀的二进制帧（§9），
//! 请求/响应关联通过 request_id（§9）。

use anyhow::Result;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use interprocess::local_socket::{GenericFilePath, ToFsName};
use tokio::sync::{mpsc, oneshot};

// §9 从 mux_protocol 导入所有 protobuf 类型。
use mux_protocol::{
    attach_request::AttachMode as AttachMode_,
    request::Body as RequestBody, response::Body as ResponseBody,
    split_node::SplitDirection, envelope::Payload as EnvelopePayload,
    frame, Envelope, Notification, PROTOCOL_VERSION,
    Request, Response, SessionInfo, TerminalSize, FetchGridUpdateResponse,
    FetchScrollbackResponse, AttachResponse, ShellCommand,
};

// §9 公共类型导出
pub use mux_protocol::attach_request::AttachMode;
// ============================================================================
// §9 MuxDomain: mux client 核心结构体
// ============================================================================

/// Mux 客户端域：连接到 mux_server，发送 RPC 请求，接收通知。
pub struct MuxDomain {
    inner: Arc<parking_lot::RwLock<DomainInner>>,
    /// §9 后台 I/O 任务的 handle。
    _io_handle: tokio::task::JoinHandle<()>,
    /// §9 响应路由任务的 handle。
    _routing_handle: tokio::task::JoinHandle<()>,
}

/// §9 内部状态：请求 ID 计数器、待处理请求、通知通道、写通道。
struct DomainInner {
    /// §9 下一个请求 ID。
    next_request_id: AtomicU64,
    /// §9 待处理请求映射：request_id → oneshot sender。
    pending_requests: HashMap<u64, oneshot::Sender<Response>>,
    /// §9 通知发送端（TODO: subscribe 使用 broadcast channel）。
    #[allow(dead_code)]
    notification_tx: mpsc::Sender<Notification>,
    /// §9 写通道：send_request 通过此通道发送帧数据给 I/O 任务。
    write_tx: std::sync::mpsc::Sender<Vec<u8>>,
}

// ============================================================================
// §9 MuxTransport: 传输层枚举
// ============================================================================

/// §9 传输层：本地 Unix socket 或 SSH 通道。
pub enum MuxTransport {
    /// §9 本地 Unix socket 连接。
    Local,
    /// §9 SSH 通道连接（Plan 19 实现远程连接）。
    Ssh,
}

/// §9 SSH 通道占位符（Plan 19 实现）。
pub struct SshChannel;

// ============================================================================
// §9 connect_local: 建立本地 socket 连接
// ============================================================================

/// §9 连接到本地 mux_server Unix socket。
///
/// 默认 socket 路径为 `$XDG_RUNTIME_DIR/z3rm/mux.sock` 或 `/tmp/z3rm-mux.sock`。
pub async fn connect_local(socket_path: impl AsRef<Path>) -> Result<MuxDomain> {
    // §9 使用 ToFsName trait 将路径转换为 local socket Name。
    let name = socket_path
        .as_ref()
        .to_fs_name::<GenericFilePath>()
        .map_err(|e| anyhow::anyhow!(e))?;
    let stream = interprocess::local_socket::traits::Stream::connect(name)
        .map_err(|e| anyhow::anyhow!(e))?;
    MuxDomain::connect_with_stream(stream).await
}

// ============================================================================
// §9 MuxDomain 实现
// ============================================================================

impl MuxDomain {
    /// §9 使用已有 stream 建立连接并启动后台 I/O 任务。
    pub async fn connect_with_stream(
        stream: interprocess::local_socket::Stream,
    ) -> Result<Self> {
        // §9 创建通知通道（容量 256）。
        let (notification_tx, _notification_rx) = mpsc::channel(256);
        // §9 创建写通道（std sync mpsc，用于 blocking I/O 线程消费）。
        let (write_tx, write_rx) = std::sync::mpsc::channel();
        // §9 创建响应通道（I/O 任务 → 路由任务）。
        let (response_tx, response_rx) = mpsc::unbounded_channel();

        // §9 启动后台 I/O 任务。
        let io_handle =
            Self::spawn_io_loop(stream, write_rx, response_tx, notification_tx.clone())?;

        let inner = Arc::new(parking_lot::RwLock::new(DomainInner {
            next_request_id: AtomicU64::new(1),
            pending_requests: HashMap::new(),
            notification_tx,
            write_tx,
        }));

        // §9 启动响应路由任务。
        let routing_inner = inner.clone();
        let routing_handle = tokio::spawn(Self::response_router(routing_inner, response_rx));

        Ok(MuxDomain {
            inner,
            _io_handle: io_handle,
            _routing_handle: routing_handle,
        })
    }

    /// §9 使用指定传输层创建连接（占位实现，本地 socket 优先）。
    pub async fn connect(_transport: MuxTransport) -> Result<Self> {
        Err(anyhow::anyhow!(
            "connect() with MuxTransport not yet supported; use connect_local()"
        ))
    }

    /// §9 启动后台 I/O 任务：读取帧、解析、分发响应/通知；消费写通道并写入。
    fn spawn_io_loop(
        stream: interprocess::local_socket::Stream,
        write_rx: std::sync::mpsc::Receiver<Vec<u8>>,
        response_tx: mpsc::UnboundedSender<Response>,
        notification_tx: mpsc::Sender<Notification>,
    ) -> Result<tokio::task::JoinHandle<()>> {
        let handle = tokio::task::spawn_blocking(move || {
            Self::io_loop(stream, write_rx, response_tx, notification_tx)
        });
        Ok(handle)
    }

    /// §9 I/O 循环：读取响应帧 + 消费写通道 + 分发通知。
    fn io_loop(
        mut stream: interprocess::local_socket::Stream,
        write_rx: std::sync::mpsc::Receiver<Vec<u8>>,
        response_tx: mpsc::UnboundedSender<Response>,
        notification_tx: mpsc::Sender<Notification>,
    ) {
        let mut buf = Vec::new();

        loop {
            // §9 轮询写通道（非阻塞）。
            loop {
                match write_rx.try_recv() {
                    Ok(framed) => {
                        match stream.write_all(&framed) {
                            Ok(_) => {}
                            Err(e) => {
                                tracing::error!(error = %e, "socket write error");
                                return;
                            }
                        }
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => return,
                }
            }

            // §9 读取下一帧。
            match Self::read_next_frame(&mut stream, &mut buf) {
                Ok(Some(framed)) => {
                    let envelope = match mux_protocol::unframe(&framed) {
                        Ok((env, _)) => env,
                        Err(e) => {
                            tracing::error!(error = %e, "failed to decode envelope");
                            break;
                        }
                    };

                    match envelope.payload {
                        Some(EnvelopePayload::Response(resp)) => {
                            if response_tx.send(resp).is_err() {
                                tracing::warn!("response channel closed");
                                break;
                            }
                        }
                        Some(EnvelopePayload::Notification(notif)) => {
                            if notification_tx.blocking_send(notif).is_err() {
                                tracing::warn!("notification channel closed");
                                break;
                            }
                        }
                        Some(EnvelopePayload::Request(_)) => {
                            tracing::trace!("unexpected request from server");
                        }
                        None => {
                            tracing::warn!("envelope with no payload");
                        }
                    }
                }
                Ok(None) => {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                Err(e) => {
                    tracing::error!(error = %e, "socket read error");
                    break;
                }
            }
        }
    }

    /// §9 读取下一帧数据。
    fn read_next_frame(
        stream: &mut interprocess::local_socket::Stream,
        buf: &mut Vec<u8>,
    ) -> std::io::Result<Option<Vec<u8>>> {
        let (frame_len, header_len) = match Self::try_parse_frame_header(buf) {
            Some(ok) => ok,
            None => {
                let mut read_buf = [0u8; 256];
                match stream.read(&mut read_buf) {
                    Ok(0) => return Ok(None),
                    Ok(n) => buf.extend_from_slice(&read_buf[..n]),
                    Err(e) => return Err(e),
                }
                match Self::try_parse_frame_header(buf) {
                    Some(ok) => ok,
                    None => return Ok(None),
                }
            }
        };

        let total_needed = header_len + frame_len as usize;
        if buf.len() < total_needed {
            let mut read_buf = [0u8; 4096];
            while buf.len() < total_needed {
                match stream.read(&mut read_buf) {
                    Ok(0) => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "connection closed",
                        ))
                    }
                    Ok(n) => buf.extend_from_slice(&read_buf[..n]),
                    Err(e) => return Err(e),
                }
            }
        }

        let frame = buf[header_len..header_len + frame_len as usize].to_vec();
        buf.drain(..total_needed);
        Ok(Some(frame))
    }

    /// §9 尝试从缓冲区解析帧头（varint 长度前缀）。
    fn try_parse_frame_header(buf: &[u8]) -> Option<(u32, usize)> {
        let mut result: u32 = 0;
        let mut shift = 0;
        for (i, &byte) in buf.iter().enumerate() {
            result |= ((byte & 0x7F) as u32) << shift;
            shift += 7;
            if byte & 0x80 == 0 {
                return Some((result, i + 1));
            }
            if shift >= 35 {
                return None;
            }
        }
        None
    }

    /// §9 响应路由任务：从 response_rx 读取响应，路由到对应的 oneshot sender。
    async fn response_router(
        inner: Arc<parking_lot::RwLock<DomainInner>>,
        mut response_rx: mpsc::UnboundedReceiver<Response>,
    ) {
        while let Some(resp) = response_rx.recv().await {
            // §9 查找对应的 oneshot sender。
            let sender = {
                let mut d = inner.write();
                d.pending_requests.remove(&resp.request_id)
            };

            // §9 路由到等待中的请求。
            match sender {
                Some(tx) => {
                    let _ = tx.send(resp);
                }
                None => {
                    tracing::trace!(
                        request_id = resp.request_id,
                        "no pending request for response"
                    );
                }
            }
        }
    }

    /// §9 分配新的 request_id。
    fn next_request_id(&self) -> u64 {
        self.inner.read().next_request_id.fetch_add(1, Ordering::SeqCst)
    }

    /// §9 发送请求并等待响应。
    async fn send_request(&self, body: RequestBody) -> Result<Response> {
        let request_id = self.next_request_id();

        let (tx, rx) = oneshot::channel();

        // §9 注册待处理请求。
        {
            let mut inner = self.inner.write();
            inner.pending_requests.insert(request_id, tx);
        }

        // §9 构建 Request 消息并编码为帧。
        let request = Request {
            request_id,
            body: Some(body),
        };
        let envelope = Envelope {
            version: Some(PROTOCOL_VERSION),
            payload: Some(EnvelopePayload::Request(request)),
        };
        let framed = frame(&envelope)?;

        // §9 通过写通道发送帧数据给 I/O 任务。
        self.inner
            .read()
            .write_tx
            .send(framed)
            .map_err(|e| anyhow::anyhow!("write channel error: {}", e))?;

        // §9 等待 oneshot 响应。
        let resp =
            tokio::time::timeout(std::time::Duration::from_secs(30), rx)
                .await
                .map_err(|_| anyhow::anyhow!("request timeout"))?
                .map_err(|_| anyhow::anyhow!("request cancelled"))?;

        // §9 检查错误。
        if let Some(ResponseBody::Error(err)) = &resp.body {
            if !err.is_empty() {
                return Err(anyhow::anyhow!("mux server error: {}", err));
            }
        }

        Ok(resp)
    }

    // ========================================================================
    // §9 Session 生命周期方法（§3.10）
    // ========================================================================

    /// §3.10 列出所有会话。
    pub async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let req = RequestBody::ListSessions(mux_protocol::ListSessionsRequest {});
        let resp = self.send_request(req).await?;
        match resp.body {
            Some(ResponseBody::Sessions(list)) => Ok(list.sessions),
            _ => Err(anyhow::anyhow!("unexpected response type for list_sessions")),
        }
    }

    /// §3.10 创建新会话，返回会话 ID。
    pub async fn create_session(&self, name: &str, cwd: &Path) -> Result<String> {
        let req = RequestBody::CreateSession(mux_protocol::CreateSessionRequest {
            name: name.to_string(),
            cwd: cwd.to_string_lossy().to_string(),
        });
        let resp = self.send_request(req).await?;
        match resp.body {
            Some(ResponseBody::Session(info)) => Ok(info.id),
            _ => Err(anyhow::anyhow!("unexpected response type for create_session")),
        }
    }

    /// §3.10 结束指定会话。
    pub async fn kill_session(&self, id: &str) -> Result<()> {
        let req = RequestBody::KillSession(mux_protocol::KillSessionRequest {
            id: id.to_string(),
        });
        let _resp = self.send_request(req).await?;
        Ok(())
    }

    /// §3.10 重命名会话。
    pub async fn rename_session(&self, id: &str, name: &str) -> Result<()> {
        let req = RequestBody::RenameSession(mux_protocol::RenameSessionRequest {
            id: id.to_string(),
            name: name.to_string(),
        });
        let _resp = self.send_request(req).await?;
        Ok(())
    }

    // ========================================================================
    // §9 Pane 生命周期方法（§3.10）
    // ========================================================================

    /// §3.10 在会话/标签页中创建新 Pane，返回 Pane ID。
    pub async fn spawn_pane(
        &self,
        session: &str,
        tab: &str,
        size: TerminalSize,
        command: Option<ShellCommand>,
        cwd: Option<&Path>,
    ) -> Result<String> {
        let req = RequestBody::SpawnPane(mux_protocol::SpawnPaneRequest {
            session_id: session.to_string(),
            tab_id: tab.to_string(),
            size: Some(size),
            command,
            cwd: cwd.map(|p| p.to_string_lossy().to_string()),
        });
        let resp = self.send_request(req).await?;
        match resp.body {
            Some(ResponseBody::PaneId(id)) => Ok(id),
            _ => Err(anyhow::anyhow!("unexpected response type for spawn_pane")),
        }
    }

    /// §3.10 拆分已有 Pane，返回新 Pane ID。
    pub async fn split_pane(&self, pane: &str, direction: SplitDirection) -> Result<String> {
        let req = RequestBody::SplitPane(mux_protocol::SplitPaneRequest {
            pane_id: pane.to_string(),
            direction: direction as i32,
        });
        let resp = self.send_request(req).await?;
        match resp.body {
            Some(ResponseBody::PaneId(id)) => Ok(id),
            _ => Err(anyhow::anyhow!("unexpected response type for split_pane")),
        }
    }

    /// §3.10 关闭 Pane。
    pub async fn close_pane(&self, pane: &str) -> Result<()> {
        let req = RequestBody::ClosePane(mux_protocol::ClosePaneRequest {
            pane_id: pane.to_string(),
        });
        let _resp = self.send_request(req).await?;
        Ok(())
    }

    /// §3.10 聚焦 Pane。
    pub async fn focus_pane(&self, pane: &str) -> Result<()> {
        let req = RequestBody::FocusPane(mux_protocol::FocusPaneRequest {
            pane_id: pane.to_string(),
        });
        let _resp = self.send_request(req).await?;
        Ok(())
    }

    /// §3.10 调整 Pane 尺寸。
    pub async fn resize_pane(&self, pane: &str, cols: u32, rows: u32) -> Result<()> {
        let req = RequestBody::ResizePane(mux_protocol::ResizePaneRequest {
            pane_id: pane.to_string(),
            cols,
            rows,
        });
        let _resp = self.send_request(req).await?;
        Ok(())
    }

    /// §3.10 设置 Pane 标题。
    pub async fn set_pane_title(&self, pane: &str, title: &str) -> Result<()> {
        let req = RequestBody::SetPaneTitle(mux_protocol::SetPaneTitleRequest {
            pane_id: pane.to_string(),
            title: title.to_string(),
        });
        let _resp = self.send_request(req).await?;
        Ok(())
    }

    // ========================================================================
    // §9 输入方法（§3.10）
    // ========================================================================

    /// §3.10 向 Pane 发送原始输入字节。
    pub async fn send_input(&self, pane: &str, bytes: &[u8]) -> Result<()> {
        let req = RequestBody::SendInput(mux_protocol::SendInputRequest {
            pane_id: pane.to_string(),
            data: bytes.to_vec(),
        });
        let _resp = self.send_request(req).await?;
        Ok(())
    }

    /// §3.10 向 Pane 粘贴文本。
    pub async fn paste(&self, pane: &str, text: &str) -> Result<()> {
        let req = RequestBody::Paste(mux_protocol::PasteRequest {
            pane_id: pane.to_string(),
            text: text.to_string(),
        });
        let _resp = self.send_request(req).await?;
        Ok(())
    }

    // ========================================================================
    // §9 Grid Sync 方法（§3.3）
    // ========================================================================

    /// §3.3 拉取自指定 generation 以来的网格变更。
    pub async fn fetch_grid_update(
        &self,
        pane: &str,
        since: u64,
    ) -> Result<FetchGridUpdateResponse> {
        let req = RequestBody::FetchGridUpdate(mux_protocol::FetchGridUpdateRequest {
            pane_id: pane.to_string(),
            since_generation: since,
        });
        let resp = self.send_request(req).await?;
        match resp.body {
            Some(ResponseBody::GridUpdate(update)) => Ok(update),
            _ => Err(anyhow::anyhow!("unexpected response type for fetch_grid_update")),
        }
    }

    /// §3.3 拉取历史滚动缓冲区。
    pub async fn fetch_scrollback(
        &self,
        pane: &str,
        from: u32,
        direction: u32,
        count: u32,
    ) -> Result<FetchScrollbackResponse> {
        let req = RequestBody::FetchScrollback(mux_protocol::FetchScrollbackRequest {
            pane_id: pane.to_string(),
            from_line: from,
            direction,
            count,
        });
        let resp = self.send_request(req).await?;
        match resp.body {
            Some(ResponseBody::Scrollback(scrollback)) => Ok(scrollback),
            _ => Err(anyhow::anyhow!("unexpected response type for fetch_scrollback")),
        }
    }

    // ========================================================================
    // §9 Attach / Detach（§3.10）
    // ========================================================================

    /// §3.10 连接会话，返回完整快照。
    pub async fn attach(&self, session: &str, mode: AttachMode_) -> Result<AttachResponse> {
        let req = RequestBody::Attach(mux_protocol::AttachRequest {
            session_id: session.to_string(),
            mode: mode as i32,
        });
        let resp = self.send_request(req).await?;
        match resp.body {
            Some(ResponseBody::Attach(resp)) => Ok(resp),
            _ => Err(anyhow::anyhow!("unexpected response type for attach")),
        }
    }

    /// §3.10 断开连接。
    pub async fn detach(&self) -> Result<()> {
        let req = RequestBody::Detach(mux_protocol::DetachRequest {});
        let _resp = self.send_request(req).await?;
        Ok(())
    }

    // ========================================================================
    // §9 订阅通知（§9）
    // ========================================================================

    /// §9 获取通知通道接收端。
    pub fn subscribe(&self) -> mpsc::Receiver<Notification> {
        // §9 TODO: 使用 broadcast channel 实现多订阅者。
        let (tx, rx) = mpsc::channel(256);
        drop(tx);
        rx
    }
}

// ============================================================================
// §9 MuxNotification: 公共通知类型别名
// ============================================================================

/// §9 通知类型别名。
pub type MuxNotification = Notification;
