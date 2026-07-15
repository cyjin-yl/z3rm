//! Stub types for project-crate APIs removed during dependency stripping.
//! These keep downstream crates (editor, picker, platform_title_bar) compiling.

use std::{ops::Range, path::PathBuf, sync::Arc};

use collections::BTreeMap;
use gpui::{App, Entity, SharedString, Task};
use serde::{Deserialize, Serialize};
use text::Anchor;

pub type CompletionId = u64;

// ---------------------------------------------------------------------------
// Inlay / hover types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum InlayId {
    Hint(u64),
    Color(u64),
    EditPrediction(u64),
    DebuggerValue(u64),
    ReplResult(u64),
}

#[derive(Clone, Debug)]
pub struct InlayHint {
    pub position: Anchor,
    pub label: InlayHintLabel,
    pub kind: lsp::InlayHintKind,
    pub text_edits: Option<Vec<lsp::TextEdit>>,
    pub tooltip: Option<InlayHintTooltip>,
    pub padding_before: bool,
    pub padding_after: bool,
}

#[derive(Clone, Debug)]
pub enum InlayHintLabel {
    String(String),
    LabelParts(Vec<InlayHintLabelPart>),
}

#[derive(Clone, Debug)]
pub struct InlayHintLabelPart {
    pub value: String,
    pub tooltip: Option<String>,
    pub location: Option<LocationLink>,
}

#[derive(Clone, Debug)]
pub struct InlayHintTooltip {
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct InlayHintLabelPartTooltip {
    pub text: String,
}

#[derive(Clone, Copy, Debug)]
pub enum InvalidationStrategy {
    OnBufferChange,
    OnCursorChange,
    OnFileChange,
    Never,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolveState {
    Pending,
    Resolved,
}

// ---------------------------------------------------------------------------
// Hover
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HoverBlockKind {
    PlainText,
    Markdown,
    Code { language: String },
}

#[derive(Clone, Debug)]
pub struct HoverBlock {
    pub text: String,
    pub kind: HoverBlockKind,
}

#[derive(Clone, Debug)]
pub struct Hover {
    pub contents: Vec<HoverBlock>,
    pub range: Option<Range<Anchor>>,
}

// ---------------------------------------------------------------------------
// Links / paths
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct LocationLink {
    pub origin_selection_range: Option<Range<Anchor>>,
    pub target_uri: Arc<dyn language::File>,
    pub target_range: Range<Anchor>,
    pub target_selection_range: Range<Anchor>,
}

impl std::fmt::Debug for LocationLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocationLink")
            .field("origin_selection_range", &self.origin_selection_range)
            .field("target_range", &self.target_range)
            .field("target_selection_range", &self.target_selection_range)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
pub struct ResolvedPath {
    pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct LanguageServerToQuery {
    pub server_id: lsp::LanguageServerId,
}

// ---------------------------------------------------------------------------
// AI / settings stubs
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DisableAiSettings {
    pub disable_ai: bool,
}

impl DisableAiSettings {
    pub fn is_ai_disabled_for_buffer(_buffer: Option<&language::Buffer>, _cx: &App) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// Document colors
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct DocumentColor {
    pub color: Color,
    pub range: Range<Anchor>,
}

#[derive(Clone, Debug)]
pub struct Color {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

// ---------------------------------------------------------------------------
// Symbol
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Symbol {
    pub name: String,
    pub kind: lsp::SymbolKind,
    pub range: Range<text::Point>,
}

// ---------------------------------------------------------------------------
// Bookmark store
// ---------------------------------------------------------------------------

pub mod bookmark_store {
    use super::*;
    use std::ops::Range;

    #[derive(Default)]
    pub struct BookmarkStore;

    impl BookmarkStore {
        pub fn all_bookmark_locations(
            _store: Entity<BookmarkStore>,
            _cx: &mut gpui::AsyncApp,
        ) -> Task<anyhow::Result<Vec<Anchor>>> {
            Task::ready(Ok(Vec::new()))
        }
    }
}

// ---------------------------------------------------------------------------
// Debugger
// ---------------------------------------------------------------------------

pub mod debugger {
    pub mod breakpoint_store {
        use super::super::*;
        use gpui::Entity;
        use std::ops::Range;
        use text::{Anchor, Point};

        #[derive(Clone, Debug)]
        pub struct Breakpoint {
            pub enabled: bool,
            pub condition: Option<String>,
            pub hit_condition: Option<String>,
            pub log_point: Option<String>,
        }

        #[derive(Clone, Copy, Debug)]
        pub enum BreakpointSessionState {
            Pending,
            Hit,
            PendingLog,
        }

        #[derive(Default)]
        pub struct BreakpointStore;

        impl BreakpointStore {
            pub fn breakpoints(
                &self,
                _buffer: &Entity<language::Buffer>,
                _range: Option<Range<Anchor>>,
                _cx: &App,
            ) -> Vec<(Anchor, Breakpoint, Option<BreakpointSessionState>)> {
                Vec::new()
            }

            pub fn active_position(&self) -> Option<&Point> {
                None
            }

            pub fn active_debug_line_pane_id(&self) -> Option<usize> {
                None
            }

            pub fn toggle_breakpoint(
                &mut self,
                _buffer: Entity<language::Buffer>,
                _breakpoint: BreakpointWithPosition,
                _cx: &mut gpui::Context<Self>,
            ) {
            }
        }

