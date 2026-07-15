# Plan 4: Crate Kill List — Cargo.toml Cleanup

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Remove ~90 deleted crates from the workspace. Delete their directories. Remove their entries from workspace `Cargo.toml`. This will break compilation — that is expected. All broken references will be marked with `#[z3rm_todo("removed-crate", ...)]` in Plan 5.

**Architecture:** Clean cut. Remove the crate directories and workspace member entries. Do NOT attempt to fix references yet — that is Pass 1 (Plan 5).

---

### Task 1: Delete AI/Agent/LLM crates

**Files:**
- Delete: `crates/agent/`, `crates/agent_servers/`, `crates/agent_settings/`, `crates/agent_skills/`, `crates/agent_ui/`, `crates/ai_onboarding/`, `crates/acp_thread/`, `crates/acp_tools/`
- Delete: `crates/edit_prediction/`, `crates/edit_prediction_cli/`, `crates/edit_prediction_context/`, `crates/edit_prediction_metrics/`, `crates/edit_prediction_types/`, `crates/edit_prediction_ui/`
- Delete: `crates/prompt_store/`, `crates/web_search/`, `crates/web_search_providers/`

- [ ] **Step 1: Delete crate directories**

```bash
rm -rf crates/agent crates/agent_servers crates/agent_settings crates/agent_skills crates/agent_ui
rm -rf crates/ai_onboarding crates/acp_thread crates/acp_tools
rm -rf crates/edit_prediction crates/edit_prediction_cli crates/edit_prediction_context crates/edit_prediction_metrics crates/edit_prediction_types crates/edit_prediction_ui
rm -rf crates/prompt_store crates/web_search crates/web_search_providers
```

- [ ] **Step 2: Remove from workspace Cargo.toml members**

Remove each `"crates/agent"`, etc. line from the `members` array in root `Cargo.toml`.

- [ ] **Step 3: Remove from workspace.dependencies**

Remove each `agent = { path = "crates/agent" }` etc. from `[workspace.dependencies]`.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "Remove AI/Agent/LLM crates"
```

### Task 2: Delete LLM Provider crates

**Files:**
- Delete: `crates/anthropic/`, `crates/bedrock/`, `crates/cloud_llm_client/`, `crates/codestral/`, `crates/deepseek/`, `crates/google_ai/`, `crates/language_model/`, `crates/language_model_core/`, `crates/language_models/`, `crates/language_models_cloud/`, `crates/lmstudio/`, `crates/mistral/`, `crates/ollama/`, `crates/open_ai/`, `crates/open_router/`, `crates/opencode/`, `crates/x_ai/`
- Delete: `crates/copilot/`, `crates/copilot_chat/`
- Delete: `crates/cloud_api_client/`, `crates/cloud_api_types/`

- [ ] **Step 1: Delete directories**

```bash
rm -rf crates/anthropic crates/bedrock crates/cloud_llm_client crates/codestral crates/deepseek
rm -rf crates/google_ai crates/language_model crates/language_model_core crates/language_models crates/language_models_cloud
rm -rf crates/lmstudio crates/mistral crates/ollama crates/open_ai crates/open_router crates/opencode crates/x_ai
rm -rf crates/copilot crates/copilot_chat
rm -rf crates/cloud_api_client crates/cloud_api_types
```

- [ ] **Step 2: Remove from workspace Cargo.toml**

Remove all corresponding entries from `members` and `[workspace.dependencies]`.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "Remove LLM provider crates"
```

### Task 3: Delete Collaboration/Communication crates

**Files:**
- Delete: `crates/collab/`, `crates/collab_ui/`, `crates/channel/`, `crates/call/`, `crates/audio/`, `crates/livekit_api/`, `crates/livekit_client/`, `crates/client/`

- [ ] **Step 1: Delete directories**

```bash
rm -rf crates/collab crates/collab_ui crates/channel crates/call crates/audio
rm -rf crates/livekit_api crates/livekit_client crates/client
```

