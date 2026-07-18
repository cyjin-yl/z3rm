use std::sync::Arc;
use crashes;
use fs::Fs;
use gpui::{actions, App, Global, UpdateGlobal as _};
use settings::SettingsStore;
use crate::log_viewer;

#[allow(dead_code)]
pub struct CrashHandler(pub Arc<crashes::Client>);

impl Global for CrashHandler {}

actions!(
    zed,
    [
        /// Quits the application.
        Quit,
    ]
);

pub fn init(cx: &mut App) {
    cx.on_action(quit);
    log_viewer::init(cx);
}

fn quit(_: &Quit, cx: &mut App) {
    cx.quit();
}

pub fn watch_settings_files(fs: Arc<dyn Fs>, cx: &mut App) {
    SettingsStore::update_global(cx, |store, cx| {
        store.watch_settings_files(fs, cx, |_, _, _| {});
    });
}
