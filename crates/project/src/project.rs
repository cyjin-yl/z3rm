pub mod buffer_store;
pub mod debounced_delay;
pub mod environment;
pub mod git_store;
pub mod manifest_tree;
pub mod project_settings;
pub mod search;
pub mod search_history;
pub mod toolchain_store;
pub mod trusted_worktrees;
pub mod worktree_store;
pub mod stubs;
pub use stubs::*;

use buffer_diff::BufferDiff;
pub use environment::ProjectEnvironmentEvent;
use git::repository::get_git_committer;
use git_store::{Repository, RepositoryId};

use crate::{
    git_store::GitStore,
    project_settings::{ProjectSettings, SettingsObserver, SettingsObserverEvent},
    trusted_worktrees::{PathTrust, RemoteHostLocation, TrustedWorktrees},
    worktree_store::WorktreeIdCounter,
};
pub use git_store::{
    ConflictRegion, ConflictSet, ConflictSetSnapshot, ConflictSetUpdate,
    git_traversal::{ChildEntriesGitIter, GitEntry, GitEntryRef, GitTraversal},
    linked_worktree_short_name, repo_identity_path, worktrees_directory_for_repo,
};
pub use manifest_tree::ManifestTree;
pub use worktree_store::WorktreePaths;

use anyhow::{Context as _, Result, anyhow};
use buffer_store::{BufferStore, BufferStoreEvent};
use clock::ReplicaId;

use collections::{BTreeSet, HashMap, HashSet, IndexSet};
use debounced_delay::DebouncedDelay;

pub use environment::ProjectEnvironment;

use ::git::{blame::Blame, status::FileStatus};
use gpui::{
    App, AppContext, AsyncApp, BorrowAppContext, Context, Entity, EventEmitter, Hsla, SharedString,
    Task, TaskExt, WeakEntity, Window,
};
use language::{Buffer, File as LanguageFile, LanguageRegistry};
use parking_lot::Mutex;
use rpc::{
    AnyProtoClient, ErrorCode,
    proto::{self, LanguageServerPromptResponse, REMOTE_SERVER_PROJECT_ID},
};
use search::{SearchInputKind, SearchQuery, SearchResult};
use search_history::SearchHistory;
use settings::{InvalidSettingsError, RegisterSetting, Settings, SettingsLocation, SettingsStore};
use std::{
    borrow::Cow,
    collections::BTreeMap,
    ffi::OsString,
    future::Future,
    ops::{Not as _, Range},
    path::{Path, PathBuf},
    pin::pin,
    str::{self, FromStr},
    sync::Arc,
    time::Duration,
};
use text::{Anchor, BufferId, Point, Rope};
use toolchain_store::EmptyToolchainStore;
use util::{
    ResultExt as _, maybe,
    path_list::PathList,
    paths::{PathStyle, SanitizedPath, is_absolute},
    rel_path::RelPath,
};
use worktree::{CreatedEntry, Snapshot, Traversal};
pub use worktree::{
    Entry, EntryKind, FS_WATCH_LATENCY, File, LocalWorktree, PathChange, ProjectEntryId,
    UpdatedEntriesSet, UpdatedGitRepositoriesSet, Worktree, WorktreeId, WorktreeSettings,
    discover_root_repo_common_dir,
};
use worktree_store::{WorktreeStore, WorktreeStoreEvent};

pub use buffer_store::ProjectTransaction;
pub use fs::*;
pub use language::Location;
pub use toolchain_store::{ToolchainStore, Toolchains};
pub use util::shell::Shell;
const MAX_PROJECT_SEARCH_HISTORY_SIZE: usize = 500;

#[derive(Clone, Copy, Debug)]
pub struct LocalProjectFlags {
    pub init_worktree_trust: bool,
    pub watch_global_configs: bool,
}

impl Default for LocalProjectFlags {
    fn default() -> Self {
        Self {
            init_worktree_trust: true,
            watch_global_configs: true,
        }
    }
}

pub trait ProjectItem: 'static {
    fn try_open(
        project: &Entity<Project>,
        path: &ProjectPath,
        cx: &mut App,
    ) -> Option<Task<Result<Entity<Self>>>>
    where
        Self: Sized;
    fn entry_id(&self, cx: &App) -> Option<ProjectEntryId>;
    fn project_path(&self, cx: &App) -> Option<ProjectPath>;
    fn is_dirty(&self) -> bool;
}

