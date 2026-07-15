//! Stub types and helpers for APIs removed during dependency stripping.
//! Avoids duplicating definitions that remain in `project::stubs`.

use std::{any::Any, sync::Arc};

use gpui::{App, AnyElement, Entity, IntoElement, Modifiers, Pixels, ScrollHandle, SharedString, Task, TextStyle, Window, div, px};
use language::{Buffer, Location};
use project::Project;
use text::{Anchor, BufferId, ToOffset};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings::SnippetSortOrder;

// Types that were previously imported from project but no longer exist there.
// Defined as stubs locally to keep the editor crate compiling.

#[derive(Clone, Debug)]
pub struct BufferSemanticTokens;

#[derive(Clone, Debug)]
pub struct CacheInlayHints;

#[derive(Clone, Debug)]
pub struct CompletionDocumentation;

#[derive(Clone, Debug)]
pub struct DocumentHighlight {
    pub range: std::ops::Range<text::Anchor>,
    pub kind: lsp::DocumentHighlightKind,
}

#[derive(Clone, Debug)]
pub struct LspAction;

#[derive(Clone, Debug)]
pub struct LspFormatTarget;

// Re-export project stub types that still exist.
pub use project::{
    DisableAiSettings, Hover, InlayHint, InlayHintLabel, InlayHintLabelPart,
    InlayHintLabelPartTooltip, InlayHintTooltip, InlayId, InvalidationStrategy,
    LanguageServerToQuery, LocationLink, OpenLspBufferHandle, PrepareRenameResponse, TaskVariables,
};

#[derive(Clone, Debug)]
pub struct RefreshForServer;

#[derive(Clone, Copy, Debug)]
pub enum FormatTrigger { Manual }


// Re-export debugger session/breakpoint types from project
pub use project::debugger::breakpoint_store::{
    Breakpoint, BreakpointSessionState, BreakpointState, BreakpointWithPosition,
};
pub use project::debugger::session::{Session, SessionEvent};

// ---------------------------------------------------------------------------
// 任务相关类型 stub (task.rs 引用)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum RevealStrategy { #[default] InCenter, PreserveX }

#[derive(Clone, Debug)]
pub struct DebugScenario;

#[derive(Clone, Debug)]
pub struct ResolvedTask;

#[derive(Clone, Debug)]
pub struct RunnableTag;

#[derive(Clone, Debug)]
pub struct TaskContext;

#[derive(Clone, Copy, Debug)]
pub enum TaskSourceKind { Local }

#[derive(Clone, Debug)]
pub struct TaskTemplate;

// ---------------------------------------------------------------------------
// Navigation / remote IDs
// ---------------------------------------------------------------------------

// linked_editing_ranges 模块已删除，定义 stub 类型
#[derive(Default)]
pub struct LinkedEditingRanges;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Prev,
    Next,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CollaboratorId { Agent(u64), PeerId(u64) }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ViewId(pub u64);

#[derive(Clone, Debug)]
pub struct Collaborator {
    pub user_id: u64,
    pub peer_id: u64,
    pub replica_id: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParticipantIndex(pub u32);

// ---------------------------------------------------------------------------
// Project / workspace helpers
// ---------------------------------------------------------------------------

pub trait ProjectExt {
    fn is_remote(&self) -> bool;
    fn is_via_remote_server(&self) -> bool;
}

impl ProjectExt for Project {
    fn is_remote(&self) -> bool { false }
    fn is_via_remote_server(&self) -> bool { false }
}

pub trait ProjectLspStoreExt {
    fn lsp_store(&self) -> Entity<Project>;
}

impl ProjectLspStoreExt for Project {
    fn lsp_store(&self) -> Entity<Project> {
        unimplemented!("LspStore stub")
    }
}

pub trait ProjectCapabilityExt {
    fn capability(&self) -> language::Capability;
}

impl ProjectCapabilityExt for Project {
    fn capability(&self) -> language::Capability {
        language::Capability::ReadOnly
    }
}

pub trait ProjectBufferExt {
    fn buffer_for_id(&self, _buffer_id: BufferId, _cx: &App) -> Option<Entity<Buffer>>;
    fn create_buffer(&mut self, _capacity: usize, _cx: &mut App,
    ) -> Task<anyhow::Result<Entity<Buffer>>>;
}

impl ProjectBufferExt for Project {
    fn buffer_for_id(&self, _buffer_id: BufferId, _cx: &App) -> Option<Entity<Buffer>> { None }
    fn create_buffer(
        &mut self,
        _capacity: usize,
        _cx: &mut App,
    ) -> Task<anyhow::Result<Entity<Buffer>>> {
        Task::ready(Err(anyhow::anyhow!("create_buffer stub")))
    }
}

pub struct RevealInFileManager;

impl gpui::Action for RevealInFileManager {
    fn boxed_clone(&self) -> Box<dyn gpui::Action> {
        Box::new(*self)
    }

