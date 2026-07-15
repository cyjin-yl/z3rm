use std::sync::{Arc, LazyLock};

use anyhow::{Context as _, Result};
use collections::HashMap;
use gpui::{App, AsyncApp, BorrowAppContext as _, Entity, Task};
use language::{LanguageRegistry, language_settings::AllLanguageSettings};
use parking_lot::RwLock;
use project::Project;
use settings::Settings as _;
use util::schemars::{AllowTrailingCommas, DefaultDenyUnknownFields};

const SCHEMA_URI_PREFIX: &str = "z3rm://schemas/";

const TSCONFIG_SCHEMA: &str = include_str!("schemas/tsconfig.json");
const PACKAGE_JSON_SCHEMA: &str = include_str!("schemas/package.json");

static JSONC_SCHEMA: LazyLock<String> = LazyLock::new(|| {
    serde_json::to_string(&generate_jsonc_schema()).expect("JSONC schema should serialize")
});

#[cfg(debug_assertions)]
static INSPECTOR_STYLE_SCHEMA: LazyLock<String> = LazyLock::new(|| {
    serde_json::to_string(&generate_inspector_style_schema())
        .expect("Inspector style schema should serialize")
});

static KEYMAP_SCHEMA: LazyLock<String> = LazyLock::new(|| {
    serde_json::to_string(&settings::KeymapFile::generate_json_schema_from_inventory())
        .expect("Keymap schema should serialize")
});

static ACTION_SCHEMA_CACHE: LazyLock<RwLock<HashMap<String, String>>> =
    LazyLock::new(|| RwLock::new(HashMap::default()));

// Runtime cache for dynamic schemas that depend on runtime state:
// - "settings": depends on installed fonts, themes, languages, LSP adapters (extensions can add these)
// Cache is invalidated via notify_schema_changed() when extensions change.
static DYNAMIC_SCHEMA_CACHE: LazyLock<RwLock<HashMap<String, String>>> =
    LazyLock::new(|| RwLock::new(HashMap::default()));

pub fn init(cx: &mut App) {
    cx.set_global(SchemaStore::default());

    if let Some(extension_events) = extension::ExtensionEvents::try_global(cx) {
        cx.subscribe(&extension_events, move |_, evt, cx| match evt {
            extension::Event::ExtensionsInstalledChanged => {
                cx.update_global::<SchemaStore, _>(|schema_store, _cx| {
                    schema_store.notify_schema_changed();
                });
            }
            extension::Event::ExtensionUninstalled(_)
            | extension::Event::ExtensionInstalled(_)
            | extension::Event::ConfigureExtensionRequested(_) => {}
        })
        .detach();
    }
}

#[derive(Default)]
pub struct SchemaStore;

impl gpui::Global for SchemaStore {}

impl SchemaStore {
    /// 扩展安装状态变化时，清除依赖运行时状态的动态 schema 缓存。
    fn notify_schema_changed(&mut self) {
        DYNAMIC_SCHEMA_CACHE.write().clear();
    }
}

pub fn handle_schema_request(
    project: Entity<Project>,
    uri: String,
    cx: &mut AsyncApp,
) -> Task<Result<String>> {
    let path = match uri.strip_prefix(SCHEMA_URI_PREFIX) {
        Some(path) => path,
        None => return Task::ready(Err(anyhow::anyhow!("Invalid schema URI: {}", uri))),
    };

    if let Some(json) = resolve_static_schema(path) {
        return Task::ready(Ok(json));
    }

    if let Some(cached) = DYNAMIC_SCHEMA_CACHE.read().get(&uri).cloned() {
        return Task::ready(Ok(cached));
    }

    let languages = project.read_with(cx, |project, _| project.languages().clone());
    let path = path.to_string();
    let uri_clone = uri.clone();
    cx.spawn(async move |cx| {
        let schema = resolve_dynamic_schema(&languages, &path, cx).await?;
        let json = serde_json::to_string(&schema).context("Failed to serialize schema")?;

        DYNAMIC_SCHEMA_CACHE.write().insert(uri_clone, json.clone());

        Ok(json)
    })
}

fn resolve_static_schema(path: &str) -> Option<String> {
    let (schema_name, rest) = path.split_once('/').unzip();
    let schema_name = schema_name.unwrap_or(path);

    match schema_name {
        "tsconfig" => Some(TSCONFIG_SCHEMA.to_string()),
        "package_json" => Some(PACKAGE_JSON_SCHEMA.to_string()),
        "jsonc" => Some(JSONC_SCHEMA.clone()),
        "keymap" => Some(KEYMAP_SCHEMA.clone()),
        "zed_inspector_style" => {
            #[cfg(debug_assertions)]
            {
                Some(INSPECTOR_STYLE_SCHEMA.clone())
            }
            #[cfg(not(debug_assertions))]
            {
                Some(
                    serde_json::to_string(&schemars::json_schema!(true).to_value())
                        .expect("true schema should serialize"),
                )
            }
        }

        "action" => {
            let normalized_action_name = match rest {
                Some(name) => name,
                None => return None,
            };
            let action_name = denormalize_action_name(normalized_action_name);

            if let Some(cached) = ACTION_SCHEMA_CACHE.read().get(&action_name).cloned() {
                return Some(cached);
            }

            let mut generator = settings::KeymapFile::action_schema_generator();
            let schema =
                settings::KeymapFile::get_action_schema_by_name(&action_name, &mut generator);
            let json = serde_json::to_string(
                &root_schema_from_action_schema(schema, &mut generator).to_value(),
            )
            .expect("Action schema should serialize");

            ACTION_SCHEMA_CACHE
                .write()
                .insert(action_name, json.clone());
            Some(json)
        }

        _ => None,
    }
}

