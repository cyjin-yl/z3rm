use core::num;

use gpui::App;
use language::CursorShape;
use project::project_settings::DiagnosticSeverity;
/// 兼容占位类型 - 设置重构后缺失的类型 (spec §16 Plan 16)
/// 这些类型已从 settings crate 移除, 在此定义以保持下游代码编译。

/// 代码透镜
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum CodeLens {
    #[default]
    On,
    Off,
}

impl CodeLens {
    /// 是否内联显示代码透镜
    pub fn inline(&self) -> bool {
        matches!(self, CodeLens::On)
    }
}

/// 补全详情对齐
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum CompletionDetailAlignment {
    #[default]
    Left,
    Right,
}

/// 补全菜单项种类
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum CompletionMenuItemKind {
    #[default]
    All,
    Symbols,
    Keywords,
    Types,
}

/// 当前行高亮
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum CurrentLineHighlight {
    #[default]
    Line,
    Gutter,
    All,
    None,
}

/// 延迟毫秒数
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct DelayMs(pub u64);

/// 差异视图样式
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum DiffViewStyle {
    #[default]
    Unified,
    Split,
}

/// 显示位置
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum DisplayIn {
    #[default]
    ActiveEditor,
    AllEditors,
}

/// 文档颜色渲染模式
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum DocumentColorsRenderMode {
    #[default]
    Inlay,
    Background,
    Border,
    Full,
    None,
}

/// 多缓冲区双击行为
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum DoubleClickInMultibuffer {
    #[default]
    Select,
    Open,
}

/// 跳转定义回退策略
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum GoToDefinitionFallback {
    #[default]
    Lens,
    Search,
    Never,
    None,
    FindAllReferences,
}

/// 跳转定义滚动策略
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum GoToDefinitionScrollStrategy {
    #[default]
    Center,
    Minimum,
    Top,
    Preserve,
}

/// 缩略图
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum MinimapThumb {
    #[default]
    Always,
    Hover,
}
/// 缩略图边框
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum MinimapThumbBorder {
    #[default]
    Full,
    LeftOnly,
    LeftOpen,
    RightOpen,
    None,
    Rounded,
    Square,
}

/// 多光标修饰键
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum MultiCursorModifier {
    #[default]
    Alt,
    CmdOrCtrl,
}

/// 滚动超过最后一行
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum ScrollBeyondLastLine {
    #[default]
    OnePage,
    Off,
    VerticalScrollMargin,
}

/// 滚动条诊断显示
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum ScrollbarDiagnostics {
    #[default]
    None,
    All,
    Error,
    Warning,
    Information,
}

/// 种子查询设置 (来自 workspace::settings_stubs)
pub use workspace::settings_stubs::SeedQuerySetting;

/// 显示缩略图
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum ShowMinimap {
    #[default]
    Auto,
    Always,
    Never,
}

/// 代码片段排序
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum SnippetSortOrder {
    #[default]
    Relevance,
    Alphabetical,
    Frequency,
}

/// 相对行号
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum RelativeLineNumbers {
    #[default]
    Disabled,
    Enabled,
    Wrapped,
}

impl RelativeLineNumbers {
    /// 是否启用了相对行号
    pub fn enabled(&self) -> bool {
        !matches!(self, RelativeLineNumbers::Disabled)
    }

    /// 是否使用环绕模式 (wrapped buffer rows)
    pub fn wrapped(&self) -> bool {
        matches!(self, RelativeLineNumbers::Wrapped)
    }
}

use settings::{RegisterSetting, Settings};
use ui::scrollbars::ShowScrollbar;