- [ ] **Step 2: Remove from workspace Cargo.toml**

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "Remove collaboration/communication crates"
```

### Task 4: Delete Editor auxiliary crates

**Files:**
- Delete: `crates/vim/`, `crates/vim_mode_setting/`, `crates/debugger_tools/`, `crates/debugger_ui/`, `crates/dap/`, `crates/dap_adapters/`, `crates/debug_adapter_extension/`, `crates/repl/`, `crates/svg_preview/`, `crates/csv_preview/`, `crates/image_viewer/`, `crates/mermaid_render/`, `crates/prettier/`, `crates/dev_container/`, `crates/tasks_ui/`, `crates/task/`, `crates/feedback/`, `crates/journal/`

- [ ] **Step 1: Delete directories**

```bash
rm -rf crates/vim crates/vim_mode_setting
rm -rf crates/debugger_tools crates/debugger_ui crates/dap crates/dap_adapters crates/debug_adapter_extension crates/repl
rm -rf crates/svg_preview crates/csv_preview crates/image_viewer crates/mermaid_render crates/prettier crates/dev_container
rm -rf crates/tasks_ui crates/task crates/feedback crates/journal
```

- [ ] **Step 2: Remove from workspace Cargo.toml**

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "Remove editor auxiliary crates"
```

### Task 5: Delete Other crates

**Files:**
- Delete: `crates/node_runtime/`, `crates/schema_generator/`, `crates/streaming_diff/`, `crates/media/`, `crates/migrator/`, `crates/install_cli/`, `crates/open_path_prompt/`, `crates/onboarding/`, `crates/language_onboarding/`, `crates/system_specs/`, `crates/time_format/`, `crates/scheduler/`, `crates/watch/`, `crates/input_latency_ui/`, `crates/inspector_ui/`, `crates/miniprofiler_ui/`, `crates/component_preview/`, `crates/markdown_preview/`, `crates/language_tools/`, `crates/snippet/`, `crates/snippet_provider/`, `crates/snippets_ui/`, `crates/outline/`, `crates/outline_panel/`, `crates/breadcrumbs/`, `crates/action_log/`, `crates/activity_indicator/`, `crates/context_server/`, `crates/windows_resources/`, `crates/toolchain_selector/`, `crates/explorer_command_injector/`, `crates/eval_cli/`, `crates/eval_utils/`
- Delete all benchmark crates: `crates/editor_benchmarks/`, `crates/project_benchmarks/`, `crates/worktree_benchmarks/`, `crates/fs_benchmarks/`, `crates/benchmarks/`

- [ ] **Step 1: Delete directories**

```bash
rm -rf crates/node_runtime crates/schema_generator crates/streaming_diff crates/media crates/migrator
rm -rf crates/install_cli crates/open_path_prompt crates/onboarding crates/language_onboarding
rm -rf crates/system_specs crates/time_format crates/scheduler crates/watch
rm -rf crates/input_latency_ui crates/inspector_ui crates/miniprofiler_ui crates/component_preview
rm -rf crates/markdown_preview crates/language_tools
rm -rf crates/snippet crates/snippet_provider crates/snippets_ui
rm -rf crates/outline crates/outline_panel crates/breadcrumbs
rm -rf crates/action_log crates/activity_indicator crates/context_server
rm -rf crates/windows_resources crates/toolchain_selector crates/explorer_command_injector
rm -rf crates/eval_cli crates/eval_utils
rm -rf crates/editor_benchmarks crates/project_benchmarks crates/worktree_benchmarks crates/fs_benchmarks crates/benchmarks
```

- [ ] **Step 2: Remove from workspace Cargo.toml**

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "Remove remaining deleted crates"
```

### Task 6: Verify workspace structure

- [ ] **Step 1: Count remaining crates**

Run: `ls crates/ | wc -l`

Expected: approximately 100-110 remaining crates (retained + new foundation crates).

- [ ] **Step 2: Verify workspace Cargo.toml has no dangling references**

Run: `grep -c 'path = "crates/' Cargo.toml` and compare with actual directories in `crates/`.

- [ ] **Step 3: Commit any final Cargo.toml cleanup**

```bash
git add -A
git commit -m "Clean up workspace Cargo.toml after crate deletion"
```
