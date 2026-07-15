//! Stub types for project-crate APIs removed during dependency stripping.
//! These keep downstream crates (editor, picker, platform_title_bar) compiling.

use std::{ops::Range, path::PathBuf, sync::Arc};

use collections::BTreeMap;
use gpui::{App, Entity, SharedString, Task};
use serde::{Deserialize, Serialize};
use fs::Fs;
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

impl InlayHint {
    pub fn text(&self) -> String {
        match &self.label {
            InlayHintLabel::String(text) => text.clone(),
            InlayHintLabel::LabelParts(parts) => parts.iter().map(|p| &p.value).cloned().collect(),
        }
    }
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

#[derive(Clone, Debug)]
pub struct DocumentHighlight {
    pub range: std::ops::Range<text::Anchor>,
    pub kind: lsp::DocumentHighlightKind,
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

/// Stub: navigation kind (from editor::GotoDefinitionKind)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GotoDefinitionKind {
    Symbol,
    Declaration,
    Type,
    Implementation,
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
    // 来源: spec §2.1 — settings 访问需要 get_global 方法
    pub fn get_global(_cx: &gpui::App) -> Self {
        Self::default()
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
// Diagnostic summary stubs (spec §8.2 M2)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiagnosticSummary {
    pub warning_count: usize,
    pub error_count: usize,
}

// ---------------------------------------------------------------------------
// Directory lister stub (spec §8.2 M2)
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub enum DirectoryLister {
    Local(Arc<dyn Fs>),
    Project(Arc<Project>),
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
        ) -> Task<anyhow::Result<std::collections::HashMap<gpui::Entity<language::Buffer>, (Vec<std::ops::Range<text::Point>>)>>> {
            Task::ready(Ok(std::collections::HashMap::new()))
        }

        /// Stub: toggle bookmark (bookmark 模块已删除)
        pub fn toggle_bookmark(
            _buffer: Entity<language::Buffer>,
            _anchor: text::Anchor,
            _label: String,
            _cx: &mut gpui::Context<Self>,
        ) {
        }

        /// Stub: bookmarks for buffer (bookmark 模块已删除)
        pub fn bookmarks_for_buffer(
            _store: Entity<BookmarkStore>,
            _buffer: &Entity<language::Buffer>,
            _cx: &mut gpui::Context<Entity<BookmarkStore>>,
        ) -> Task<anyhow::Result<Vec<text::Anchor>>> {
            Task::ready(Ok(Vec::new()))
        }

        /// Stub: find bookmark (bookmark 模块已删除)
        pub fn find_bookmark(
            _store: Entity<BookmarkStore>,
            _buffer: Entity<language::Buffer>,
            _point: text::Point,
            _cx: &gpui::App,
        ) -> Option<text::Anchor> {
            None
        }