/// `Project` manages worktree and git integration.
pub struct Project {
    active_entry: Option<ProjectEntryId>,
    languages: Arc<LanguageRegistry>,
    fs: Arc<dyn Fs>,
    git_store: Entity<GitStore>,
    worktree_store: Entity<WorktreeStore>,
    buffer_store: Entity<BufferStore>,
    _subscriptions: Vec<gpui::Subscription>,
    buffers_needing_diff: HashSet<WeakEntity<Buffer>>,
    git_diff_debouncer: DebouncedDelay<Self>,
    search_history: SearchHistory,
    search_included_history: SearchHistory,
    search_excluded_history: SearchHistory,
    environment: Entity<ProjectEnvironment>,
    settings_observer: Entity<SettingsObserver>,
    toolchain_store: Option<Entity<ToolchainStore>>,
    last_worktree_paths: WorktreePaths,
}

pub enum Event {
    Closed,
    WorktreeAdded(WorktreeId),
    WorktreeRemoved(WorktreeId),
    WorktreeOrderChanged,
    ActiveEntryChanged(Option<ProjectEntryId>),
    DeletedEntry(WorktreeId, ProjectEntryId),
    WorktreePathsChanged { old_worktree_paths: WorktreePaths },
    WorktreeUpdatedEntries(WorktreeId, Vec<(ProjectEntryId, ProjectEntryId, PathChange)>),
    Toast {
        notification_id: String,
        message: String,
        link: Option<String>,
    },
    /// Stub variants for deleted diagnostic/remote features (spec §8.2 M2)
    DiskBasedDiagnosticsStarted,
    DiskBasedDiagnosticsFinished,
    DiagnosticsUpdated,
    LanguageServerRemoved,
    DisconnectedFromRemote,
    DisconnectedFromHost,
    LanguageNotFound(Entity<language::Buffer>),
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct ProjectPath {
    pub worktree_id: WorktreeId,
    pub path: Arc<RelPath>,
}

impl ProjectPath {
    pub fn from_file(value: &dyn language::File, cx: &App) -> Self {
        ProjectPath {
            worktree_id: value.worktree_id(cx),
            path: value.path().clone(),
        }
    }

    pub fn from_proto(p: proto::ProjectPath) -> Option<Self> {
        Some(Self {
            worktree_id: WorktreeId::from_proto(p.worktree_id),
            path: RelPath::from_proto(&p.path).log_err()?,
        })
    }

    pub fn to_proto(&self) -> proto::ProjectPath {
        proto::ProjectPath {
            worktree_id: self.worktree_id.to_proto(),
            path: self.path.as_ref().to_proto(),
        }
    }

    pub fn root_path(worktree_id: WorktreeId) -> Self {
        Self {
            worktree_id,
            path: RelPath::empty_arc(),
        }
    }

