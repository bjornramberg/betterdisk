# betterdisk
[![Built With Ratatui](https://img.shields.io/badge/Built_With_Ratatui-000?logo=ratatui&logoColor=fff)](https://ratatui.rs/)

A fast TUI disk space analyzer for Linux written in Rust.

## Features

- Browse and analyze disk usage across all mounted filesystems
- Bar chart visualization showing space consumption per directory
- Drill-down navigation to explore folder sizes in detail
- Keyboard-driven interface

## Controls

| Key | Action |
|-----|--------|
| `↑` `↓` | Navigate selection |
| `Enter` | Select drive / Enter directory |
| `Backspace` | Go to parent directory |
| `r` | Refresh scan |
| `q` | Quit |

## Building

```bash
cargo build --release
```

## Running

```bash
./target/release/betterdisk
```

## Dependencies

- [ratatui](https://crates.io/crates/ratatui) - Terminal UI framework
- [sysinfo](https://crates.io/crates/sysinfo) - System information
- [walkdir](https://crates.io/crates/walkdir) - Directory traversal
