# Z3rm Foundation — Implementation Plan Index

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement these plans task-by-task.

**Goal:** Convert a Zed editor fork into z3rm — a high-performance GPU-rendered terminal + multiplexer with server-canonical architecture, QuickJS extension system, shadow snapshot engine, file/diff viewer, UDP resilient transport, and comprehensive testing.

**Architecture:** Server-canonical (mux_server owns PTY + alacritty emulator + grid + layout). Client renders grid only. All layout logic server-side. QuickJS extension host. Shadow snapshot. UDP resilient transport. All Day 0.

**Tech Stack:** Rust, GPUI (from Zed), alacritty terminal engine, prost/protobuf, QuickJS (rquickjs), SQLite, interprocess, AES-256-GCM, zstd.

**Spec:** `docs/superpowers/specs/2026-07-14-z3rm-foundation-design.md`

**Post-foundation (only 3 items):** Extension marketplace, Kitty graphics protocol / iTerm2 OSC 1337, Log viewer UI. Everything else is Day 0.

---

## Plan Execution Order

Plans must be executed in order. Each plan depends on the previous.

| # | Plan | Depends On | Key Deliverable |
|---|---|---|---|
| 1 | [Foundation Setup](./2026-07-14-plan-01-foundation-setup.md) | — | z3rm_macros crate, migration feature flag |
| 2 | [Branding & Naming](./2026-07-14-plan-02-branding-naming.md) | 1 | APP_NAME, paths, binary name, README |
| 3 | [AGENTS.md & .rules Rewrite](./2026-07-14-plan-03-agents-rules.md) | 2 | AGENTS.md (symlink CLAUDE.md), .rules |
| 4 | [Crate Kill List](./2026-07-14-plan-04-crate-kill-list.md) | 3 | ~90 crates removed from workspace |
| 5 | [Pass 1: Static Analysis Scan](./2026-07-14-plan-05-08-two-pass-migration.md) | 4 | All holes marked with #[z3rm_todo] |
| 6 | [Pass 2: removed-crate holes](./2026-07-14-plan-05-08-two-pass-migration.md) | 5 | removed-crate count = 0 |
| 7 | [Pass 2: broken-ref holes](./2026-07-14-plan-05-08-two-pass-migration.md) | 6 | broken-ref count = 0 |
| 8 | [Pass 2: stubs & disabled-feature](./2026-07-14-plan-05-08-two-pass-migration.md) | 7 | total hole count = 0 |
| 9 | [mux_protocol](./2026-07-14-plan-09-mux-protocol.md) | 8 | prost wire types (complete .proto) |
| 10 | [mux_server](./2026-07-14-plan-10-mux-server.md) | 9 | PTY, alacritty, layout, keepalive, persistence |
| 11 | [mux (client)](./2026-07-14-plan-11-mux-client.md) | 10 | MuxDomain struct, transport, grid sync |
| 12 | [z3rm entry point](./2026-07-14-plan-12-entry-point.md) | 11 | slimmed main.rs, daemon auto-spawn |
| 13 | [shadow_snapshot](./2026-07-14-plan-13-shadow-snapshot.md) | 9 | version tree, WAL, SQLite, delta chain, GC |
| 14 | [quickjs_runtime + extensions](./2026-07-14-plan-14-quickjs-extensions.md) | 12 | QuickJS, extension host, chrome baseline |
| 15 | [workspace migration](./2026-07-14-plan-15-workspace-migration.md) | 10, 11 | pane_group → server, client = renderer |
| 16 | [settings schema](./2026-07-14-plan-16-settings.md) | 12 | terminal/mux/extension settings |
| 17 | [keymap profiles](./2026-07-14-plan-17-keymap-profiles.md) | 16 | default + tmux + zellij + screen + prefix mode |
| 18 | [file viewer & diff](./2026-07-14-plan-18-file-viewer-diff.md) | 13, 15 | editor readonly, worktree RPC, diff review |
| 19 | [remote connection](./2026-07-14-plan-19-remote.md) | 12 | SSH, auto-install, extension sync |
| 20 | [clipboard](./2026-07-14-plan-20-clipboard.md) | 9, 10 | server relay hub, OSC 52, path forwarding |
| 21 | [input routing](./2026-07-14-plan-21-input-routing.md) | 17 | priority chain, prefix, passthrough, IME |
| 22 | [scrollback](./2026-07-14-plan-22-scrollback.md) | 10, 11 | per-client + sync scroll, fetch, search |
| 23 | [testing & verification](./2026-07-14-plan-23-testing.md) | all crates | unit, integration, property, conformance, fuzz |
| 24 | [logging & diagnostics](./2026-07-14-plan-24-logging.md) | 12 | file logs, status CLI, GPUI notifications |
| 25 | [transport_resilient](./2026-07-14-plan-25-transport-resilient.md) | 9 | UDP AEAD, roaming, RTT, frame-rate control |
| 26 | [final compilation gate](./2026-07-14-plan-26-final-gate.md) | all | clean build, all tests pass, SLOs met |
| 27 | [CLI control interface](./2026-07-14-plan-27-cli-control-interface.md) | 9, 11 | tmux-compatible send-keys/capture-pane/ls |
