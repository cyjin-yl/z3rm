//! Stub module replacing deleted hover_popover functionality.
//! 来源: spec §8.2 M2 - broken-ref 修复

use gpui::{App, Context, SharedString, Window};
use crate::Editor;

/// 替代已删除的 hover_popover::diagnostics_markdown_style (spec §8.2 M2)
pub fn diagnostics_markdown_style(_window: &Window, _cx: &App) -> markdown::MarkdownStyle {
    markdown::MarkdownStyle::default()
}

/// Stub: open_markdown_url (spec §8.2 M2)
pub fn open_markdown_url(
    _url: SharedString,
    _editor: &mut Editor,
    _window: &mut Window,
    _cx: &mut Context<Editor>,
) {
    // Markdown preview deleted; no-op stub
}