/// Imports from the VSCode settings at
/// https://code.visualstudio.com/docs/reference/default-settings
#[derive(Clone, RegisterSetting)]
pub struct EditorSettings {
    pub cursor_blink: bool,
    pub cursor_shape: Option<CursorShape>,
    pub current_line_highlight: CurrentLineHighlight,
    pub selection_highlight: bool,
    pub rounded_selection: bool,
    pub lsp_highlight_debounce: DelayMs,
    pub hover_popover_enabled: bool,
    pub hover_popover_delay: DelayMs,
    pub hover_popover_sticky: bool,
    pub hover_popover_hiding_delay: DelayMs,
    pub toolbar: Toolbar,
    pub scrollbar: Scrollbar,
    pub minimap: Minimap,
    pub gutter: Gutter,
    pub scroll_beyond_last_line: ScrollBeyondLastLine,
    pub vertical_scroll_margin: f64,
    pub autoscroll_on_clicks: bool,
    pub horizontal_scroll_margin: f32,
    pub scroll_sensitivity: f32,
    pub mouse_wheel_zoom: bool,
    pub fast_scroll_sensitivity: f32,
    pub sticky_scroll: StickyScroll,
    pub relative_line_numbers: RelativeLineNumbers,
    pub seed_search_query_from_cursor: SeedQuerySetting,
    pub use_smartcase_search: bool,
    pub multi_cursor_modifier: MultiCursorModifier,
    pub redact_private_values: bool,
    pub expand_excerpt_lines: u32,
    pub excerpt_context_lines: u32,
    pub middle_click_paste: bool,
    pub double_click_in_multibuffer: DoubleClickInMultibuffer,
    pub search_wrap: bool,
    pub search: SearchSettings,
    pub auto_signature_help: bool,
    pub show_signature_help_after_edits: bool,
    pub go_to_definition_fallback: GoToDefinitionFallback,
    pub go_to_definition_scroll_strategy: GoToDefinitionScrollStrategy,
    pub jupyter: Jupyter,
    pub snippet_sort_order: SnippetSortOrder,
    pub diagnostics_max_severity: Option<DiagnosticSeverity>,
    pub inline_code_actions: bool,
    pub drag_and_drop_selection: DragAndDropSelection,
    pub code_lens: CodeLens,
    pub lsp_document_colors: DocumentColorsRenderMode,
    pub lsp_document_links: bool,
    pub minimum_contrast_for_highlights: f32,
    pub completion_menu_scrollbar: ShowScrollbar,
    pub completion_detail_alignment: CompletionDetailAlignment,
    pub completion_menu_item_kind: CompletionMenuItemKind,
    pub diff_view_style: DiffViewStyle,
    pub minimum_split_diff_width: f32,
}
#[derive(Debug, Clone)]
pub struct Jupyter {
    /// Whether the Jupyter feature is enabled.
    ///
    /// Default: true
    pub enabled: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct StickyScroll {
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Toolbar {
    pub breadcrumbs: bool,
    pub quick_actions: bool,
    pub selections_menu: bool,
    pub agent_review: bool,
    pub code_actions: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Scrollbar {
    pub show: ShowScrollbar,
    pub git_diff: bool,
    pub selected_text: bool,
    pub selected_symbol: bool,
    pub search_results: bool,
    pub diagnostics: ScrollbarDiagnostics,
    pub cursors: bool,
    pub axes: ScrollbarAxes,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Minimap {
    pub show: ShowMinimap,
    pub display_in: DisplayIn,
    pub thumb: MinimapThumb,
    pub thumb_border: MinimapThumbBorder,
    pub current_line_highlight: Option<CurrentLineHighlight>,
    pub max_width_columns: num::NonZeroU32,
}

impl Minimap {
    pub fn minimap_enabled(&self) -> bool {
        self.show != ShowMinimap::Never
    }

    #[inline]
    pub fn on_active_editor(&self) -> bool {
        self.display_in == DisplayIn::ActiveEditor
    }

    pub fn with_show_override(self) -> Self {
        Self {
            show: ShowMinimap::Always,
            ..self
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Gutter {
    pub min_line_number_digits: usize,
    pub line_numbers: bool,
    pub runnables: bool,
    pub breakpoints: bool,
    pub bookmarks: bool,
    pub folds: bool,
}

/// Forcefully enable or disable the scrollbar for each axis
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ScrollbarAxes {
    /// When false, forcefully disables the horizontal scrollbar. Otherwise, obey other settings.
    ///
    /// Default: true
    pub horizontal: bool,

    /// When false, forcefully disables the vertical scrollbar. Otherwise, obey other settings.
    ///
    /// Default: true
    pub vertical: bool,
}

/// Whether to allow drag and drop text selection in buffer.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct DragAndDropSelection {
    /// When true, enables drag and drop text selection in buffer.
    ///
    /// Default: true
    pub enabled: bool,

    /// The delay in milliseconds that must elapse before drag and drop is allowed. Otherwise, a new text selection is created.
    ///
    /// Default: 300
    pub delay: DelayMs,
}

/// Default options for buffer and project search items.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct SearchSettings {
    /// Whether to show the project search button in the status bar.
    pub button: bool,
    /// Whether to only match on whole words.
    pub whole_word: bool,
    /// Whether to match case sensitively.
    pub case_sensitive: bool,
    /// Whether to include gitignored files in search results.
    pub include_ignored: bool,
    /// Whether to interpret the search query as a regular expression.
    pub regex: bool,
    /// Whether to center the cursor on each search match when navigating.
    pub center_on_match: bool,
}

impl EditorSettings {
    pub fn jupyter_enabled(cx: &App) -> bool {
        EditorSettings::get_global(cx).jupyter.enabled
    }
}

impl Settings for EditorSettings {
    fn from_settings(_content: &settings::SettingsContent) -> Self {
        // 设置重构后 SettingsContent 不再包含 editor 字段,
        // 返回默认值 (spec §16 Plan 16)
        Self {
            cursor_blink: true,
            cursor_shape: None,
            current_line_highlight: CurrentLineHighlight::default(),
            selection_highlight: true,
            rounded_selection: true,
            lsp_highlight_debounce: DelayMs(100),
            hover_popover_enabled: true,
            hover_popover_delay: DelayMs(50),
            hover_popover_sticky: true,
            hover_popover_hiding_delay: DelayMs(100),
            toolbar: Toolbar {
                breadcrumbs: true,
                quick_actions: true,
                selections_menu: true,
                agent_review: true,
                code_actions: true,
            },
            scrollbar: Scrollbar {
                show: ShowScrollbar::Auto,
                git_diff: true,
                selected_text: true,
                selected_symbol: true,
                search_results: true,
                diagnostics: ScrollbarDiagnostics::default(),
                cursors: true,
                axes: ScrollbarAxes {
                    horizontal: true,
                    vertical: true,
                },
            },
            minimap: Minimap {
                show: ShowMinimap::default(),
                display_in: DisplayIn::default(),
                thumb: MinimapThumb::default(),
                thumb_border: MinimapThumbBorder::default(),
                current_line_highlight: None,
                max_width_columns: num::NonZeroU32::new(128).unwrap(),
            },
            gutter: Gutter {
                min_line_number_digits: 2,
                line_numbers: true,
                runnables: true,
                breakpoints: true,
                bookmarks: true,
                folds: true,
            },
            scroll_beyond_last_line: ScrollBeyondLastLine::default(),
            vertical_scroll_margin: 0.0,
            autoscroll_on_clicks: true,
            horizontal_scroll_margin: 0.0,
            scroll_sensitivity: 1.0,
            mouse_wheel_zoom: false,
            fast_scroll_sensitivity: 2.0,
            sticky_scroll: StickyScroll { enabled: false },
            relative_line_numbers: RelativeLineNumbers::default(),
            seed_search_query_from_cursor: SeedQuerySetting::default(),
            use_smartcase_search: true,
            multi_cursor_modifier: MultiCursorModifier::default(),
            redact_private_values: false,
            expand_excerpt_lines: 3,
            excerpt_context_lines: 3,
            middle_click_paste: false,
            double_click_in_multibuffer: DoubleClickInMultibuffer::default(),
            search_wrap: true,
            search: SearchSettings {
                button: true,
                whole_word: false,
                case_sensitive: false,
                include_ignored: false,
                regex: false,
                center_on_match: true,
            },
            auto_signature_help: true,
            show_signature_help_after_edits: false,
            go_to_definition_fallback: GoToDefinitionFallback::default(),
            go_to_definition_scroll_strategy: GoToDefinitionScrollStrategy::default(),
            jupyter: Jupyter { enabled: true },
            snippet_sort_order: SnippetSortOrder::default(),
            diagnostics_max_severity: None,
            inline_code_actions: false,
            drag_and_drop_selection: DragAndDropSelection {
                enabled: true,
                delay: DelayMs(300),
            },
            code_lens: CodeLens::default(),
            lsp_document_colors: DocumentColorsRenderMode::default(),
            lsp_document_links: true,
            minimum_contrast_for_highlights: 0.15,
            completion_menu_scrollbar: ShowScrollbar::Auto,
            completion_detail_alignment: CompletionDetailAlignment::default(),
            completion_menu_item_kind: CompletionMenuItemKind::default(),
            diff_view_style: DiffViewStyle::default(),
            minimum_split_diff_width: 480.0,
        }
    }
}

#[derive(Default)]
pub struct EditorSettingsScrollbarProxy;

impl ui::scrollbars::ScrollbarVisibility for EditorSettingsScrollbarProxy {
    fn visibility(&self, cx: &App) -> ShowScrollbar {
        EditorSettings::get_global(cx).scrollbar.show
    }
}

pub fn ui_scrollbar_settings_from_raw(
    value: settings::ShowScrollbar,
) -> ui::scrollbars::ShowScrollbar {
    match value {
        settings::ShowScrollbar::Auto => ShowScrollbar::Auto,
        settings::ShowScrollbar::System => ShowScrollbar::System,
        settings::ShowScrollbar::Always => ShowScrollbar::Always,
        settings::ShowScrollbar::Never => ShowScrollbar::Never,
    }
}