        #[derive(Clone, Debug)]
        pub struct BreakpointWithPosition {
            pub breakpoint: Breakpoint,
            pub position: Point,
        }
    }

    pub mod session {
        #[derive(Default)]
        pub struct Session;
    }
}

// ---------------------------------------------------------------------------
// LSP command helpers
// ---------------------------------------------------------------------------

pub mod lsp_command {
    use super::LocationLink;

    pub fn location_link_from_proto(_link: &rpc::proto::LocationLink) -> Option<LocationLink> {
        None
    }
}

// ---------------------------------------------------------------------------
// LSP store
// ---------------------------------------------------------------------------

pub mod lsp_store {
    use super::*;
    use std::ops::Range;

    pub mod lsp_ext_command {
        use text::{BufferId, Point};

        #[derive(Clone, Debug)]
        pub struct SwitchSourceHeaderResult(pub String);

        #[derive(Clone, Debug)]
        pub struct SwitchSourceHeader;

        #[derive(Clone, Debug)]
        pub struct ExpandedMacro {
            pub name: String,
            pub expansion: String,
        }

        #[derive(Clone, Debug)]
        pub struct GoToParentModule {
            pub position: Point,
        }

        #[derive(Clone, Debug)]
        pub struct OpenDocs {
            pub position: Point,
        }

        #[derive(Clone, Debug)]
        pub struct DocsUrls {
            pub web: Option<String>,
            pub local: Option<String>,
        }
    }

    pub mod rust_analyzer_ext {
        pub const RUST_ANALYZER_NAME: &str = "rust-analyzer";

        pub fn run_flycheck(_cx: &mut gpui::App) {}
        pub fn clear_flycheck(_cx: &mut gpui::App) {}
        pub fn cancel_flycheck(_cx: &mut gpui::App) {}
    }

    pub mod clangd_ext {
        pub const CLANGD_SERVER_NAME: &str = "clangd";
    }

    #[derive(Clone, Debug)]
    pub struct LspDocumentLink {
        pub range: Range<Anchor>,
        pub target: String,
    }

    #[derive(Clone, Debug)]
    pub struct ResolvedDocumentLink {
        pub buffer_id: text::BufferId,
        pub link: LspDocumentLink,
    }

    #[derive(Clone, Debug)]
    pub struct BufferDocumentLinks {
        pub links: Vec<LspDocumentLink>,
    }

    #[derive(Clone, Debug, Default)]
    pub struct LspFoldingRange {
        pub start: text::Point,
        pub end: text::Point,
        pub kind: Option<lsp::FoldingRangeKind>,
    }

    #[derive(Clone, Copy, Debug)]
    pub struct TokenType(pub u32);

    #[derive(Clone, Copy, Debug)]
    pub enum FormatTrigger {
        Invocation,
        TypeChange,
        Save,
    }

    #[derive(Clone, Debug)]
    pub struct BufferSemanticToken {
        pub range: Range<text::Anchor>,
        pub token_type: TokenType,
        pub token_modifiers: u32,
    }

    #[derive(Clone, Debug, Default)]
    pub struct BufferSemanticTokens {
        pub tokens: Vec<BufferSemanticToken>,
    }

    #[derive(Clone, Debug)]
    pub struct CacheInlayHints;

    #[derive(Clone, Debug)]
    pub struct ResolvedHint;

    #[derive(Clone, Debug)]
    pub struct RefreshForServer {
        pub server_id: lsp::LanguageServerId,
        pub request_id: usize,
    }

    #[derive(Clone, Debug, Default)]
    pub struct SemanticTokenStylizer;

    impl SemanticTokenStylizer {
        pub fn server_id(&self) -> lsp::LanguageServerId {
            lsp::LanguageServerId(0)
        }
    }

    #[derive(Default)]
    pub struct LspStore;

    impl LspStore {
        pub fn upstream_client(&self) -> Option<(anyhow::Result<()>, u64)> {
            None
        }

        pub fn last_formatting_failure(&self) -> Option<&str> {
            None
        }

        pub fn as_local(&self) -> Option<&Self> {
            Some(self)
        }

        pub fn result_id_for_buffer_pull(
            &self,
            _server_id: lsp::LanguageServerId,
            _buffer_id: text::BufferId,
            _extra: &Option<String>,
            _cx: &mut gpui::Context<Self>,
        ) -> Option<String> {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Task store
// ---------------------------------------------------------------------------

pub mod task_store {
    use super::*;
    use std::ops::Range;

    #[derive(Default)]
    pub struct TaskStore;

    impl TaskStore {
        pub fn task_inventory(&self) -> Option<Entity<TaskInventory>> {
            None
        }

        pub fn task_context_for_location(
            &self,
            _variables: crate::TaskVariables,
            _location: language::Location,
            _cx: &mut gpui::Context<Self>,
        ) -> Task<anyhow::Result<crate::TaskVariables>> {
            Task::ready(Ok(crate::TaskVariables::default()))
        }
    }

    #[derive(Default)]
    pub struct TaskInventory;
}

// ---------------------------------------------------------------------------
// Re-exported task-like types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct TaskVariables {
    pub map: BTreeMap<String, String>,
}

impl Default for TaskVariables {
    fn default() -> Self {
        Self {
            map: BTreeMap::default(),
        }
    }
}


