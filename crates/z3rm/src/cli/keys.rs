// tmux 兼容的按键名解析
// 来源: spec §3.10 — send-keys 接受 tmux 风格按键名

/// 将 tmux 风格的按键名转换为字节序列。
///
/// 支持的格式:
/// - 命名按键: `Enter`, `Tab`, `BSpace`, `Escape`, `Space`, `Up`, `Down`, `Left`, `Right`,
///   `Home`, `End`, `PageUp`, `PageDown`
/// - Ctrl 组合: `C-a` through `C-z` (以及 `C-A` through `C-Z`)
/// - Alt 组合: `M-a` through `M-z` (以及 `M-A` through `M-Z`)
/// - 字面文本: 其他字符串直接作为 UTF-8 字节
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
        // C-c → Ctrl+C = 0x03
        s if s.starts_with("C-") && s.len() == 3 => {
            let c = s.as_bytes()[2].to_ascii_lowercase();
            vec![c.wrapping_sub(b'a').wrapping_add(1)]
        }
        // M-x → Alt+X: ESC followed by x
        s if s.starts_with("M-") && s.len() == 3 => {
            vec![0x1b, s.as_bytes()[2]]
        }
        // 字面文本
        _ => name.as_bytes().to_vec(),
    }
}

/// 解析多个按键名，返回合并的字节序列。
pub fn parse_keys(names: &[String]) -> Vec<u8> {
    let mut result = Vec::new();
    for name in names {
        result.extend(parse_key(name));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_enter() {
        assert_eq!(parse_key("Enter"), b"\r");
        assert_eq!(parse_key("Return"), b"\r");
    }

    #[test]
    fn test_parse_tab() {
        assert_eq!(parse_key("Tab"), b"\t");
    }

    #[test]
    fn test_parse_backspace() {
        assert_eq!(parse_key("BSpace"), b"\x7f");
    }

    #[test]
    fn test_parse_escape() {
        assert_eq!(parse_key("Escape"), b"\x1b");
    }

    #[test]
    fn test_parse_space() {
        assert_eq!(parse_key("Space"), b" ");
    }

    #[test]
    fn test_parse_arrow_keys() {
        assert_eq!(parse_key("Up"), b"\x1b[A");
        assert_eq!(parse_key("Down"), b"\x1b[B");
        assert_eq!(parse_key("Right"), b"\x1b[C");
        assert_eq!(parse_key("Left"), b"\x1b[D");
    }

    #[test]
    fn test_parse_home_end() {
        assert_eq!(parse_key("Home"), b"\x1b[H");
        assert_eq!(parse_key("End"), b"\x1b[F");
    }

    #[test]
    fn test_parse_page_keys() {
        assert_eq!(parse_key("PageUp"), b"\x1b[5~");
        assert_eq!(parse_key("PageDown"), b"\x1b[6~");
    }

    #[test]
    fn test_parse_ctrl_keys() {
        // Ctrl+A = 0x01, Ctrl+B = 0x02, Ctrl+C = 0x03
        assert_eq!(parse_key("C-a"), vec![1]);
        assert_eq!(parse_key("C-b"), vec![2]);
        assert_eq!(parse_key("C-c"), vec![3]);
        assert_eq!(parse_key("C-d"), vec![4]);
        // 大写也有效
        assert_eq!(parse_key("C-A"), vec![1]);
        assert_eq!(parse_key("C-Z"), vec![26]);
    }

    #[test]
    fn test_parse_meta_keys() {
        // M-a = ESC + 'a'
        assert_eq!(parse_key("M-a"), vec![0x1b, b'a']);
        assert_eq!(parse_key("M-x"), vec![0x1b, b'x']);
    }

    #[test]
    fn test_parse_literal() {
        assert_eq!(parse_key("hello"), b"hello");
        assert_eq!(parse_key("foo bar"), b"foo bar");
    }

    #[test]
    fn test_parse_keys_multiple() {
        let keys = vec!["echo".to_string(), " ".to_string(), "hello".to_string(), "Enter".to_string()];
        let bytes = parse_keys(&keys);
        assert_eq!(bytes, b"echo hello\r");
    }
}
