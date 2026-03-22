# rust-tui

A toy TUI library written in Rust. This is a learning project and a work in progress.

## About

This project is an experiment in building a terminal UI framework from scratch — no `ratatui`, no `crossterm`, just raw POSIX syscalls via `libc` and hand-rolled ANSI escape sequences.

The goal is to understand how TUI libraries work under the hood by implementing the foundational pieces directly.

## Features

- **`EscapeSequencer`** — low-level ANSI/VT100 cursor movement and screen control
- **`AnsiCode`** — full SGR (Select Graphic Rendition) enum covering text styles, 8-color, bright, and true-color (24-bit RGB) foreground/background
- **`TerminalSurface`** — a rectangular rendering region with ANSI-aware word wrapping
- **`TerminalRenderer`** — manages a stack of surfaces and handles re-rendering
- Terminal resize handling via `SIGWINCH`

## Building

Requires Rust (edition 2024) and Cargo.

```sh
cargo build
cargo run
```

## Status

Early-stage / WIP. The current demo renders an ANSI block-art birdcage and reacts to terminal resize events. The library API is not stable.

## Dependencies

- [`libc`](https://crates.io/crates/libc) — POSIX syscalls (`ioctl`, `signal`, `TIOCGWINSZ`)
