mod extension_lsp_adapter;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use extension::{ExtensionGrammarProxy, ExtensionHostProxy, ExtensionLanguageProxy};
use gpui::{App, Entity};
use language::{LanguageMatcher, LanguageName, LanguageRegistry, LoadedLanguage};
use project::Project;

#[derive(Clone)]
pub enum LspAccess {
    /// project::LspStore 已删除，z3rm 使用 ViaWorkspaces。
    ViaWorkspaces(Arc<dyn Fn(&mut App) -> Result<Vec<Entity<Project>>> + Send + Sync + 'static>),
    Noop,
}

pub fn init(
    lsp_access: LspAccess,
    extension_host_proxy: Arc<ExtensionHostProxy>,
    language_registry: Arc<LanguageRegistry>,
) {
    let _ = lsp_access;
    let language_server_registry_proxy = LanguageServerRegistryProxy { language_registry };
    extension_host_proxy.register_grammar_proxy(language_server_registry_proxy.clone());
    extension_host_proxy.register_language_proxy(language_server_registry_proxy.clone());
    extension_host_proxy.register_language_server_proxy(language_server_registry_proxy);
}

#[derive(Clone)]
struct LanguageServerRegistryProxy {
    language_registry: Arc<LanguageRegistry>,
}

impl ExtensionGrammarProxy for LanguageServerRegistryProxy {
    #[ztracing::instrument(skip_all)]
    fn register_grammars(&self, grammars: Vec<(Arc<str>, PathBuf)>) {
        self.language_registry.register_wasm_grammars(grammars)
    }
}

impl ExtensionLanguageProxy for LanguageServerRegistryProxy {
    fn register_language(
        &self,
        language: LanguageName,
        grammar: Option<Arc<str>>,
        matcher: LanguageMatcher,
        hidden: bool,
        load: Arc<dyn Fn() -> Result<LoadedLanguage> + Send + Sync + 'static>,
    ) {
        self.language_registry
            .register_language(language, grammar, matcher, hidden, None, load);
    }

    fn remove_languages(
        &self,
        languages_to_remove: &[LanguageName],
        grammars_to_remove: &[Arc<str>],
    ) {
        self.language_registry
            .remove_languages(languages_to_remove, grammars_to_remove);
    }
}
