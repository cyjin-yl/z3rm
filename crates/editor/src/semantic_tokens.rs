use std::collections::HashMap;

use gpui::{App, Context, Task};
use project::{
    lsp_store::{BufferSemanticTokens, RefreshForServer, SemanticTokenStylizer},
    project_settings::ProjectSettings,
};
use settings::{SemanticTokenRules, Settings as _};
use text::BufferId;

use crate::{Editor, actions::ToggleSemanticHighlights, display_map::SemanticTokenHighlight};

pub(super) struct SemanticTokenState {
    rules: SemanticTokenRules,
    enabled: bool,
    update_task: Task<()>,
    fetched_for_buffers: HashMap<BufferId, clock::Global>,
}

impl SemanticTokenState {
    pub(super) fn new(cx: &App, enabled: bool) -> Self {
        Self {
            rules: ProjectSettings::get_global(cx)
                .global_lsp_settings
                .semantic_token_rules
                .clone(),
            enabled,
            update_task: Task::ready(()),
            fetched_for_buffers: HashMap::default(),
        }
    }

    pub(super) fn enabled(&self) -> bool {
        self.enabled
    }

    pub(super) fn toggle_enabled(&mut self) {
        self.enabled = !self.enabled;
    }

    #[cfg(test)]
    pub(super) fn take_update_task(&mut self) -> Task<()> {
        std::mem::replace(&mut self.update_task, Task::ready(()))
    }

    pub(super) fn invalidate_buffer(&mut self, buffer_id: &BufferId) {
        self.fetched_for_buffers.remove(buffer_id);
    }

    pub(super) fn update_rules(&mut self, new_rules: SemanticTokenRules) -> bool {
        if new_rules != self.rules {
            self.rules = new_rules;
            true
        } else {
            false
        }
    }
}

impl Editor {
    pub fn supports_semantic_tokens(&self, _cx: &mut App) -> bool {
        false
    }

    pub fn toggle_semantic_highlights(
        &mut self,
        _: &ToggleSemanticHighlights,
        _window: &mut gpui::Window,
        _cx: &mut Context<Self>,
    ) {
        self.semantic_token_state.toggle_enabled();
    }

    pub(super) fn invalidate_semantic_tokens(&mut self, _buffer_id: Option<BufferId>) {
        // 只读编辑器：语义令牌已禁用。
    }

    pub(super) fn refresh_semantic_tokens(
        &mut self,
        _for_buffer: Option<BufferId>,
        _refresh: Option<RefreshForServer>,
        _cx: &mut Context<Self>,
    ) {
        // 只读编辑器：语义令牌已禁用。
    }

    pub(super) fn report_highlights_for_navigation(
        &mut self,
        _cx: &mut Context<Self>,
    ) -> Vec<SemanticTokenHighlight> {
        Vec::new()
    }
}

pub(super) fn buffer_semantic_tokens_to_highlights(
    _buffer_tokens: &BufferSemanticTokens,
    _stylizer: &SemanticTokenStylizer,
) -> Vec<SemanticTokenHighlight> {
    Vec::new()
}
