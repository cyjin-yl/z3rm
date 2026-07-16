// Disable command line from opening on release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod zed;

// Ensure the binary name stays in sync with APP_NAME so that the paths used
// at runtime (data dir, config dir, etc.) match what the binary is called.
const _: () = assert!(
    paths::APP_NAME_LOWERCASE
        .as_bytes()
        .eq_ignore_ascii_case(env!("CARGO_BIN_NAME").as_bytes()),
    "paths::APP_NAME_LOWERCASE must match the binary name. \
     Forks: update APP_NAME in crates/paths/src/paths.rs when renaming the binary.",
);

use anyhow::Context as _;
use assets::Assets;
use collections::HashMap;
use crashes::InitCrashHandler;
use fs::{Fs, RealFs};
use futures::StreamExt as _;
use gpui::{App, AppContext, Application, QuitMode, TaskExt, WindowOptions};
use gpui_platform;
use parking_lot::Mutex;
use release_channel::{AppCommitSha, AppVersion, ReleaseChannel};
use std::{
    env,
    io,
    path::Path,
    process,
    sync::{Arc, OnceLock},
    time::Instant,
};
use theme::ThemeRegistry;
use theme_settings::load_user_theme;
use util::ResultExt as _;
use crate::zed::{CrashHandler, init as zed_init, watch_settings_files};

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

static STARTUP_TIME: OnceLock<Instant> = OnceLock::new();

fn build_application() -> Application {
    let platform = gpui_platform::current_platform(false);
    if std::env::var("Z3RM_EXPERIMENTAL_A11Y").as_deref() == Ok("1") {
        Application::with_platform(platform)
    } else {
        Application::new_inaccessible(platform)
    }
}

fn files_not_created_on_launch(errors: HashMap<io::ErrorKind, Vec<&Path>>) {
    let message = "Zed failed to launch";
    let error_details = errors
        .into_iter()
        .flat_map(|(kind, paths)| {
            #[allow(unused_mut)] // for non-unix platforms
            let mut error_kind_details = match paths.len() {
                0 => return None,
                1 => format!(
                    "{kind} when creating directory {:?}",
                    paths.first().expect("match arm checks for a single entry")
                ),
                _many => format!("{kind} when creating directories {paths:?}"),
            };

            #[cfg(unix)]
            {
                if kind == io::ErrorKind::PermissionDenied {
                    error_kind_details.push_str("\n\nConsider using chown and chmod tools for altering the directories permissions if your user has corresponding rights.\
                        \nFor example, `sudo chown $(whoami):staff ~/.config` and `chmod +uwrx ~/.config`");
                }
            }

            Some(error_kind_details)
        })
        .collect::<Vec<_>>().join("\n\n");

    eprintln!("{message}: {error_details}");
    build_application()
        .with_quit_mode(QuitMode::Explicit)
        .run(move |cx| {
            if let Ok(window) = cx.open_window(gpui::WindowOptions::default(), |_, cx| {
                cx.new(|_| gpui::Empty)
            }) {
                window
                    .update(cx, |_, window, cx| {
                        let response = window.prompt(
                            gpui::PromptLevel::Critical,
                            message,
                            Some(&error_details),
                            &["Exit"],
                            cx,
                        );

                        cx.spawn_in(window, async move |_, cx| {
                            response.await?;
                            cx.update(|_, cx| cx.quit())
                        })
                        .detach_and_log_err(cx);
                    })
                    .log_err();
            } else {
                fail_to_open_window(anyhow::anyhow!("{message}: {error_details}"), cx)
            }
        })
}

fn fail_to_open_window(e: anyhow::Error, _cx: &mut App) {
    eprintln!(
        "Zed failed to open a window: {e:?}. See https://zed.dev/docs/linux for troubleshooting steps."
    );
    process::exit(1);
}