    fn partial_eq(&self, _other: &dyn gpui::Action) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "RevealInFileManager"
    }

    fn name_for_type() -> &'static str {
        "RevealInFileManager"
    }

    fn build(_value: serde_json::Value) -> anyhow::Result<Box<dyn gpui::Action>> {
        Ok(Box::new(Self))
    }
}

pub fn parse_zed_link(_link: &str) -> Option<Location> { None }

// ---------------------------------------------------------------------------
// Telemetry
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct TelemetrySpawnLocation;

pub fn send_telemetry(_event_name: &str, _cx: &App) {}

// ---------------------------------------------------------------------------
// Completions
// ---------------------------------------------------------------------------

pub trait CompletionProvider: Send + Sync {
    fn clone_box(&self) -> Box<dyn CompletionProvider>;
    fn selection_changed(
        &self,
        _selection: Option<&std::ops::Range<Anchor>>,
        _window: &mut Window,
        _cx: &mut gpui::Context<crate::Editor>,
    ) {}
}

impl Clone for Box<dyn CompletionProvider> {
    fn clone(&self) -> Self { self.clone_box() }
}

impl CompletionProvider for gpui::Entity<Project> {
    fn clone_box(&self) -> Box<dyn CompletionProvider> {
        Box::new(self.clone())
    }
}

pub type CompletionId = u64;

#[derive(Clone, Debug)]
pub struct Completion {
    pub new_text: SharedString,
    pub old_range: std::ops::Range<Anchor>,
    pub replace_range: std::ops::Range<Anchor>,
    pub label: SharedString,
    pub source: CompletionSource,
}

impl Completion {
    pub fn is_snippet(&self) -> bool { false }
    pub fn label(&self) -> Option<language::CodeLabel> { None }
    pub fn kind(&self) -> Option<lsp::CompletionItemKind> { None }
}

#[derive(Clone, Copy, Debug)]
pub enum CompletionIntent { Add, Replace, Compose, Complete, CompleteWithReplace, CompleteWithInsert }

#[derive(Clone, Debug)]
pub struct CompletionDisplayOptions;

#[derive(Clone, Debug)]
pub struct CompletionGroup;

#[derive(Clone, Debug)]
pub struct CompletionResponse;

#[derive(Clone, Copy, Debug)]
pub enum CompletionSource {
    Lsp {
        server_id: lsp::LanguageServerId,
        insert_range: Option<std::ops::Range<Anchor>>,
    }
}

pub fn split_words(_text: &str) -> Vec<String> { Vec::new() }

// ---------------------------------------------------------------------------
// Code actions
// ---------------------------------------------------------------------------

pub trait CodeActionProvider: Send + Sync {
    fn clone_box(&self) -> Box<dyn CodeActionProvider>;
}

impl Clone for Box<dyn CodeActionProvider> {
    fn clone(&self) -> Self { self.clone_box() }
}

impl CodeActionProvider for gpui::Entity<Project> {
    fn clone_box(&self) -> Box<dyn CodeActionProvider> {
        Box::new(self.clone())
    }
}

#[derive(Clone, Debug)]
pub struct AvailableCodeAction;