async fn resolve_dynamic_schema(
    languages: &Arc<LanguageRegistry>,
    path: &str,
    cx: &mut AsyncApp,
) -> Result<serde_json::Value> {
    let (schema_name, _rest) = path.split_once('/').unzip();
    let schema_name = schema_name.unwrap_or(path);

    let schema = match schema_name {
        "settings" => {
            let mut lsp_adapter_names: Vec<String> = languages
                .all_lsp_adapters()
                .into_iter()
                .map(|adapter| adapter.name())
                .chain(languages.available_lsp_adapter_names())
                .map(|name| name.to_string())
                .collect();

            let mut i = 0;
            while i < lsp_adapter_names.len() {
                let mut j = i + 1;
                while j < lsp_adapter_names.len() {
                    if lsp_adapter_names[i] == lsp_adapter_names[j] {
                        lsp_adapter_names.swap_remove(j);
                    } else {
                        j += 1;
                    }
                }
                i += 1;
            }

            cx.update(|cx| {
                let font_names = &cx.text_system().all_font_names();
                let language_names = &languages
                    .language_names()
                    .into_iter()
                    .map(|name| name.to_string())
                    .collect::<Vec<_>>();

                let mut icon_theme_names = vec![];
                let mut theme_names = vec![];
                if let Some(registry) = theme::ThemeRegistry::try_global(cx) {
                    icon_theme_names.extend(
                        registry
                            .list_icon_themes()
                            .into_iter()
                            .map(|icon_theme| icon_theme.name),
                    );
                    theme_names.extend(registry.list_names());
                }
                let icon_theme_names = icon_theme_names.as_slice();
                let theme_names = theme_names.as_slice();

                let action_names = cx.all_action_names();
                let action_documentation = cx.action_documentation();
                let deprecations = cx.deprecated_actions_to_preferred_actions();
                let deprecation_messages = cx.action_deprecation_messages();

                let mut schema =
                    settings::SettingsStore::json_schema(&settings::SettingsJsonSchemaParams {
                        language_names,
                        font_names,
                        theme_names,
                        icon_theme_names,
                        lsp_adapter_names: &lsp_adapter_names,
                        action_names,
                        action_documentation,
                        deprecations,
                        deprecation_messages,
                    });
                inject_feature_flags_schema(&mut schema);
                schema
            })
        }
        "project_settings" => {
            let lsp_adapter_names = languages
                .all_lsp_adapters()
                .into_iter()
                .map(|adapter| adapter.name().to_string())
                .collect::<Vec<_>>();

            let language_names = &languages
                .language_names()
                .into_iter()
                .map(|name| name.to_string())
                .collect::<Vec<_>>();

            let mut schema =
                settings::SettingsStore::project_json_schema(&settings::SettingsJsonSchemaParams {
                    language_names,
                    lsp_adapter_names: &lsp_adapter_names,
                    // These are not allowed in project-specific settings but
                    // they're still fields required by the
                    // `SettingsJsonSchemaParams` struct.
                    font_names: &[],
                    theme_names: &[],
                    icon_theme_names: &[],
                    action_names: &[],
                    action_documentation: &HashMap::default(),
                    deprecations: &HashMap::default(),
                    deprecation_messages: &HashMap::default(),
                });
            inject_feature_flags_schema(&mut schema);
            schema
        }
        "keymap" => cx.update(settings::KeymapFile::generate_json_schema_for_registered_actions),
        "action" => {
            anyhow::bail!("Action schemas are resolved statically");
        }
        _ => {
            anyhow::bail!("Unrecognized schema: {schema_name}");
        }
    };
    Ok(schema)
}

const JSONC_LANGUAGE_NAME: &str = "JSONC";

