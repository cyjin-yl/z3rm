# Zerminal Foundation — Implementation Plan Index

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement these plans task-by-task.

**Goal:** Convert a Zed editor fork into zerminal — a high-performance GPU-rendered terminal + multiplexer with server-canonical architecture, QuickJS extension system, shadow snapshot engine, and file/diff viewer.

**Architecture:** Server-canonical (mux_server owns PTY + alacritty emulator + grid + layout). Client renders grid only. All layout logic server-side. QuickJS extension host Day 0. Shadow snapshot Day 0.

**Tech Stack:** Rust, GPUI (from Zed), alacritty terminal engine, prost/protobuf, QuickJS (rquickjs), SQLite, interprocess crate.

**Spec:** `docs/superpowers/specs/2026-07-14-zerminal-foundation-design.md`

---

## Plan Dependency Order

Plans must be executed in order. Each plan depends on the previous.

| # | Plan | Depends On | Key Deliverable |
|---|---|---|---|
| 1 | [Foundation Setup](./2026-07-14-plan-01-foundation-setup.md) | — | zerminal_macros crate, migration feature flag, Cargo profile |
| 2 | [Branding & Naming](./2026-07-14-plan-02-branding-naming.md) | 1 | APP_NAME, paths, binary name, README |
| 3 | [AGENTS.md & .rules Rewrite](./2026-07-14-plan-03-agents-rules.md) | 2 | AGENTS.md (symlink CLAUDE.md), .rules |
| 4 | [Crate Kill List](./2026-07-14-plan-04-crate-kill-list.md) | 3 | Cargo.toml cleanup, ~90 crates removed |
| 5 | [Pass 1: Static Analysis Scan](./2026-07-14-plan-05-pass1-scan.md) | 4 | All migration holes marked with #[zerminal_todo] |
| 6 | [Pass 2: Migration — removed-crate holes](./2026-07-14-plan-06-pass2-removed-crate.md) | 5 | removed-crate count = 0 |
| 7 | [Pass 2: Migration — broken-ref holes](./2026-07-14-plan-07-pass2-broken-ref.md) | 6 | broken-ref count = 0 |
| 8 | [Pass 2: Migration — stub & disabled-feature](./2026-07-14-plan-08-pass2-stubs.md) | 7 | total hole count = 0 |
| 9 | [mux_protocol](./2026-07-14-plan-09-mux-protocol.md) | 8 | prost wire types, grid diff, file fetch, clipboard, version negotiation |
| 10 | [mux_server](./2026-07-14-plan-10-mux-server.md) | 9 | PTY management, alacritty emulator, layout engine, keepalive, session persistence |
| 11 | [mux (client)](./2026-07-14-plan-11-mux-client.md) | 10 | MuxDomain struct, transport enum, grid sync, fetch APIs |
| 12 | [zerminal entry point](./2026-07-14-plan-12-entry-point.md) | 11 | slimmed main.rs, daemon auto-spawn, window creation |
| 13 | [shadow_snapshot](./2026-07-14-plan-13-shadow-snapshot.md) | 9 | version tree, WAL, SQLite, delta chain, quota GC |
| 14 | [quickjs_runtime + extension system](./2026-07-14-plan-14-quickjs-extensions.md) | 12 | QuickJS bundled, resource limits, extension host, chrome baseline |
| 15 | [workspace migration](./2026-07-14-plan-15-workspace-migration.md) | 10, 11 | pane_group → server, client = layout renderer |
| 16 | [settings schema rewrite](./2026-07-14-plan-16-settings.md) | 12 | terminal/mux/extension settings schema |
| 17 | [keymap profiles](./2026-07-14-plan-17-keymap-profiles.md) | 16 | default + tmux + zellij + screen profiles |
| 18 | [file viewer & diff](./2026-07-14-plan-18-file-viewer-diff.md) | 13, 15 | editor readonly, worktree on mux_protocol, auto-split-right |
| 19 | [remote connection](./2026-07-14-plan-19-remote.md) | 12 | SSH exec, auto-install, extension sync |
| 20 | [clipboard](./2026-07-14-plan-20-clipboard.md) | 9, 10 | server relay hub, OSC 52, bracketed paste, path forwarding |
| 21 | [input routing](./2026-07-14-plan-21-input-routing.md) | 17 | priority chain, prefix mode, nesting passthrough |
| 22 | [scrollback](./2026-07-14-plan-22-scrollback.md) | 10, 11 | per-client scroll, sync scroll, fetch protocol, cache invalidation |
| 23 | [testing & verification](./2026-07-14-plan-23-testing.md) | all | unit tests + e2e integration test |
| 24 | [logging & diagnostics](./2026-07-14-plan-24-logging.md) | 12 | file logs, status CLI, GPUI notifications |
| 25 | [final compilation gate](./2026-07-14-plan-25-final-gate.md) | all | `cargo check` without migration feature → clean → done |
