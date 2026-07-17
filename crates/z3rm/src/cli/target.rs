// tmux 风格的目标 specifier 解析
// 来源: spec §3.10 — 支持 session_name, session:window.pane, %N 格式

/// 目标类型: session、pane 索引、session:window.pane、当前焦点
#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    /// 按名称指定 session: `z3rm send-keys -t mysession`
    Session(String),
    /// 按 session:window.pane 指定 pane: `z3rm send-keys -t mysession:0.1`
    /// window = tab index, pane = pane index within that tab
    PaneInSession {
        session: String,
        window: u32,
        pane: u32,
    },
    /// 按 pane 全局索引: `z3rm send-keys -t %3`
    PaneByIndex(u32),
    /// 未指定 target, 使用当前 focused pane
    Current,
}

/// 解析 tmux 风格的目标字符串。
///
/// 支持格式:
/// - `None` → Current
/// - `%N` → PaneByIndex(N)
/// - `session:W.P` → PaneInSession { session, window: W, pane: P }
/// - `session` → Session(session)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_none() {
        let target = parse_target(&None);
        assert!(matches!(target, Target::Current));
    }

    #[test]
    fn test_parse_session_name() {
        let target = parse_target(&Some("mysession".to_string()));
        assert_eq!(target, Target::Session("mysession".to_string()));
    }

    #[test]
    fn test_parse_pane_index() {
        let target = parse_target(&Some("%3".to_string()));
        assert_eq!(target, Target::PaneByIndex(3));
    }

    #[test]
    fn test_parse_session_window_pane() {
        let target = parse_target(&Some("dev:0.1".to_string()));
        assert_eq!(
            target,
            Target::PaneInSession {
                session: "dev".to_string(),
                window: 0,
                pane: 1,
            }
        );
    }

    #[test]
    fn test_parse_session_window_pane_multi() {
        let target = parse_target(&Some("prod:2.5".to_string()));
        assert_eq!(
            target,
            Target::PaneInSession {
                session: "prod".to_string(),
                window: 2,
                pane: 5,
            }
        );
    }

    #[test]
    fn test_parse_pane_index_zero() {
        let target = parse_target(&Some("%0".to_string()));
        assert_eq!(target, Target::PaneByIndex(0));
    }
}
