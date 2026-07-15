# Plan 27: CLI Control Interface (tmux-compatible)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Implement tmux-compatible CLI subcommands for z3rm: `ls`, `new`, `kill`, `attach`, `detach`, `split-window`, `send-keys`, `capture-pane`, `list-panes`, `select-pane`, `kill-pane`, `resize-pane`, `new-window`, `rename-window`. CLI agents (Claude Code, aider, omp) can control z3rm panes with zero learning cost.

**Architecture:** `z3rm` CLI subcommands connect to local mux_server socket via MuxDomain, issue mux_protocol RPCs, print results. Same protocol as GUI client. Key name parsing translates tmux-style keys (`Enter`, `C-c`, `Up`) to bytes.

**Dependencies:** `mux`, `mux_protocol`, `clap` (or manual arg parsing).

**Spec:** §3.10 CLI Control Interface

## Concept Mapping: z3rm vs tmux

z3rm and tmux have different internal models. The CLI translates tmux concepts to z3rm equivalents transparently:

| tmux concept | z3rm concept | CLI mapping behavior |
|---|---|---|
| **session** | **session** | 1:1 identical. `tmux new -s dev` = `z3rm new -s dev` |
| **window** (tab in a session) | **tab** | tmux "window" maps to z3rm "tab". Same as tmux's own `window` = tab concept. `new-window` creates a new tab. |
| **pane** (split inside a window) | **pane** | 1:1 identical. A pane holds one PTY. |
| **client** (terminal emulator connected) | **client** (GUI window or CLI session) | tmux client = terminal emulator. z3rm client = GUI window or CLI attach. |
| **prefix key** | **prefix key** (keymap profile) | Same concept. z3rm uses keymap profiles (§16.5). Default profile has no prefix; `tmux` profile uses `Ctrl-b`. |
| `target-session` (`-t name`) | `target-session` | 1:1. `-t dev` selects session named "dev". |
| `target-window` (`-t session:0`) | `target-tab` | tmux `session:0` = z3rm `session:tab_index`. Window index = tab index. |
| `target-pane` (`-t session:0.1`) | `target-pane` | tmux `session:0.1` = z3rm `session:tab.pane`. |
| `%N` (pane index) | pane index | z3rm also supports `%N` for pane-by-index. |

**Key differences to document for agent developers:**

1. **z3rm has no separate "window" level above tab.** tmux: session → window → pane. z3rm: session → tab → pane. They are structurally identical — "window" in tmux IS a tab. The CLI uses tmux terminology (`new-window`, `rename-window`) for compatibility but internally operates on tabs.

2. **z3rm panes are server-canonical.** tmux's pane state lives in the server too, but z3rm's grid/scrollback/layout ALL live in mux_server. `capture-pane` fetches from the server, not from any client terminal — this means it always works even when no GUI client is attached.

3. **z3rm `attach` opens a GUI window** (unlike tmux which attaches to the current terminal). For pure terminal attach (like tmux), use a future terminal-mode client or nest tmux inside z3rm. **Critical:** `z3rm attach` prints a one-line stderr confirmation and exits immediately — it does NOT block like `tmux attach`. Agents that blindly run `tmux attach` and wait will get an immediate exit instead of hanging forever. All other CLI commands (`send-keys`, `capture-pane`, `ls`, etc.) also execute and return immediately. This non-blocking behavior is by design and must be consistent across all CLI commands.

4. **Pane indexing:** tmux assigns pane indexes per-window. z3rm assigns pane IDs globally per-session (e.g., `w1:p1`, `w1:p2`, `w2:p3`). The CLI accepts both formats: `%N` (tmux-style per-window index) and `session:tab.pane` (z3rm-style global).