        pub fn edit_bookmark(
            &mut self,
            _buffer: &Entity<language::Buffer>,
            _anchor: text::Anchor,
            _label: String,
            _cx: &mut gpui::Context<Self>,
        ) {
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

        #[derive(Clone, Copy, Debug)]
        pub enum BreakpointState {
            Enabled,
            Disabled,
        }

        #[derive(Clone, Debug)]
        pub struct Breakpoint {
            pub state: BreakpointState,
            pub condition: Option<String>,
            pub hit_condition: Option<String>,
            pub log_point: Option<String>,
            pub message: Option<String>,
        }

        impl Breakpoint {
            pub fn new_standard() -> Self {
                Self { state: BreakpointState::Enabled, condition: None, hit_condition: None, log_point: None, message: None }
            }

            pub fn is_enabled(&self) -> bool {
                matches!(self.state, BreakpointState::Enabled)
            }

            pub fn is_disabled(&self) -> bool {
                !self.is_enabled()
            }
        }

        #[derive(Clone, Copy, Debug)]
        pub struct BreakpointSessionState {
            pub verified: bool,
        }

        #[derive(Default)]
        pub struct BreakpointStore;

        impl BreakpointStore {
            pub fn breakpoints(
                &self,
                _buffer: &Entity<language::Buffer>,
                _range: Option<Range<Anchor>>,
                _snapshot: &language::BufferSnapshot,
                _cx: &App,
            ) -> std::vec::IntoIter<(BreakpointWithPosition, Option<BreakpointSessionState>)> {
                Vec::new().into_iter()
            }

            pub fn active_position(&self) -> Option<super::super::StackFrame> {
                None
            }

            pub fn set_active_debug_pane_id(&mut self, _pane_id: gpui::EntityId) {}

            pub fn active_debug_line_pane_id(&self) -> Option<gpui::EntityId> {
                None
            }

            pub fn set_active_debug_line_pane_id(&mut self, _pane_id: gpui::EntityId) {}

            pub fn toggle_breakpoint(
                &mut self,
                _buffer: Entity<language::Buffer>,
                _breakpoint: BreakpointWithPosition,
                _edit_action: BreakpointEditAction,
                _cx: &mut gpui::Context<Self>,
            ) {
            }
        }

        #[derive(Clone, Debug)]
        pub struct BreakpointWithPosition {
            pub bp: Breakpoint,
            pub position: text::Anchor,
        }

        #[derive(Clone, Debug)]
        pub enum BreakpointEditAction {
            Toggle,
            InvertState,
            EditLogMessage(String),
            EditHitCondition(String),
            EditCondition(String),
        }
    }

    pub mod session {
        #[derive(Default)]
        pub struct Session;

        #[derive(Clone, Debug)]
        pub enum SessionEvent {
            InvalidateInlineValue,
        }

        impl gpui::EventEmitter<SessionEvent> for Session {}

        impl Session {
            pub fn any_stopped_thread(&self) -> Option<usize> {
                None
            }
        }
    }

    pub mod dap_store {
        use super::session::Session;
        use gpui::Entity;

        #[derive(Default)]
        pub struct DapStore;

        impl DapStore {
            pub fn sessions(&self) -> std::slice::Iter<'_, Entity<Session>> {
                [].iter()
            }
        }
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

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum FormatTrigger {
        Invocation,
        TypeChange,
        Save,
        Manual,
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
        ) -> Task<anyhow::Result<Option<crate::TaskVariables>>> {
            Task::ready(Ok(None))
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

impl TaskVariables {
    pub fn insert(&mut self, key: VariableName, value: String) {
        self.map.insert(key.to_string(), value);
    }
}

#[derive(Clone, Debug)]
pub enum VariableName {
    Custom(String),
}

impl std::fmt::Display for VariableName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VariableName::Custom(s) => f.write_str(s),
        }
    }
}

// ---------------------------------------------------------------------------
// Types referenced by the Project method stubs below
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct OpenLspBufferHandle;

#[derive(Clone, Debug)]
pub enum PrepareRenameResponse {
    Ready(std::ops::Range<text::Anchor>, bool),
    Success(Range<text::Anchor>),
    OnlyUnpreparedRenameSupported,
    InvalidPosition,
}

#[derive(Default)]
pub struct DapStore;

impl DapStore {
    pub fn sessions(&self) -> std::slice::Iter<'_, gpui::Entity<debugger::session::Session>> {
        [].iter()
    }
}

#[derive(Default)]
pub struct Client;

#[derive(Default)]
pub struct Telemetry;

impl Client {
    pub fn telemetry(&self) -> Arc<Telemetry> {
        Arc::new(Telemetry)
    }
}

impl Telemetry {
    pub fn log_edit_event(&self, _name: &str, _is_via_ssh: bool) {}
}

// ---------------------------------------------------------------------------
// Project method stubs for APIs removed during dependency stripping
// ---------------------------------------------------------------------------

use crate::{
    bookmark_store::BookmarkStore,
    debugger::breakpoint_store::BreakpointStore,
    Location, Project, ProjectPath, ProjectTransaction, Worktree, WorktreeId,
};
use git::blame::Blame;
use lsp::LanguageServerId;
use util::rel_path::RelPath;