    pub fn starts_with(&self, other: &ProjectPath) -> bool {
        self.worktree_id == other.worktree_id && self.path.starts_with(&other.path)
    }
}

impl Project {
    pub fn local(
        languages: Arc<LanguageRegistry>,
        fs: Arc<dyn Fs>,
        env: Option<HashMap<String, String>>,
        worktrees: Vec<PathBuf>,
        cx: &mut App,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let worktree_store =
                cx.new(|cx| WorktreeStore::local(false, fs.clone(), WorktreeIdCounter::get(cx)));

            let buffer_store = cx.new(|cx| BufferStore::local(worktree_store.clone(), cx));
            let project_settings = cx.new(|cx| {
                SettingsObserver::new_local(fs.clone(), worktree_store.clone(), true, cx)
            });

            let environment = cx.new(|cx| {
                ProjectEnvironment::new(env, worktree_store.downgrade(), None, false, cx)
            });

            let git_store = cx.new(|cx| {
                GitStore::local(
                    &worktree_store,
                    buffer_store.clone(),
                    environment.clone(),
                    fs.clone(),
                    cx,
                )
            });

            let mut project = Self {
                active_entry: None,
                languages,
                fs: fs.clone(),
                git_store,
                worktree_store,
                buffer_store,
                _subscriptions: Vec::new(),
                buffers_needing_diff: HashSet::default(),
                git_diff_debouncer: DebouncedDelay::new(),
                search_history: SearchHistory::new(
                    Some(MAX_PROJECT_SEARCH_HISTORY_SIZE),
                    search_history::QueryInsertionBehavior::default(),
                ),
                search_included_history: SearchHistory::new(
                    Some(MAX_PROJECT_SEARCH_HISTORY_SIZE),
                    search_history::QueryInsertionBehavior::default(),
                ),
                search_excluded_history: SearchHistory::new(
                    Some(MAX_PROJECT_SEARCH_HISTORY_SIZE),
                    search_history::QueryInsertionBehavior::default(),
                ),
                environment,
                settings_observer: project_settings,
                toolchain_store: None,
                last_worktree_paths: WorktreePaths::default(),
            };

            for worktree_path in worktrees {
                project.add_local_worktree(worktree_path, true, cx);
            }

            project
        })
    }

    pub fn fs(&self) -> &Arc<dyn Fs> {
        &self.fs
    }

    pub fn languages(&self) -> &Arc<LanguageRegistry> {
        &self.languages
    }

    pub fn worktree_store(&self) -> &Entity<WorktreeStore> {
        &self.worktree_store
    }

    pub fn git_store(&self) -> &Entity<GitStore> {
        &self.git_store
    }

    pub fn buffer_store(&self) -> &Entity<BufferStore> {
        &self.buffer_store
    }

    pub fn environment(&self) -> &Entity<ProjectEnvironment> {
        &self.environment
    }

    pub fn settings_observer(&self) -> &Entity<SettingsObserver> {
        &self.settings_observer
    }

    pub fn active_entry(&self) -> Option<ProjectEntryId> {
        self.active_entry
    }

    pub fn set_active_entry(
        &mut self,
        active_entry: Option<ProjectEntryId>,
        cx: &mut Context<Self>,
    ) {
        self.active_entry = active_entry;
        cx.emit(Event::ActiveEntryChanged(active_entry));
    }

    pub fn worktree_for_entry(
        &self,
        entry_id: ProjectEntryId,
        cx: &App,
    ) -> Option<Entity<Worktree>> {
        self.worktree_store
            .read(cx)
            .worktree_for_entry(entry_id, cx)
    }

    pub fn worktree_for_id(&self, id: WorktreeId, cx: &App) -> Option<Entity<Worktree>> {
        self.worktree_store.read(cx).worktree_for_id(id, cx)
    }

    pub fn worktrees(&self, cx: &App) -> impl Iterator<Item = Entity<Worktree>> {
        self.worktree_store.read(cx).worktrees()
    }

    pub fn entry_for_path<'a>(&'a self, path: &ProjectPath, cx: &'a App) -> Option<&'a Entry> {
        self.worktree_store.read(cx).entry_for_path(path, cx)
    }

    pub fn entry_for_id<'a>(&'a self, entry_id: ProjectEntryId, cx: &'a App) -> Option<&'a Entry> {
        self.worktree_store.read(cx).entry_for_id(entry_id, cx)
    }

    pub fn project_path_for_absolute_path(&self, abs_path: &Path, cx: &App) -> Option<ProjectPath> {
        self.worktree_store
            .read(cx)
            .project_path_for_absolute_path(abs_path, cx)
    }

    pub fn absolute_path(&self, path: &ProjectPath, cx: &App) -> Option<PathBuf> {
        self.worktree_store.read(cx).absolutize(path, cx)
    }

    pub fn default_visible_worktree_paths(
        worktree_store: &WorktreeStore,
        cx: &App,
    ) -> Vec<Arc<Path>> {
        worktree_store
            .worktrees()
            .filter(|worktree| worktree.read(cx).is_visible())
            .map(|worktree| worktree.read(cx).abs_path())
            .collect()
    }

    pub fn worktree_paths(&self, cx: &App) -> WorktreePaths {
        self.worktree_store.read(cx).paths(cx)
    }

    pub fn path_style(&self, cx: &App) -> PathStyle {
        self.worktree_store
            .read(cx)
            .worktrees()
            .next()
            .map(|worktree| worktree.read(cx).path_style())
            .unwrap_or(PathStyle::Posix)
    }

    pub fn add_local_worktree(
        &mut self,
        abs_path: impl Into<PathBuf> + Send + 'static,
        visible: bool,
        cx: &mut Context<Self>,
    ) -> Task<Result<Entity<Worktree>>> {
        let worktree_store = self.worktree_store.clone();
        cx.spawn(async move |this, cx| {
            let task = worktree_store.update(cx, |store, cx| {
                store.create_worktree(abs_path.into(), visible, cx)
            });
            let worktree = task.await?;
            let worktree_id = worktree.read_with(cx, |tree, _| tree.id());
            this.update(cx, |project, cx| {
                project.last_worktree_paths = project.worktree_store.read(cx).paths(cx);
                cx.emit(Event::WorktreeAdded(worktree_id));
            })?;
            Ok(worktree)
        })
    }

    pub fn open_buffer(
        &mut self,
        path: ProjectPath,
        cx: &mut Context<Self>,
    ) -> Task<Result<Entity<Buffer>>> {
        self.buffer_store
            .update(cx, |store, cx| store.open_buffer(path, cx))
    }

    pub fn open_uncommitted_diff(
        &mut self,
        buffer: Entity<Buffer>,
        cx: &mut Context<Self>,
    ) -> Task<Result<Entity<buffer_diff::BufferDiff>>> {
        self.git_store
            .update(cx, |store, cx| store.open_uncommitted_diff(buffer, cx))
    }

    pub fn open_unstaged_diff(
        &mut self,
        buffer: Entity<Buffer>,
        cx: &mut Context<Self>,
    ) -> Task<Result<Entity<buffer_diff::BufferDiff>>> {
        self.git_store
            .update(cx, |store, cx| store.open_unstaged_diff(buffer, cx))
    }

    pub fn open_staged_diff(
        &mut self,
        buffer: Entity<Buffer>,
        cx: &mut Context<Self>,
    ) -> Task<Result<(Entity<buffer_diff::BufferDiff>, Entity<Buffer>)>> {
        self.git_store
            .update(cx, |store, cx| store.open_staged_diff(buffer, cx))
    }

    // =====================================================================
    // 以下为 stub 方法 — 对应已删除的远程协作 / 终端 / 符号跳转功能模块
    // =====================================================================

    /// Stub: is_shared (collaboration 模块已删除)
    pub fn is_shared(&self) -> bool {
        false
    }

    /// Stub: is_via_remote_server (remote 模块已删除)
    pub fn is_via_remote_server(&self) -> bool {
        false
    }

    /// Stub: project_path_git_status (git status 模块已简化)
    pub fn project_path_git_status(
        &self,
        _path: &ProjectPath,
        _cx: &App,
    ) -> Option<git::status::FileStatus> {
        None
    }

    /// Stub: set_language_for_buffer (language assignment 功能已删除)
    pub fn set_language_for_buffer(
        &mut self,
        _buffer: &Entity<Buffer>,
        _language: Arc<language::Language>,
        _cx: &mut Context<Self>,
    ) {
    }

    /// Stub: open_buffer_for_symbol (symbol search 功能已删除)
    pub fn open_buffer_for_symbol(
        &self,
        _symbol: &Symbol,
        _cx: &mut Context<Self>,
    ) -> Task<anyhow::Result<Entity<language::Buffer>>> {
        Task::ready(Err(anyhow::anyhow!("stub: symbol search disabled")))
    }
}

impl ProjectItem for Buffer {
    fn try_open(
        project: &Entity<Project>,
        path: &ProjectPath,
        cx: &mut App,
    ) -> Option<Task<Result<Entity<Self>>>> {
        Some(project.update(cx, |project, cx| project.open_buffer(path.clone(), cx)))
    }

    fn entry_id(&self, _cx: &App) -> Option<ProjectEntryId> {
        None
    }

    fn project_path(&self, cx: &App) -> Option<ProjectPath> {
        let file = self.file()?;
        Some(ProjectPath {
            worktree_id: file.worktree_id(cx),
            path: file.path().clone(),
        })
    }

    fn is_dirty(&self) -> bool {
        self.is_dirty()
    }
}

impl From<(WorktreeId, Arc<RelPath>)> for ProjectPath {
    fn from((worktree_id, path): (WorktreeId, Arc<RelPath>)) -> Self {
        Self { worktree_id, path }
    }
}

impl EventEmitter<Event> for Project {}

impl<'a> From<&'a ProjectPath> for SettingsLocation<'a> {
    fn from(val: &'a ProjectPath) -> Self {
        SettingsLocation {
            worktree_id: val.worktree_id,
            path: val.path.as_ref(),
        }
    }
}
