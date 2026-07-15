use gpui::{Context, Task};
use language::OutlineItem;
use multi_buffer::{Anchor, MultiBufferSnapshot};
use text::BufferId;
use theme::ActiveTheme as _;

use crate::Editor;

impl Editor {
    pub fn buffer_outline_items(
        &self,
        buffer_id: BufferId,
        cx: &mut Context<Self>,
    ) -> Task<Vec<OutlineItem<text::Anchor>>> {
        let Some(buffer) = self.buffer.read(cx).buffer(buffer_id) else {
            return Task::ready(Vec::new());
        };

        let buffer_snapshot = buffer.read(cx).snapshot();
        let syntax = cx.theme().syntax().clone();
        cx.background_executor()
            .spawn(async move { buffer_snapshot.outline(Some(&syntax)).items })
    }

    pub(super) fn uses_lsp_document_symbols(
        &self,
        _cursor: Anchor,
        _multi_buffer_snapshot: &MultiBufferSnapshot,
        _cx: &Context<Self>,
    ) -> bool {
        false
    }

    pub(super) fn lsp_symbols_at_cursor(
        &self,
        _cursor: Anchor,
        _multi_buffer_snapshot: &MultiBufferSnapshot,
        _cx: &Context<Self>,
    ) -> Option<(BufferId, Vec<OutlineItem<Anchor>>)> {
        None
    }

    pub(super) fn refresh_document_symbols(
        &mut self,
        _for_buffer: Option<BufferId>,
        _cx: &mut Context<Self>,
    ) {
        // 只读编辑器：LSP 文档符号已禁用。
    }
}
