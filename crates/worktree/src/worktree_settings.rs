use std::path::Path;

use anyhow::Context as _;
use settings::{RegisterSetting, ScanSymlinksSetting, Settings};
use util::{
    ResultExt,
    paths::{PathMatcher, PathStyle},
    rel_path::RelPath,
};

#[derive(Clone, PartialEq, Eq, RegisterSetting)]
pub struct WorktreeSettings {
    /// Whether to prevent this project from being shared in public channels.
    pub prevent_sharing_in_public_channels: bool,
    pub file_scan_exclusions: PathMatcher,
    pub file_scan_inclusions: PathMatcher,
    /// This field contains all ancestors of the `file_scan_inclusions`. It's used to
    /// determine whether to terminate worktree scanning for a given dir.
    pub parent_dir_scan_inclusions: PathMatcher,
    pub scan_symlinks: ScanSymlinksSetting,
    pub private_files: PathMatcher,
    pub hidden_files: PathMatcher,
    pub read_only_files: PathMatcher,
}

impl WorktreeSettings {
    pub fn is_path_private(&self, path: &RelPath) -> bool {
        path.ancestors()
            .any(|ancestor| self.private_files.is_match(ancestor))
    }

    pub fn is_path_excluded(&self, path: &RelPath) -> bool {
        path.ancestors()
            .any(|ancestor| self.file_scan_exclusions.is_match(ancestor))
    }

    pub fn is_path_always_included(&self, path: &RelPath, is_dir: bool) -> bool {
        if is_dir {
            self.parent_dir_scan_inclusions.is_match(path)
        } else {
            self.file_scan_inclusions.is_match(path)
        }
    }

    pub fn is_path_hidden(&self, path: &RelPath) -> bool {
        path.ancestors()
            .any(|ancestor| self.hidden_files.is_match(ancestor))
    }

    pub fn is_path_read_only(&self, path: &RelPath) -> bool {
        self.read_only_files.is_match(path)
    }

    pub fn is_std_path_read_only(&self, path: &Path) -> bool {
        self.read_only_files.is_match_std_path(path)
    }
}

impl Settings for WorktreeSettings {
    fn from_settings(content: &settings::SettingsContent) -> Self {
        // 项目设置结构已重构, worktree 子模块已移除
        // 使用 project 级别的字段和默认值填充 WorktreeSettings (spec §16 Plan 16)
        let scan_symlinks = content.project.scan_symlinks.clone();
        let excluded_paths = content.project.excluded_paths.clone().unwrap_or_default();
        let file_scan_exclusions: Vec<String> = excluded_paths
            .iter()
            .map(|p| p.to_string_lossy().into())
            .collect();

        Self {
            prevent_sharing_in_public_channels: false,
            file_scan_exclusions: path_matchers(file_scan_exclusions, "file_scan_exclusions")
                .log_err()
                .unwrap_or_default(),
            parent_dir_scan_inclusions: PathMatcher::default(),
            file_scan_inclusions: PathMatcher::default(),
            private_files: PathMatcher::default(),
            hidden_files: PathMatcher::default(),
            read_only_files: PathMatcher::default(),
            scan_symlinks,
        }
    }
}

fn path_matchers(mut values: Vec<String>, context: &'static str) -> anyhow::Result<PathMatcher> {
    values.sort();
    PathMatcher::new(values, PathStyle::local())
        .with_context(|| format!("Failed to parse globs from {}", context))
}