#[derive(Clone, Debug)]
pub enum CodeContextMenu {
    Completions(CompletionsMenu),
    CodeActions(CodeActionsMenu),
}

impl CodeContextMenu {
    pub fn select_first(
        &mut self,
        _completion_provider: Option<&dyn CompletionProvider>,
        _window: &mut Window,
        _cx: &mut gpui::Context<crate::Editor>,
    ) -> bool {
        false
    }

    pub fn visible(&self) -> bool {
        false
    }

    pub fn select_last(
        &mut self,
        _completion_provider: Option<&dyn CompletionProvider>,
        _window: &mut Window,
        _cx: &mut gpui::Context<crate::Editor>,
    ) -> bool {
        false
    }

    pub fn origin(&self) -> ContextMenuOrigin {
        ContextMenuOrigin::Cursor
    }

    pub fn render(
        &self,
        _style: &crate::EditorStyle,
        _max_height_in_lines: u32,
        _window: &mut Window,
        _cx: &mut gpui::Context<crate::Editor>,
    ) -> AnyElement {
        div().into_any()
    }

    pub fn render_aside(
        &mut self,
        _max_size: gpui::Size<gpui::Pixels>,
        _window: &mut Window,
        _cx: &mut gpui::Context<crate::Editor>,
    ) -> Option<AnyElement> {
        None
    }

    pub fn select_prev(
        &mut self,
        _completion_provider: Option<&dyn CompletionProvider>,
        _window: &mut Window,
        _cx: &mut gpui::Context<crate::Editor>,
    ) {
    }

    pub fn select_next(
        &mut self,
        _completion_provider: Option<&dyn CompletionProvider>,
        _window: &mut Window,
        _cx: &mut gpui::Context<crate::Editor>,
    ) {
    }

    pub fn focused(&self, _window: &Window, _cx: &gpui::Context<crate::Editor>) -> bool {
        false
    }

