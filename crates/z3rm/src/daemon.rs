//! §16.1 daemon 自动启动与连接管理

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use mux::MuxDomain;

/// 默认 socket 路径: $XDG_RUNTIME_DIR/z3rm/mux.sock 或 /tmp/z3rm/mux.sock (§16.1)
fn default_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir)
    } else {
        PathBuf::from("/tmp")
    }
    .join("z3rm")
    .join("mux.sock")
}

/// 确保 mux_server daemon 正在运行，返回已连接的 MuxDomain。
///
/// 流程 (§16.1):
/// 1. 尝试连接默认 socket
/// 2. 连接失败 → 启动 z3rm-server --daemonize
/// 3. 等待 socket 就绪 (最多 5s)
/// 4. 再次连接，返回 MuxDomain
pub async fn ensure_daemon_running() -> Result<MuxDomain> {
    let socket_path = default_socket_path();

    // §16.1 Step 1: 先尝试连接，daemon 可能已经在运行
    if let Ok(domain) = mux::connect_local(&socket_path).await {
        tracing::info!("connected to existing daemon");
        return Ok(domain);
    }

    // §16.1 Step 2: 连接失败，启动 daemon
    tracing::info!("daemon not running, spawning...");
    spawn_daemon()?;

    // §16.1 Step 3: 等待 socket 就绪
    wait_for_socket(&socket_path, Duration::from_secs(5)).await?;

    // §16.1 Step 4: 再次连接
    let domain = mux::connect_local(&socket_path)
        .await
        .context("failed to connect to daemon after spawn")?;
    tracing::info!("connected to daemon after spawn");
    Ok(domain)
}

/// 启动 z3rm-server daemon 进程 (§16.1)
fn spawn_daemon() -> Result<()> {
    // 优先使用 z3rm-server 命令；如果找不到则尝试 z3rm --server
    let result = Command::new("z3rm-server")
        .arg("--daemonize")
        .spawn();

    match result {
        Ok(_) => {
            tracing::info!("spawned z3rm-server --daemonize");
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // 回退: 尝试 z3rm --server
            let result = Command::new("z3rm")
                .arg("--server")
                .arg("--daemonize")
                .spawn();
            match result {
                Ok(_) => {
                    tracing::info!("spawned z3rm --server --daemonize");
                    Ok(())
                }
                Err(e2) => {
                    Err(anyhow::anyhow!(
                        "cannot spawn daemon: z3rm-server not found ({e}), z3rm --server failed ({e2})"
                    ))
                }
            }
        }
        Err(e) => {
            Err(anyhow::anyhow!("failed to spawn z3rm-server: {e}"))
        }
    }
}

/// 轮询等待 socket 文件就绪 (§16.1)
async fn wait_for_socket(socket_path: &Path, timeout: Duration) -> Result<()> {
    let start = std::time::Instant::now();
    let poll_interval = Duration::from_millis(50);

    loop {
        if start.elapsed() > timeout {
            return Err(anyhow::anyhow!(
                "timed out waiting for daemon socket at {} ({:?})",
                socket_path.display(),
                timeout
            ));
        }

        if socket_path.exists() {
            tracing::info!(
                "daemon socket ready at {} after {:?}",
                socket_path.display(),
                start.elapsed()
            );
            return Ok(());
        }

        tokio::time::sleep(poll_interval).await;
    }
}

/// 首次启动时创建默认 session (§16.1)
pub async fn ensure_default_session(domain: &MuxDomain) -> Result<String> {
    let sessions = domain.list_sessions().await?;

    if sessions.is_empty() {
        // 创建默认 session，工作目录为 home 目录
        let cwd = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        let session_id = domain.create_session("default", &cwd).await?;
        tracing::info!(session_id = %session_id, "created default session");
        Ok(session_id)
    } else {
        // 已有 session，使用第一个
        let session_id = sessions[0].id.clone();
        tracing::info!(session_id = %session_id, "using existing session");
        Ok(session_id)
    }
}