#[derive(Clone, Debug)]
pub struct StackFrame { pub position: text::Point }

/// Stub for deleted remote::RemoteConnectionOptions (spec §8.2 M2)
#[derive(Clone, Debug)]
pub struct RemoteConnectionOptionsStub;

/// Stub for deleted git::Repository (spec §8.2 M2)
#[derive(Clone, Debug)]
pub struct Repository;

impl Project {
    pub fn open_buffer_by_id(
        &mut self,
        _id: text::BufferId,
        _cx: &mut gpui::Context<Self>,
    ) -> Task<anyhow::Result<Entity<language::Buffer>>> {
        Task::ready(Err(anyhow::anyhow!("stub: open_buffer_by_id")))
    }

    pub fn open_local_buffer(
        &mut self,
        _path: &std::path::Path,
        _cx: &mut gpui::Context<Self>,
    ) -> Task<anyhow::Result<Entity<language::Buffer>>> {
        Task::ready(Err(anyhow::anyhow!("stub: open_local_buffer")))
    }

    pub fn open_path(
        &mut self,
        _path: ProjectPath,
        _cx: &mut gpui::Context<Self>,
    ) -> Task<anyhow::Result<(Entity<worktree::Worktree>, Entity<language::Buffer>)>> {
        Task::ready(Err(anyhow::anyhow!("stub: open_path")))
    }

    pub fn open_local_buffer_via_lsp(
        &mut self,
        _uri: lsp::Uri,
        _server_id: LanguageServerId,
        _cx: &mut gpui::Context<Self>,
    ) -> Task<anyhow::Result<Entity<language::Buffer>>> {
        Task::ready(Err(anyhow::anyhow!("stub: open_local_buffer_via_lsp")))
    }

    pub fn find_project_path(
        &self,
        _full_path: &std::path::Path,
        _cx: &gpui::App,
    ) -> Option<ProjectPath> {
        None
    }

    pub fn find_worktree(
        &mut self,
        _abs_path: &std::path::Path,
        _cx: &mut gpui::Context<Self>,
    ) -> Option<(Entity<Worktree>, Arc<RelPath>)> {
        None
    }

    pub fn save_buffers(
        &mut self,
        _buffers: collections::HashSet<Entity<language::Buffer>>,
        _cx: &mut gpui::Context<Self>,
    ) -> Task<anyhow::Result<()>> {
        Task::ready(Ok(()))
    }

    pub fn save_buffer_as(
        &mut self,
        _buffer: Entity<language::Buffer>,
        _path: ProjectPath,
        _cx: &mut gpui::Context<Self>,
    ) -> Task<anyhow::Result<()>> {
        Task::ready(Ok(()))
    }

    pub fn reload_buffers(
        &mut self,
        _buffers: collections::HashSet<Entity<language::Buffer>>,
        _reload: bool,
        _cx: &mut gpui::Context<Self>,
    ) -> Task<anyhow::Result<ProjectTransaction>> {
        Task::ready(Ok(ProjectTransaction::default()))
    }

    pub fn blame_buffer(
        &mut self,
        _buffer: &Entity<language::Buffer>,
        _version: Option<git::Oid>,
        _cx: &mut gpui::Context<Self>,
    ) -> Task<anyhow::Result<Blame>> {
        Task::ready(Err(anyhow::anyhow!("stub: blame_buffer")))
    }

    pub fn references(
        &mut self,
        _buffer: &Entity<language::Buffer>,
        _position: text::Anchor,
        _cx: &mut gpui::Context<Self>,
    ) -> Task<anyhow::Result<Option<Vec<Location>>>> {
        Task::ready(Ok(None))
    }

    pub fn hover(
        &mut self,
        _buffer: &Entity<language::Buffer>,
        _position: text::Anchor,
        _cx: &mut gpui::Context<Self>,
    ) -> Task<anyhow::Result<Option<Vec<super::Hover>>>> {
        Task::ready(Ok(None))
    }