fn main() {
    STARTUP_TIME.get_or_init(|| Instant::now());

    // If this process was re-executed as a Linux sandbox helper, run that mode
    // without returning.
    sandbox::run_sandbox_launcher_if_invoked();

    #[cfg(unix)]
    util::prevent_root_execution();

    let file_errors = init_paths();
    if !file_errors.is_empty() {
        files_not_created_on_launch(file_errors);
        return;
    }

    ztracing::init();

    let version = option_env!("Z3RM_BUILD_ID");
    let app_commit_sha =
        option_env!("Z3RM_COMMIT_SHA").map(|commit_sha| AppCommitSha::new(commit_sha.to_string()));
    let app_version = AppVersion::load(env!("CARGO_PKG_VERSION"), version, app_commit_sha.clone());

    tracing::info!(
        "========== starting zed version {}, sha {} ==========",
        app_version,
        app_commit_sha
            .as_ref()
            .map(|sha| sha.short())
            .as_deref()
            .unwrap_or("unknown"),
    );

    let app = build_application().with_assets(Assets);
    let background_executor = app.background_executor();

    let should_install_crash_handler = matches!(
        env::var("Z3RM_GENERATE_MINIDUMPS").as_deref(),
        Ok("true" | "1")
    ) || *release_channel::RELEASE_CHANNEL != ReleaseChannel::Dev;

    let crash_handler = if should_install_crash_handler {
        Some(background_executor.spawn(crashes::init(
            InitCrashHandler {
                session_id: String::new(),
                zed_version: format!(
                    "{}.{}.{}",
                    app_version.major, app_version.minor, app_version.patch
                ),
                binary: "zed".to_string(),
                release_channel: release_channel::RELEASE_CHANNEL_NAME.clone(),
                commit_sha: app_commit_sha
                    .as_ref()
                    .map(|sha| sha.full())
                    .unwrap_or_else(|| "no sha".to_owned()),
            },
            {
                let background_executor = background_executor.clone();
                move |task| {
                    background_executor.spawn(task).detach();
                }
            },
            |pid| paths::temp_dir().join(format!("zed-crash-handler-{pid}")),
            {
                let background_executor = background_executor.clone();
                move |duration| background_executor.timer(duration)
            },
        )))
    } else {
        crashes::force_backtrace();
        None
    };

    let fs = Arc::new(RealFs::new(None, background_executor.clone()));

    app.run(move |cx| {
        cx.set_global(db::AppDatabase::new());
        release_channel::init(app_version.clone(), cx);
        settings::init(cx);
        theme_settings::init(theme::LoadThemes::All(Box::new(Assets)), cx);
        zed_init(cx);
        watch_settings_files(fs.clone(), cx);

        load_embedded_fonts(cx);
        load_user_themes_in_background(fs.clone(), cx);
        watch_themes(fs.clone(), cx);

        if let Some(crash_handler) = crash_handler {
            cx.spawn(async move |cx| {
                let client = crash_handler.await;
                cx.update(|cx| cx.set_global(CrashHandler(client)));
            })
            .detach();
        }

        if let Err(e) = cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| gpui::Empty)) {
            fail_to_open_window(e, cx);
        }
        cx.activate(true);
    });
}

fn init_paths() -> HashMap<io::ErrorKind, Vec<&'static Path>> {
    [
        paths::config_dir(),
        paths::extensions_dir(),
        paths::languages_dir(),
        paths::debug_adapters_dir(),
        paths::database_dir(),
        paths::logs_dir(),
        paths::temp_dir(),
        paths::hang_traces_dir(),
    ]
    .into_iter()
    .fold(HashMap::default(), |mut errors, path| {
        if let Err(e) = std::fs::create_dir_all(path) {
            errors.entry(e.kind()).or_insert_with(Vec::new).push(path);
        }
        errors
    })
}

fn load_embedded_fonts(cx: &App) {
    let asset_source = cx.asset_source();
    let font_paths = asset_source.list("fonts").unwrap();
    let embedded_fonts = Mutex::new(Vec::new());
    let executor = cx.background_executor();

    cx.foreground_executor().block_on(executor.scoped(|scope| {
        for font_path in &font_paths {
            if !font_path.ends_with(".ttf") {
                continue;
            }

            scope.spawn(async {
                let font_bytes = asset_source.load(font_path).unwrap().unwrap();
                embedded_fonts.lock().push(font_bytes);
            });
        }
    }));

    cx.text_system()
        .add_fonts(embedded_fonts.into_inner())
        .unwrap();
}

/// Spawns a background task to load the user themes from the themes directory.
fn load_user_themes_in_background(fs: Arc<dyn Fs>, cx: &mut App) {
    cx.spawn({
        let fs = fs.clone();
        async move |cx| {
            let theme_registry = cx.update(|cx| ThemeRegistry::global(cx));
            let themes_dir = paths::themes_dir().as_ref();
            match fs
                .metadata(themes_dir)
                .await
                .ok()
                .flatten()
                .map(|m| m.is_dir)
            {
                Some(is_dir) => {
                    anyhow::ensure!(is_dir, "Themes dir path {themes_dir:?} is not a directory")
                }
                None => {
                    fs.create_dir(themes_dir).await.with_context(|| {
                        format!("Failed to create themes dir at path {themes_dir:?}")
                    })?;
                }
            }

            let mut theme_paths = fs
                .read_dir(themes_dir)
                .await
                .with_context(|| format!("reading themes from {themes_dir:?}"))?;

            while let Some(theme_path) = theme_paths.next().await {
                let Some(theme_path) = theme_path.log_err() else {
                    continue;
                };
                let Some(bytes) = fs.load_bytes(&theme_path).await.log_err() else {
                    continue;
                };

                load_user_theme(&theme_registry, &bytes).log_err();
            }

            cx.update(theme_settings::reload_theme);
            anyhow::Ok(())
        }
    })
    .detach_and_log_err(cx);
}

/// Spawns a background task to watch the themes directory for changes.
fn watch_themes(fs: Arc<dyn Fs>, cx: &mut App) {
    use std::time::Duration;
    cx.spawn(async move |cx| {
        let (mut events, _) = fs
            .watch(paths::themes_dir(), Duration::from_millis(100))
            .await;

        while let Some(paths) = events.next().await {
            for event in paths {
                if fs
                    .metadata(&event.path)
                    .await
                    .ok()
                    .flatten()
                    .is_some_and(|m| !m.is_dir)
                {
                    let theme_registry = cx.update(|cx| ThemeRegistry::global(cx));
                    if let Some(bytes) = fs.load_bytes(&event.path).await.log_err()
                        && load_user_theme(&theme_registry, &bytes).log_err().is_some()
                    {
                        cx.update(theme_settings::reload_theme);
                    }
                }
            }
        }
    })
    .detach()
}