    pub fn primary_scroll_handle(&self) -> Option<ScrollHandle> {
        None
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum ContextMenuOrigin { #[default] Cursor, GutterIndicator(u32) }
#[derive(Clone, Debug, Default)]
pub struct CompletionsMenu;

impl CompletionsMenu {
    pub fn visible(&self) -> bool { false }

    pub fn new_snippet_choices(
        _id: CompletionId,
        _show_completion_documentation: bool,
        _choices: Vec<Snippet>,
        _position: Anchor,
        _range: std::ops::Range<Anchor>,
        _buffer: Entity<Buffer>,
        _scroll_handle: Option<ScrollHandle>,
        _snippet_sort_order: SnippetSortOrder,
    ) -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug, Default)]
pub struct CodeActionsMenu {
    pub deployed_from: ContextMenuOrigin,
    pub actions: Vec<AvailableCodeAction>,
}

impl CodeActionsMenu {
    pub fn visible(&self) -> bool { false }
}

// ---------------------------------------------------------------------------
// Signature help / hover / code lens
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct SignatureHelpState;

impl SignatureHelpState {
    pub fn popover_mut(&mut self,
    ) -> Option<&mut SignatureHelpPopover> { None }

    pub fn has_multiple_signatures(&self) -> bool { false }
}

#[derive(Clone, Debug, Default)]
pub struct SignatureHelpPopover {
    pub current_signature: usize,
    pub signatures: Vec<()>,
}

impl SignatureHelpPopover {
    pub fn render(
        &mut self,
        _max_size: gpui::Size<gpui::Pixels>,
        _window: &mut Window,
        _cx: &mut gpui::Context<crate::Editor>,
    ) -> AnyElement {
        div().into_any()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SignatureHelpHiddenBy { Escape }

#[derive(Clone, Debug, Default)]
pub struct HoverState;

impl HoverState {
    pub fn focused(&self, _window: &Window, _cx: &gpui::Context<crate::Editor>) -> bool { false }

    pub fn render(
        &self,
        _snapshot: &crate::DisplaySnapshot,
        _rows: std::ops::Range<crate::DisplayRow>,
        _max_size: gpui::Size<gpui::Pixels>,
        _text_layout_details: &crate::movement::TextLayoutDetails,
        _window: &mut Window,
        _cx: &mut gpui::Context<crate::Editor>,
    ) -> Option<(crate::DisplayPoint, Vec<AnyElement>)> {
        None
    }
}

#[derive(Clone, Debug, Default)]
pub struct HoveredLinkState {
    pub links: Vec<HoverLink>,
    pub symbol_range: Option<std::ops::Range<Anchor>>,
}

#[derive(Clone, Debug)]
pub struct FileTarget {
    pub resolved_path: project::ResolvedPath,
    pub project_path: project::ProjectPath,
}

impl FileTarget {
    pub fn navigate_item_to_position(&self, _item: gpui::AnyView, _cx: &mut gpui::AsyncApp) {}
}

#[derive(Clone, Debug)]
pub enum HoverLink {
    Url(String),
    InlayHighlight(LocationLink),
    Text(LocationLink),
    LspLocation(language::Location, lsp::LanguageServerId),
    File(FileTarget),
}

pub fn find_file(_path: &std::path::Path) -> Option<Entity<Buffer>> { None }

pub fn find_url(_text: &str) -> Option<String> { None }

pub fn find_url_from_range(_text: &str, _range: std::ops::Range<usize>) -> Option<String> { None }

pub fn exclude_link_to_position(
    _buffer: &Entity<Buffer>,
    _position: &text::Anchor,
    _location: &language::Location,
    _cx: &App,
) -> bool {
    false
}

pub fn hide_hover(_editor: &mut crate::Editor, _window: &mut Window, _cx: &mut gpui::Context<crate::Editor>) -> bool { false }

pub fn hover_at(
    _editor: &mut crate::Editor,
    _point: Option<crate::DisplayPoint>,
    _event_position: Option<gpui::Point<gpui::Pixels>>,
    _window: &mut Window,
    _cx: &mut gpui::Context<crate::Editor>,
) {
}

pub fn hover_markdown_style(_cx: &App) -> TextStyle { TextStyle::default() }

#[derive(Clone, Debug, Default)]
pub struct CodeLensState;

// ---------------------------------------------------------------------------
// Runnables
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct RunnableData;

impl RunnableData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn all_runnables(&self) -> impl Iterator<Item = &Arc<RunnableTasks>> {
        std::iter::empty()
    }
}

#[derive(Clone, Debug)]
pub struct RunnableTasks {
    pub offset: Anchor,
    pub column: u32,
    pub extra_variables: Vec<(String, String)>,
}

impl Default for RunnableTasks {
    fn default() -> Self {
        Self {
            offset: text::Anchor::min_for_buffer(text::BufferId::new(1).unwrap()),
            column: 0,
            extra_variables: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ResolvedTasks;

// ---------------------------------------------------------------------------
// Inlay / diagnostics
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct InlineValueCache {
    pub enabled: bool,
    pub inlays: Vec<InlayId>,
    pub refresh_task: Task<()>,
}

impl Default for InlineValueCache {
    fn default() -> Self {
        Self {
            enabled: false,
            inlays: Vec::new(),
            refresh_task: Task::ready(()),
        }
    }
}

impl InlineValueCache {
    pub fn new(enabled: bool) -> Self {
        Self { enabled, ..Default::default() }
    }
}

#[derive(Clone, Debug, Default)]
pub struct LspInlayHintData;

impl LspInlayHintData {
    pub fn new(_settings: InlayHintSettings) -> Self {
        Self::default()
    }

    pub fn remove_inlay_chunk_data(&mut self, _buffer_ids: &[language::BufferId]) {}
}
pub fn inlay_hint_settings(
    _anchor: text::Anchor,
    _snapshot: &language::BufferSnapshot,
    _cx: &App,
) -> InlayHintSettings {
    InlayHintSettings::default()
}

#[derive(Clone, Copy, Debug, Default)]
pub struct InlayHintSettings {
    pub enabled: bool,
    pub show_value_hints: bool,
    pub toggle_on_modifiers_press: Option<Modifiers>,
}

#[derive(Clone, Debug)]
pub enum InlayHintRefreshReason {
    RefreshRequested {
        server_id: lsp::LanguageServerId,
        request_id: Option<usize>,
    },
    NewLinesShown,
    ModifiersChanged(bool),
    SettingsChange(InlayHintSettings),
    BuffersRemoved(Vec<language::BufferId>),
    BufferEdited(language::BufferId),
}

#[derive(Clone, Debug)]
pub struct InlaySplice {
    pub to_remove: Vec<InlayId>,
    pub to_insert: Vec<(Anchor, InlayHint)>,
}

impl InlaySplice {
    pub fn is_empty(&self) -> bool {
        self.to_remove.is_empty() && self.to_insert.is_empty()
    }
}

#[derive(Clone, Debug, Default)]
pub struct ActiveDiagnostic;

impl ActiveDiagnostic {
    pub const None: Option<Self> = None;
}

#[derive(Clone, Debug)]
pub struct InlineDiagnostic {
    pub severity: language::DiagnosticSeverity,
    pub new_text: SharedString,
    pub message: SharedString,
    pub start: language::Point,
    pub group_id: usize,
    pub is_primary: bool,
}

pub trait DiagnosticRenderer: Send + Sync {
    fn clone_box(&self) -> Box<dyn DiagnosticRenderer>;
}

impl Clone for Box<dyn DiagnosticRenderer> {
    fn clone(&self) -> Self { self.clone_box() }
}

pub fn set_diagnostic_renderer(_renderer: Option<Box<dyn DiagnosticRenderer>>) {}

#[derive(Clone, Debug, Default)]
pub struct GlobalDiagnosticRenderer;

#[derive(Clone, Copy, Debug, Default)]
pub enum CursorPopoverType {
    #[default]
    CodeContextMenu,
    EditPrediction,
    Edit { display_mode: EditDisplayMode },
}

// ---------------------------------------------------------------------------
// Edit predictions / snippets / rename
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default)]
pub struct EditPredictionRequestTrigger;

impl EditPredictionRequestTrigger {
    pub const BufferEdit: Self = Self;
    pub const Other: Self = Self;
}

#[derive(Clone, Debug)]
pub enum EditPredictionDelegate { None }

pub type EditPredictionDelegateHandle = Arc<dyn Any>;

#[derive(Clone, Copy, Debug)]
pub enum EditPredictionDiscardReason { Accepted, Rejected }

#[derive(Clone, Copy, Debug)]
pub enum EditPredictionGranularity { Char, Word, Line }

#[derive(Clone, Copy, Debug)]
pub enum SuggestionDisplayType { Inline, Popup }

#[derive(Clone, Debug)]
pub struct RegisteredEditPredictionDelegate;

#[derive(Clone, Copy, Debug, Default)]
pub enum MenuEditPredictionsPolicy { #[default] Disabled, ByProvider }

#[derive(Clone, Copy, Debug, Default)]
pub enum EditDisplayMode { #[default] Inline, TabAccept }

#[derive(Clone, Debug, Default)]
pub struct EditPredictionPreview;

#[derive(Clone, Debug, Default)]
pub struct EditPredictionSettings;

impl EditPredictionSettings {
    pub const Disabled: Self = Self;
}

#[derive(Clone, Debug, Default)]
pub struct EditPredictionState {
    pub completion: CursorPopoverType,
}

pub fn make_suggestion_styles(_cx: &App) -> TextStyle { TextStyle::default() }

#[derive(Clone, Debug)]
pub struct Snippet { pub text: SharedString }

impl Snippet {
    pub fn parse(text: &str) -> anyhow::Result<Self> {
        Ok(Self { text: SharedString::from(text) })
    }
}

// ---------------------------------------------------------------------------
// Breakpoints (define missing variants/types not in project::stubs)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum BreakpointEditAction {
    Toggle,
    InvertState,
    EditLogMessage(SharedString),
    EditHitCondition(SharedString),
    EditCondition(SharedString),
}

#[derive(Clone, Debug)]
pub struct BreakpointStoreEvent;

#[derive(Default)]
pub struct BreakpointStore;

impl BreakpointStore {
    pub fn breakpoints(
        &self,
        _buffer: &Entity<Buffer>,
        _range: Option<std::ops::Range<Anchor>>,
        _cx: &App,
    ) -> Vec<(Anchor, Breakpoint, Option<BreakpointSessionState>)> {
        Vec::new()
    }

    pub fn active_position(&self) -> Option<&text::Point> {
        None
    }

    pub fn active_debug_line_pane_id(&self) -> Option<usize> {
        None
    }

    pub fn set_active_debug_line_pane_id(&mut self, _pane_id: Option<usize>) {}

    pub fn toggle_breakpoint(
        &mut self,
        _buffer: Entity<Buffer>,
        _breakpoint: BreakpointWithPosition,
        _cx: &mut gpui::Context<Self>,
    ) {
    }
}

// ---------------------------------------------------------------------------
// Vim / task variables
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default)]
pub struct VimModeSetting(pub bool);

impl VimModeSetting {
    pub fn try_get(_cx: &App) -> Option<Self> { None }
}

pub use project::VariableName;

// ---------------------------------------------------------------------------
// Collaboration hub stub
// ---------------------------------------------------------------------------

pub trait CollaborationHub: Send + Sync {
    fn user_names(&self, _cx: &App) -> std::collections::HashMap<u64, SharedString>;
}

// ---------------------------------------------------------------------------
// 常量 stub (来自已删除模块)
// ---------------------------------------------------------------------------

pub const HOVER_POPOVER_GAP: Pixels = px(4.);
pub const POPOVER_RIGHT_OFFSET: Pixels = px(4.);
pub const MIN_POPOVER_CHARACTER_WIDTH: f32 = 240.;
pub const MIN_POPOVER_LINE_HEIGHT: f32 = 24.;

pub const MENU_GAP: Pixels = px(8.);
pub const MENU_ASIDE_MIN_WIDTH: Pixels = px(240.);
pub const MENU_ASIDE_MAX_WIDTH: Pixels = px(480.);

/// Stub: refresh linked ranges (linked editing 模块已删除)
pub fn refresh_linked_ranges(
    _editor: &mut crate::Editor,
    _cx: &mut gpui::Context<crate::Editor>,
) {
}

// ---------------------------------------------------------------------------
#[derive(Clone)]
pub struct Inlay {
    pub id: project::InlayId,
    pub position: text::Anchor,
    text: text::Rope,
    pub content: InlayContent,
}

impl std::fmt::Debug for Inlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inlay").field("id", &self.id).field("position", &self.position).field("text", &self.text.to_string()).field("content", &self.content).finish()
    }
}

impl Inlay {
    pub fn text(&self) -> &text::Rope {
        &self.text
    }