    pub fn document_highlights(
        &mut self,
        _buffer: &Entity<language::Buffer>,
        _position: text::Anchor,
        _cx: &mut gpui::Context<Self>,
    ) -> Task<anyhow::Result<Vec<DocumentHighlight>>> {
        Task::ready(Ok(Vec::new()))
    }


    pub fn definitions(
        &mut self,
        _buffer: &Entity<language::Buffer>,
        _position: text::Anchor,
        _kind: GotoDefinitionKind,
        _cx: &mut gpui::Context<Self>,
    ) -> Task<anyhow::Result<Option<Vec<LocationLink>>>> {
        Task::ready(Ok(None))
    }

    pub fn declarations(
        &mut self,
        _buffer: &Entity<language::Buffer>,
        _position: text::Anchor,
        _cx: &mut gpui::Context<Self>,
    ) -> Task<anyhow::Result<Option<Vec<LocationLink>>>> {
        Task::ready(Ok(None))
    }

    pub fn type_definitions(
        &mut self,
        _buffer: &Entity<language::Buffer>,
        _position: text::Anchor,
        _cx: &mut gpui::Context<Self>,
    ) -> Task<anyhow::Result<Option<Vec<LocationLink>>>> {
        Task::ready(Ok(None))
    }

    pub fn implementations(
        &mut self,
        _buffer: &Entity<language::Buffer>,
        _position: text::Anchor,
        _cx: &mut gpui::Context<Self>,
    ) -> Task<anyhow::Result<Option<Vec<LocationLink>>>> {
        Task::ready(Ok(None))
    }

    pub fn prepare_rename(
        &mut self,
        _buffer: Entity<language::Buffer>,
        _position: text::Anchor,
        _cx: &mut gpui::Context<Self>,
    ) -> Task<anyhow::Result<PrepareRenameResponse>> {
        Task::ready(Ok(PrepareRenameResponse::InvalidPosition))
    }

    pub fn apply_code_action_kind(
        &mut self,
        _buffers: collections::HashSet<Entity<language::Buffer>>,
        _kind: lsp::CodeActionKind,
        _only: bool,
        _cx: &mut gpui::Context<Self>,
    ) -> Task<anyhow::Result<()>> {
        Task::ready(Ok(()))
    }

    pub fn supports_range_formatting(
        &self,
        _buffer: &Entity<language::Buffer>,
        _cx: &gpui::App,
    ) -> bool {
        false
    }

    pub fn restart_language_servers_for_buffers(
        &mut self,
        _buffers: collections::HashSet<Entity<language::Buffer>>,
        _server_ids: collections::HashSet<LanguageServerId>,
        _restart: bool,
        _cx: &mut gpui::Context<Self>,
    ) {
    }

    pub fn stop_language_servers_for_buffers(
        &mut self,
        _buffers: collections::HashSet<Entity<language::Buffer>>,
        _server_ids: collections::HashSet<LanguageServerId>,
        _cx: &mut gpui::Context<Self>,
    ) {
    }

    pub fn cancel_language_server_work_for_buffers(
        &mut self,
        _buffers: collections::HashSet<Entity<language::Buffer>>,
        _cx: &mut gpui::Context<Self>,
    ) {
    }

    pub fn reveal_path(&mut self, _path: &std::path::Path, _cx: &mut gpui::Context<Self>) {}

    pub fn register_buffer_with_language_servers(
        &mut self,
        _buffer: &Entity<language::Buffer>,
        _cx: &mut gpui::Context<Self>,
    ) -> OpenLspBufferHandle {
        OpenLspBufferHandle
    }

    pub fn client(&self) -> &Client {
        static CLIENT: std::sync::LazyLock<Client> = std::sync::LazyLock::new(Client::default);
        &CLIENT
    }

    pub fn task_store(&self) -> Entity<crate::task_store::TaskStore> {
        unimplemented!("stub: task_store")
    }

    pub fn dap_store(&self) -> Entity<DapStore> {
        unimplemented!("stub: dap_store")
    }

    pub fn bookmark_store(&self) -> Entity<BookmarkStore> {
        unimplemented!("stub: bookmark_store")
    }