pub fn all_schema_file_associations(
    languages: &Arc<LanguageRegistry>,
    path: Option<settings::SettingsLocation<'_>>,
    cx: &mut App,
) -> serde_json::Value {
    let extension_globs = languages
        .available_language_for_name(JSONC_LANGUAGE_NAME)
        .map(|language| language.matcher().path_suffixes.clone())
        .into_iter()
        .flatten()
        // Path suffixes can be entire file names or just their extensions.
        .flat_map(|path_suffix| [format!("*.{path_suffix}"), path_suffix]);
    let override_globs = AllLanguageSettings::get(path, cx)
        .file_types
        .get(JSONC_LANGUAGE_NAME)
        .into_iter()
        .flat_map(|(_, glob_strings)| glob_strings)
        .cloned();
    let jsonc_globs = extension_globs.chain(override_globs).collect::<Vec<_>>();

    let mut file_associations = serde_json::json!([
        {
            "fileMatch": [
                schema_file_match(paths::settings_file()),
            ],
            "url": format!("{SCHEMA_URI_PREFIX}settings"),
        },
        {
            "fileMatch": [
            paths::local_settings_file_relative_path()],
            "url": format!("{SCHEMA_URI_PREFIX}project_settings"),
        },
        {
            "fileMatch": [schema_file_match(paths::keymap_file())],
            "url": format!("{SCHEMA_URI_PREFIX}keymap"),
        },
        {
            "fileMatch": ["tsconfig.json"],
            "url": format!("{SCHEMA_URI_PREFIX}tsconfig")
        },
        {
            "fileMatch": ["package.json"],
            "url": format!("{SCHEMA_URI_PREFIX}package_json")
        },
        {
            "fileMatch": &jsonc_globs,
            "url": format!("{SCHEMA_URI_PREFIX}jsonc")
        },
    ]);

    #[cfg(debug_assertions)]
    {
        file_associations
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "fileMatch": [
                    "zed-inspector-style.json"
                ],
                "url": format!("{SCHEMA_URI_PREFIX}zed_inspector_style")
            }));
    }

    file_associations
        .as_array_mut()
        .unwrap()
        .extend(cx.all_action_names().into_iter().map(|&name| {
            let normalized_name = normalize_action_name(name);
            let file_name = normalized_action_name_to_file_name(normalized_name.clone());
            serde_json::json!({
                "fileMatch": [file_name],
                "url": format!("{SCHEMA_URI_PREFIX}action/{normalized_name}")
            })
        }));

    file_associations
}

/// Swaps the placeholder [`settings::FeatureFlagsMap`] subschema produced by
/// schemars for an enriched one that lists each known flag's variants. The
/// placeholder is registered in the `settings_content` crate so the
/// `settings` crate doesn't need a reverse dependency on `feature_flags`.
fn inject_feature_flags_schema(schema: &mut serde_json::Value) {
    use schemars::JsonSchema;

    let Some(defs) = schema.get_mut("$defs").and_then(|d| d.as_object_mut()) else {
        return;
    };
    let schema_name = settings::FeatureFlagsMap::schema_name();
    let enriched = feature_flags::generate_feature_flags_schema().to_value();
    defs.insert(schema_name.into_owned(), enriched);
}

fn generate_jsonc_schema() -> serde_json::Value {
    let generator = schemars::generate::SchemaSettings::draft2019_09()
        .with_transform(DefaultDenyUnknownFields)
        .with_transform(AllowTrailingCommas)
        .into_generator();
    let meta_schema = generator
        .settings()
        .meta_schema
        .as_ref()
        .expect("meta_schema should be present in schemars settings")
        .to_string();
    let defs = generator.definitions();
    let schema = schemars::json_schema!({
        "$schema": meta_schema,
        "allowTrailingCommas": true,
        "$defs": defs,
    });
    serde_json::to_value(schema).unwrap()
}

#[cfg(debug_assertions)]
fn generate_inspector_style_schema() -> serde_json::Value {
    let schema = schemars::generate::SchemaSettings::draft2019_09()
        .with_transform(util::schemars::DefaultDenyUnknownFields)
        .into_generator()
        .root_schema_for::<gpui::StyleRefinement>();

    serde_json::to_value(schema).unwrap()
}

pub fn normalize_action_name(action_name: &str) -> String {
    action_name.replace("::", "__")
}

pub fn denormalize_action_name(action_name: &str) -> String {
    action_name.replace("__", "::")
}

pub fn normalized_action_file_name(action_name: &str) -> String {
    normalized_action_name_to_file_name(normalize_action_name(action_name))
}

pub fn normalized_action_name_to_file_name(mut normalized_action_name: String) -> String {
    normalized_action_name.push_str(".json");
    normalized_action_name
}

fn root_schema_from_action_schema(
    action_schema: Option<schemars::Schema>,
    generator: &mut schemars::SchemaGenerator,
) -> schemars::Schema {
    let Some(mut action_schema) = action_schema else {
        return schemars::json_schema!(false);
    };
    let meta_schema = generator
        .settings()
        .meta_schema
        .as_ref()
        .expect("meta_schema should be present in schemars settings")
        .to_string();
    let defs = generator.definitions();
    let mut schema = schemars::json_schema!({
        "$schema": meta_schema,
        "allowTrailingCommas": true,
        "$defs": defs,
    });
    schema
        .ensure_object()
        .extend(std::mem::take(action_schema.ensure_object()));
    schema
}

#[inline]
fn schema_file_match(path: &std::path::Path) -> String {
    path.strip_prefix(path.parent().unwrap().parent().unwrap())
        .unwrap()
        .display()
        .to_string()
        .replace('\\', "/")
}