    pub fn mock_hint(_id: usize, anchor: text::Anchor, hint_text: &str) -> Self {
        Self { id: project::InlayId::Hint(0), position: anchor, text: text::Rope::from(hint_text), content: InlayContent::Label(gpui::SharedString::from(hint_text)) }
    }

    pub fn edit_prediction(_id: usize, anchor: text::Anchor, pred_text: &str) -> Self {
        Self { id: project::InlayId::Hint(0), position: anchor, text: text::Rope::from(pred_text), content: InlayContent::Label(gpui::SharedString::from(pred_text)) }
    }

    pub fn debugger(_id: usize, anchor: text::Anchor, text: &str) -> Self {
        Self { id: project::InlayId::DebuggerValue(0), position: anchor, text: text::Rope::from(text), content: InlayContent::Label(gpui::SharedString::from(text)) }
    }
}

// InlayContent stub (inlays 模块已删除)
#[derive(Clone, Debug)]
pub enum InlayContent {
    Label(gpui::SharedString),
    Color(gpui::Hsla),
}

// InlayHighlight stub (hover_links 模块已删除)
#[derive(Clone, Debug)]
pub struct InlayHighlight {
    pub inlay: project::InlayId,
    pub inlay_position: text::Anchor,
    pub range: std::ops::Range<usize>,
}