5. **Environment variables:** z3rm sets `Z3RM_SESSION` and `Z3RM_PANE` (vs tmux's `TMUX`). Agents can detect z3rm by checking for these env vars.

---

### Task 1: Define CLI subcommands

**Files:**
- Create: `crates/z3rm/src/cli/` directory
- Create: `crates/z3rm/src/cli/mod.rs`

- [ ] **Step 1: Define CLI command enum and parsing**

```rust
// CLI 控制接口
// 来源: spec §3.10 — tmux 兼容的 CLI 命令，让 agent 零学习成本操控 z3rm

pub enum CliCommand {
    /// `z3rm ls` — 列出所有 session
    ListSessions,
    /// `z3rm new -s <name>` — 创建新 session
    NewSession { name: Option<String>, cwd: Option<PathBuf> },
    /// `z3rm kill -t <target>` — 终止 session
    KillSession { target: String },
    /// `z3rm attach -t <target>` — 连接到 session (打开 GUI)
    Attach { target: Option<String> },
    /// `z3rm detach` — 断开当前 client
    Detach,
    /// `z3rm split-window -t <target> [-h|-v]` — 分割 pane
    SplitWindow { target: Option<String>, horizontal: bool, command: Option<String> },
    /// `z3rm send-keys -t <target> <keys...>` — 发送输入到 pane
    SendKeys { target: Option<String>, keys: Vec<String> },
    /// `z3rm capture-pane -t <target> [-p] [-S <-N>] [-e]` — 捕获 pane 内容
    CapturePane { target: Option<String>, print: bool, scrollback: Option<i32>, escape: bool },
    /// `z3rm list-panes -t <target>` — 列出 session 中的 pane
    ListPanes { target: Option<String> },
    /// `z3rm select-pane -t <target>` — 聚焦 pane
    SelectPane { target: Option<String> },
    /// `z3rm kill-pane -t <target>` — 关闭 pane
    KillPane { target: Option<String> },
    /// `z3rm resize-pane -t <target> -x <W> -y <H>` — 调整 pane 大小
    ResizePane { target: Option<String>, width: Option<u16>, height: Option<u16> },
    /// `z3rm new-window -t <target>` — 创建新 tab
    NewWindow { target: Option<String> },
    /// `z3rm rename-window -t <target> <title>` — 设置 pane 标题
    RenameWindow { target: Option<String>, title: String },
}
```

---

### Task 2: Key name parser

**Files:**
- Create: `crates/z3rm/src/cli/keys.rs`

- [ ] **Step 1: Implement tmux-style key name parsing**

```rust
// tmux 兼容的按键名解析
// 来源: spec §3.10 — send-keys 接受 tmux 风格按键名

pub fn parse_key(name: &str) -> Vec<u8> {
    match name {
        "Enter" | "Return" => b"\r".to_vec(),
        "Tab" => b"\t".to_vec(),
        "BSpace" => b"\x7f".to_vec(),
        "Escape" => b"\x1b".to_vec(),
        "Space" => b" ".to_vec(),
        "Up" => b"\x1b[A".to_vec(),
        "Down" => b"\x1b[B".to_vec(),
        "Right" => b"\x1b[C".to_vec(),
        "Left" => b"\x1b[D".to_vec(),
        "Home" => b"\x1b[H".to_vec(),
        "End" => b"\x1b[F".to_vec(),
        "PageUp" => b"\x1b[5~".to_vec(),
        "PageDown" => b"\x1b[6~".to_vec(),
        // C-c → Ctrl+C
        s if s.starts_with("C-") && s.len() == 3 => {
            let c = s.as_bytes()[2].to_ascii_lowercase();
            vec![c.wrapping_sub(b'a').wrapping_add(1)] // Ctrl+A = 0x01, etc.
        }
        // M-x → Alt+X (ESC followed by x)
        s if s.starts_with("M-") && s.len() == 3 => {
            vec![0x1b, s.as_bytes()[2]]
        }
        // Literal text
        _ => name.as_bytes().to_vec(),
    }
}
```

---

### Task 3: Target specifier parser

**Files:**
- Create: `crates/z3rm/src/cli/target.rs`

- [ ] **Step 1: Parse tmux-style target strings**

```rust
// tmux 风格的目标 specifier 解析
// 来源: spec §3.10 — 支持 session_name, session:window.pane, %N 格式

pub enum Target {
    Session(String),
    PaneInSession { session: String, window: u32, pane: u32 },
    PaneByIndex(u32),
    Current, // 不指定 target，使用当前 focused pane
}

pub fn parse_target(s: &Option<String>) -> Target {
    match s {
        None => Target::Current,
        Some(s) if s.starts_with('%') => {
            Target::PaneByIndex(s[1..].parse().unwrap_or(0))
        }
        Some(s) if s.contains(':') && s.contains('.') => {
            // session:window.pane
            let parts: Vec<&str> = s.splitn(3, |c| c == ':' || c == '.').collect();
            Target::PaneInSession {
                session: parts[0].to_string(),
                window: parts[1].parse().unwrap_or(0),
                pane: parts[2].parse().unwrap_or(0),
            }
        }
        Some(s) => Target::Session(s.clone()),
    }
}
```

---

### Task 4: Implement capture-pane

**Files:**
- Create: `crates/z3rm/src/cli/capture.rs`

- [ ] **Step 1: Fetch grid and convert to text**

```rust
// capture-pane 实现: 从 server 拉取 grid 并转为文本
// 来源: spec §3.10 — capture-pane -p 输出 pane 可见内容

pub async fn capture_pane(
    domain: &MuxDomain,
    pane: &str,
    scrollback_lines: Option<i32>,
    preserve_ansi: bool,
) -> Result<String> {
    if let Some(n) = scrollback_lines.filter(|n| *n < 0) {
        // 包含 scrollback: 先拉取历史行
        let scrollback = domain.fetch_scrollback(pane, 0, 1, (-n) as u32).await?;
        // ... 合并 scrollback + visible grid
    }
    
    let grid = domain.fetch_grid_update(pane, 0).await?; // full snapshot
    // 将 grid cells 转为文本行
    let mut output = String::new();
    for row in 0..grid.rows {
        for col in 0..grid.cols {
            let cell = &grid.cells[row * grid.cols + col];
            if preserve_ansi {
                // 保留 ANSI 颜色码
                append_cell_with_ansi(&mut output, cell);
            } else {
                output.push_str(&cell.char);
            }
        }
        output.push('\n');
    }
    Ok(output)
}
```

---

### Task 5: Wire CLI commands to mux_protocol RPCs

**Files:**
- Create: `crates/z3rm/src/cli/dispatch.rs`

- [ ] **Step 1: Connect to daemon, dispatch command, print result**

```rust
// CLI 命令调度: 连接 daemon，执行命令，输出结果
// 来源: spec §3.10

pub async fn run_cli_command(cmd: CliCommand) -> Result<()> {
    let domain = MuxDomain::connect_local().await?;
    
    match cmd {
        CliCommand::ListSessions => {
            let sessions = domain.list_sessions().await?;
            for s in &sessions {
                println!("{}: {} windows ({} attached)", s.name, s.tab_count, s.attached_clients);
            }
        }
        CliCommand::SendKeys { target, keys } => {
            let pane = resolve_target(&domain, &target).await?;
            for key in &keys {
                let bytes = parse_key(key);
                domain.send_input(&pane, &bytes).await?;
            }
        }
        CliCommand::CapturePane { target, print, scrollback, escape } => {
            let pane = resolve_target(&domain, &target).await?;
            let text = capture_pane(&domain, &pane, scrollback, escape).await?;
            if print { print!("{}", text); } else { println!("{}", text); }
        }
        // ... 其他命令
    }
    Ok(())
}
```

---

### Task 6: Set Z3RM_SESSION and Z3RM_PANE env vars

**Files:**
- Modify: `crates/mux_server/src/pane.rs`

- [ ] **Step 1: Set env vars on PTY spawn**

When mux_server spawns a PTY, set `Z3RM_SESSION=<session_name>` and `Z3RM_PANE=<pane_id>` in the PTY's environment. Agents can read these to self-identify.

---

### Task 7: Tests

- [ ] **Step 1: Key parser unit test**

```rust
assert_eq!(parse_key("Enter"), b"\r");
assert_eq!(parse_key("C-c"), vec![3]); // Ctrl+C = 0x03
assert_eq!(parse_key("Up"), b"\x1b[A");
assert_eq!(parse_key("hello"), b"hello");
```

- [ ] **Step 2: Target parser unit test**

- [ ] **Step 3: Integration test: send-keys + capture-pane round-trip**

Spawn session → send-keys "echo hello" Enter → capture-pane → verify "hello" appears.

---

### Task 8: Commit

```bash
git add -A
git commit -m "Add tmux-compatible CLI control interface

新增 tmux 兼容的 CLI 控制接口。
支持 send-keys, capture-pane, split-window, list-panes 等 tmux 命令。
CLI agent 可零学习成本操控 z3rm pane。

来源: spec §3.10"
git push origin HEAD
```
