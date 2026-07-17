// §3.1 mux_server — mux_server 守护进程库。
// 管理 PTY、alacritty 终端模拟、layout 引擎、session 持久化。

use anyhow::Result;
use sqlez::connection::Connection;
use std::path::PathBuf;
use tokio::net::UnixListener;

pub mod connection;
pub mod clipboard;
pub mod grid_sync;
pub mod layout;
pub mod pane;
pub mod persistence;

#[cfg(test)]
mod tests;
pub mod session;

/// 默认 socket 路径: $XDG_RUNTIME_DIR/z3rm/mux.sock (§16.1)
fn default_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir)
    } else {
        PathBuf::from("/tmp")
    }
    .join("z3rm")
    .join("mux.sock")
}

/// 绑定本地 socket (§9)
fn bind_socket(path: &PathBuf) -> Result<UnixListener> {
    // 确保父目录存在
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // 删除可能存在的旧 socket
    let _ = std::fs::remove_file(path);

    // 创建 socket 文件
    let listener = UnixListener::bind(path)?;

    // 设置 0600 权限 (§9)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, perms)?;
    }

    Ok(listener)
}

/// §3.6 初始化数据库连接
fn init_database(db_path: &PathBuf) -> Result<Connection> {
    let db = Connection::open_file(db_path.to_str().unwrap_or("file::memory:?mode=memory"));
    // §3.6 初始化持久化表
    persistence::init_tables(&db)?;
    Ok(db)
}

/// 启动守护进程 (§3.1)
pub fn run() -> Result<()> {
    let socket_path = default_socket_path();
    let listener = bind_socket(&socket_path)?;
    let addr = listener.local_addr()?;
    tracing::info!(?addr, "mux_server listening");

    let db_path = dirs::runtime_dir()
        .or_else(|| Some(std::env::temp_dir().join("z3rm")))
        .unwrap_or_else(|| PathBuf::from("/tmp/z3rm"));
    std::fs::create_dir_all(&db_path)?;
    let db_path = db_path.join("z3rm.db");
    let db = init_database(&db_path)?;

    // §3.6 启动时恢复 session
    let recovered = persistence::recover_sessions(&db)?;
    tracing::info!(count = recovered.len(), "recovered sessions");

    let sessions = std::sync::Arc::new(parking_lot::RwLock::new(recovered));
    let db = std::sync::Arc::new(parking_lot::Mutex::new(db));

    // §3.6 启动持久化后台任务 (每 10s 快照)
    let sessions_clone = sessions.clone();
    let db_clone = db.clone();
    let persist_handle = tokio::spawn(async move {
        persistence::persist_loop(sessions_clone, db_clone).await;
    });

    let clipboard = std::sync::Arc::new(clipboard::ServerClipboard::new());
    let server = Server {
        sessions,
        _db: db,
        _persist_handle: Some(persist_handle),
        clipboard,
    };

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(server.run(listener))
}

/// 服务器主结构 (§3.1)
pub struct Server {
    // §3.2 session 注册表
    sessions: std::sync::Arc<parking_lot::RwLock<Vec<session::Session>>>,
    // §3.6 SQLite 持久化连接
    _db: std::sync::Arc<parking_lot::Mutex<Connection>>,
    // §3.6 持久化后台任务句柄
    _persist_handle: Option<tokio::task::JoinHandle<()>>,
    // §16.6 服务器剪贴板
    clipboard: std::sync::Arc<clipboard::ServerClipboard>,
}

impl Server {
    /// §9 监听连接并处理请求
    async fn run(self, listener: UnixListener) -> Result<()> {
        loop {
            let (stream, addr) = listener.accept().await?;
            tracing::debug!(?addr, "new connection");

            let sessions = self.sessions.clone();
            let db = self._db.clone();
            let clipboard = self.clipboard.clone();

            tokio::spawn(async move {
                if let Err(e) = connection::handle_connection(stream, sessions, db, clipboard).await {
                    tracing::error!(error = %e, "connection error");
                }
            });
        }
    }
}