    pub fn breakpoint_store(&self) -> Entity<BreakpointStore> {
        unimplemented!("stub: breakpoint_store")
    }

    pub fn active_debug_session(
        &self,
        _cx: &gpui::App,
    ) -> Option<(Entity<crate::debugger::session::Session>, StackFrame)> {
        None
    }

    pub fn any_language_server_supports_inlay_hints(
        &mut self,
        _buffer: &language::Buffer,
    ) -> bool {
        false
    }

    pub fn any_language_server_supports_semantic_tokens(
        &mut self,
        _buffer: &language::Buffer,
    ) -> bool {
        false
    }

    pub fn inline_values(
        &mut self,
        _session: Entity<crate::debugger::session::Session>,
        _stack_frame: StackFrame,
        _buffer_handle: Entity<language::Buffer>,
        _range: Range<text::Anchor>,
        _cx: &mut gpui::Context<Self>,
    ) -> Task<anyhow::Result<Vec<InlayHint>>> {
        Task::ready(Ok(Vec::new()))
    }

    pub fn visible_worktrees(&self, _cx: &gpui::App) -> impl Iterator<Item = Entity<Worktree>> {
        std::iter::empty::<Entity<Worktree>>()
    }

    // --- Stub methods for deleted diagnostic/remote features (spec §8.2 M2) ---

    pub fn diagnostic_summary(&self, _warnings: bool, _cx: &App) -> DiagnosticSummary {
        DiagnosticSummary::default()
    }

    pub fn diagnostic_summary_for_path(&self, _path: &ProjectPath, _cx: &App) -> DiagnosticSummary {
        DiagnosticSummary::default()
    }

    pub fn diagnostic_summaries(
        &mut self,
        _cx: &mut gpui::Context<Self>,
    ) -> Task<anyhow::Result<BTreeMap<WorktreeId, DiagnosticSummary>>> {
        Task::ready(Ok(BTreeMap::new()))
    }

    pub fn capability(
        &self,
        _server_id: LanguageServerId,
        _capability: language::Capability,
        _cx: &App,
    ) -> bool {
        false
    }

    pub fn is_local(&self, _cx: &App) -> bool {
        true
    }

    pub fn remote_client(&self) -> Option<Arc<Client>> {
        None
    }

    pub fn remote_connection_options(&self) -> Option<Arc<RemoteConnectionOptionsStub>> {
        None
    }

    pub fn language_servers_running_disk_based_diagnostics(
        &self,
        _cx: &App,
    ) -> Vec<lsp::LanguageServerId> {
        Vec::new()
    }

    pub fn remove_worktree(
        &mut self,
        _worktree_id: WorktreeId,
        _cx: &mut gpui::Context<Self>,
    ) -> Task<anyhow::Result<()>> {
        Task::ready(Ok(()))
    }

    pub fn repositories(&self, _cx: &App) -> Vec<Arc<Repository>> {
        Vec::new()
    }

    pub fn active_repository(
        &self,
        _path: &ProjectPath,
        _cx: &App,
    ) -> Option<Arc<Repository>> {
        None
    }

    pub fn save_buffer(
        &mut self,
        _buffer: Entity<language::Buffer>,
        _cx: &mut gpui::Context<Self>,
    ) -> Task<anyhow::Result<()>> {
        Task::ready(Ok(()))
    }

    pub fn get_open_buffer(
        &self,
        _file: &dyn language::File,
        _cx: &App,
    ) -> Option<Entity<language::Buffer>> {
        None
    }

    pub fn create_buffer(
        &mut self,
        _language: Arc<language::Language>,
        cx: &mut gpui::Context<Self>,
    ) -> gpui::Task<Entity<language::Buffer>> {
        gpui::Task::ready(panic!("stub: create_buffer"))
    }

    pub fn set_language_for_buffer(
        &mut self,
        _buffer: &Entity<language::Buffer>,
        _language: Arc<language::Language>,
        _cx: &mut gpui::Context<Self>,
    ) {
    }
}


