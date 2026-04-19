# betterdisk

A fast TUI disk space analyzer for Linux with treemap visualization, written in Rust.

## Features

- Treemap visualization showing disk usage as proportional colored rectangles
- Drill-down navigation to explore folder sizes in detail
- Full keyboard-driven interface
- Drive selector modal popup

## Screenshot

```
┌──────────────────────────────────────────────────────────────┐
│  /home  [████████████████████████░░░░░░░░░] 45.2 / 100 GB    │
├──────────────────────────────────────────────────────────────┤
│  ┌──────────┬──────────┬───────┐                             │
│  │ Downloads│  .cache  │ .var  │                             │
│  │   8G    │     25G   │ 10G   │                             │
│  ├─────────┴───────────┴───────┤                             │
│  │     .config         │ .local│                             │
│  │       5G            │  12G  │                             │
│  ├─────────────────────┴───────┤                             │
│  │      ...                    │                             │
│  └─────────────────────────────┘                             │
├──────────────────────────────────────────────────────────────┤
│  > .cache │ 25.2 GB │ 56% │ ↑Enter drill │ /drive            │
└──────────────────────────────────────────────────────────────┘
```

**Drive Selector Modal:**
```
     ┌─────────────────────────┐
     │   Select Drive          │
     ├─────────────────────────┤
     │> / (100GB)              │
     │  /home (50GB)           │
     │  /mnt/data (500GB)      │
     └─────────────────────────┘
```

## Controls

| Key | Action |
|-----|--------|
| `↑` `↓` `←` `→` | Navigate treemap cells |
| `Enter` | Drill into selected folder |
| `Backspace` | Go to parent directory |
| `/` | Open drive selector |
| `Tab` | Toggle drive selector |
| `r` | Refresh scan |
| `q` | Quit |

## Building

```bash
cargo build --release
```

The binary will be at `target/release/betterdisk`.

## Running

```bash
./target/release/betterdisk
```

## Dependencies

- [ratatui](https://crates.io/crates/ratatui) - Terminal UI framework
- [sysinfo](https://crates.io/crates/sysinfo) - System/disk information
- [walkdir](https://crates.io/crates/walkdir) - Directory traversal
