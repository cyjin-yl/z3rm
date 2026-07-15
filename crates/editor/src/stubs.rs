//! Stub types and helpers for APIs removed during dependency stripping.
//! Avoids duplicating definitions that remain in `project::stubs`.

use std::{any::Any, sync::Arc};

use gpui::{App, Entity, SharedString, Task, TextStyle, Window};
use language::{Buffer, Location};
use project::Project;
use text::{Anchor, BufferId, Point};

// Re-export project stub types that the editor still references.
pub use project::{
    BufferSemanticTokens, CacheInlayHints, CompletionDocumentation, DisableAiSettings,
    DocumentHighlight, Hover, InlayHint, InlayHintLabel, InlayHintLabelPart,
    InlayHintLabelPartTooltip, InlayHintTooltip, InlayId, InvalidationStrategy, LanguageServerToQuery,
    LocationLink, LspAction, LspFormatTarget, OpenLspBufferHandle, TaskVariables,
};

#[derive(Clone, Debug)]
pub struct RefreshForServer;

#[derive(Clone, Copy, Debug)]
pub enum FormatTrigger { Manual }

pub use project::debugger::{
    breakpoint_store::{
        Breakpoint, BreakpointSessionState, BreakpointWithPosition,
    },
    session::{Session, SessionEvent},
};

// ---------------------------------------------------------------------------
// Navigation / remote IDs
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Prev,
    Next,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CollaboratorId(pub u64);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ViewId(pub u64);

#[derive(Clone, Debug)]
pub struct Collaborator {
    pub user_id: u64,
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

pub enum RevealInFileManager {}

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
}

impl Clone for Box<dyn CompletionProvider> {
    fn clone(&self) -> Self { self.clone_box() }
}

pub type CompletionId = u64;

#[derive(Clone, Debug)]
pub struct Completion {
    pub new_text: SharedString,
    pub old_range: std::ops::Range<Anchor>,
}

#[derive(Clone, Copy, Debug)]
pub enum CompletionIntent { Add, Replace }

#[derive(Clone, Debug)]
pub struct CompletionDisplayOptions;

#[derive(Clone, Debug)]
pub struct CompletionGroup;

#[derive(Clone, Debug)]
pub struct CompletionResponse;

#[derive(Clone, Copy, Debug)]
pub enum CompletionSource { Lsp }

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

    pub fn select_last(
        &mut self,
        _completion_provider: Option<&dyn CompletionProvider>,
        _window: &mut Window,
        _cx: &mut gpui::Context<crate::Editor>,
    ) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ContextMenuOrigin { Cursor, GutterIndicator(u32) }

#[derive(Clone, Debug, Default)]
pub struct CompletionsMenu;

#[derive(Clone, Debug, Default)]
pub struct CodeActionsMenu;

// ---------------------------------------------------------------------------
// Signature help / hover / code lens
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct SignatureHelpState;

impl SignatureHelpState {
    pub fn popover_mut(&mut self,
    ) -> Option<&mut SignatureHelpPopover> { None }
}

#[derive(Clone, Debug, Default)]
pub struct SignatureHelpPopover;

#[derive(Clone, Copy, Debug)]
pub enum SignatureHelpHiddenBy { Escape }

#[derive(Clone, Debug, Default)]
pub struct HoverState;

impl HoverState {
    pub fn focused(&self) -> bool { false }
}

#[derive(Clone, Debug, Default)]
pub struct HoveredLinkState;

#[derive(Clone, Debug)]
pub enum HoverLink {
    Url(String),
    InlayHighlight(LocationLink),
}

pub fn find_file(_path: &std::path::Path) -> Option<Entity<Buffer>> { None }

pub fn find_url(_text: &str) -> Option<String> { None }

pub fn find_url_from_range(_text: &str, _range: std::ops::Range<usize>) -> Option<String> { None }

pub fn hide_hover(_editor: &mut crate::Editor, _window: &mut Window, _cx: &mut gpui::Context<crate::Editor>) {}

pub fn hover_at(
    _editor: &mut crate::Editor,
    _point: Option<crate::DisplayPoint>,
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

#[derive(Clone, Debug)]
pub struct RunnableTasks;

#[derive(Clone, Debug)]
pub struct ResolvedTasks;

// ---------------------------------------------------------------------------
// Inlay / diagnostics
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct InlineValueCache;

#[derive(Clone, Debug, Default)]
pub struct LspInlayHintData;

pub fn inlay_hint_settings(_language: Option<&language::LanguageName>, _cx: &App) -> InlayHintSettings {
    InlayHintSettings::default()
}

#[derive(Clone, Copy, Debug, Default)]
pub struct InlayHintSettings { pub enabled: bool }

#[derive(Clone, Copy, Debug)]
pub enum InlayHintRefreshReason { RefreshRequested }

#[derive(Clone, Debug)]
pub struct InlaySplice {
    pub to_remove: Vec<InlayId>,
    pub to_insert: Vec<(Anchor, InlayHint)>,
}

#[derive(Clone, Debug, Default)]
pub struct ActiveDiagnostic;

#[derive(Clone, Debug)]
pub struct InlineDiagnostic {
    pub severity: language::DiagnosticSeverity,
    pub new_text: SharedString,
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

// ---------------------------------------------------------------------------
// Edit predictions / snippets / rename
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default)]
pub struct EditPredictionRequestTrigger;

impl EditPredictionRequestTrigger {
    pub const BufferEdit: Self = Self;
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
pub enum MenuEditPredictionsPolicy { #[default] Disabled }

#[derive(Clone, Copy, Debug, Default)]
pub enum EditDisplayMode { #[default] Inline }

#[derive(Clone, Debug, Default)]
pub struct EditPredictionPreview;

#[derive(Clone, Debug, Default)]
pub struct EditPredictionSettings;

#[derive(Clone, Debug, Default)]
pub struct EditPredictionState;

pub fn make_suggestion_styles(_cx: &App) -> TextStyle { TextStyle::default() }

#[derive(Clone, Debug)]
pub struct Snippet;

#[derive(Clone, Debug)]
pub enum PrepareRenameResponse { Ready }

// ---------------------------------------------------------------------------
// Breakpoints (define missing variants/types not in project::stubs)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub enum BreakpointEditAction { Toggle, InvertState }

#[derive(Clone, Copy, Debug)]
pub enum BreakpointState { Enabled, Disabled }

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
}

// ---------------------------------------------------------------------------
// Vim / task variables
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default)]
pub struct VimModeSetting(pub bool);

impl VimModeSetting {
    pub fn try_get(_cx: &App) -> Option<Self> { None }
}

#[derive(Clone, Debug)]
pub enum VariableName { Custom(SharedString) }

// ---------------------------------------------------------------------------
// Collaboration hub stub
// ---------------------------------------------------------------------------

pub trait CollaborationHub: Send + Sync {
    fn user_names(&self, _cx: &App) -> std::collections::HashMap<u64, SharedString>;
}
