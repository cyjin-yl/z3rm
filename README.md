# Zerminal

A high-performance GPU-rendered terminal with a built-in multiplexer, read-only file viewer with diff review, and QuickJS extension system.

Forked from [Zed](https://github.com/zed-industries/zed). All editor, AI, and collaboration features removed. The retained core: GPUI rendering engine, terminal emulation (alacritty-based), workspace pane management, theme/settings infrastructure, and a slimmed read-only editor for file/diff viewing.

## Features

- **GPU-rendered terminal** — powered by GPUI
- **Built-in multiplexer** — tmux-class session management with detach/reattach
- **Server-canonical architecture** — mux_server owns PTY + terminal state; GUI client renders grid
- **File viewer & diff review** — read-only editor with syntax highlighting for CLI agent workflows
- **Shadow snapshot engine** — fine-grained filesystem versioning for undo/decline
- **QuickJS extension system** — all UI chrome implemented as extensions
- **Remote sessions** — SSH tunnel support with auto server installation

## Building

- [Building for Linux](./docs/development/building-linux.md)
- [Building for Windows](./docs/development/building-windows.md)

## License

Zerminal source code is licensed under GPL-3.0-or-later (inherited from Zed) with Apache-2.0 components where marked. New zerminal crates are Apache-2.0.
