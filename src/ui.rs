use std::io;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table},
    Frame, Terminal,
};

use crate::scanner::DirEntry;
use crate::state::{AppState, MountInfo};

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn draw_mount_selector(f: &mut Frame, state: &AppState, area: Rect) {
    let items: Vec<ListItem> = state
        .mounts
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let label = format!(
                "{} {} ({})",
                if i == state.selected_mount { ">" } else { " " },
                m.mount_point.to_string_lossy(),
                format_size(m.total_space)
            );
            ListItem::new(label)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Select Drive"))
        .highlight_style(Color::Cyan);

    f.render_widget(list, area);
}

fn draw_disk_usage(f: &mut Frame, mount: &MountInfo, area: Rect) {
    let used_percent = if mount.total_space > 0 {
        (mount.used_space as f64 / mount.total_space as f64) * 100.0
    } else {
        0.0
    };

    let bar_width = ((area.width.saturating_sub(2)) as f64 * used_percent / 100.0) as u16;
    let w = (area.width.saturating_sub(2)) as usize;
    let bar: String = "█".repeat(bar_width as usize).chars().take(w).collect();
    let empty: String = "░".repeat(w.saturating_sub(bar_width as usize)).chars().take(w).collect();

    let gauge = format!("{}|{} {:5.1}%", bar, empty, used_percent);

    let paragraph = Paragraph::new(gauge)
        .block(Block::default().borders(Borders::ALL).title("Disk Usage"));

    f.render_widget(paragraph, area);
}

fn draw_directory_view(f: &mut Frame, state: &AppState, area: Rect) {
    let entry = match &state.current_entry {
        Some(e) => e,
        None => return,
    };

    let max_items = (area.height as usize).saturating_sub(2);
    let visible_entries: Vec<&DirEntry> = entry
        .children
        .iter()
        .skip(state.scroll_offset)
        .take(max_items)
        .collect();

    let total_size = entry.size;
    let rows: Vec<Row> = visible_entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let percent = if total_size > 0 {
                (e.size as f64 / total_size as f64 * 100.0) as u64
            } else {
                0
            };
            let name = e.path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| e.path.to_string_lossy().to_string());
            let bar_len = (percent as f64 / 10.0).min(50.0) as usize;
            let bar = "█".repeat(bar_len);
            Row::new(vec![
                format!("{}{}", if state.scroll_offset + i == state.selected_index { ">" } else { " " }, name),
                format_size(e.size),
                format!("{}%", percent),
                bar,
            ])
        })
        .collect();

    if rows.is_empty() {
        let paragraph = Paragraph::new("No subdirectories")
            .block(Block::default().borders(Borders::ALL).title(state.current_path.to_string_lossy().to_string()));
        f.render_widget(paragraph, area);
        return;
    }

    let widths = [
        Constraint::Percentage(30),
        Constraint::Percentage(15),
        Constraint::Percentage(10),
        Constraint::Percentage(45),
    ];

    let table = Table::new(rows, widths)
        .block(Block::default().borders(Borders::ALL).title(state.current_path.to_string_lossy().to_string()))
        .highlight_style(Color::Cyan);

    f.render_widget(table, area);
}

fn draw_status_bar(f: &mut Frame, state: &AppState, area: Rect) {
    let status = if state.is_scanning {
        "Scanning...".to_string()
    } else if let Some(mount) = state.get_selected_mount() {
        let free = format_size(mount.available_space);
        let used = format_size(mount.used_space);
        let total = format_size(mount.total_space);
        format!("Used: {} | Free: {} | Total: {}", used, free, total)
    } else {
        "No drive selected".to_string()
    };

    let paragraph = Paragraph::new(status)
        .block(Block::default().borders(Borders::ALL).title("Status"));

    f.render_widget(paragraph, area);
}

pub fn run_app() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = AppState::new();
    state.refresh_mounts();

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Percentage(20),
                    Constraint::Percentage(60),
                    Constraint::Length(3),
                ])
                .split(f.area());

            if let Some(mount) = state.get_selected_mount() {
                draw_disk_usage(f, mount, chunks[0]);
            }

            draw_mount_selector(f, &state, chunks[1]);

            if state.current_entry.is_some() {
                draw_directory_view(f, &state, chunks[2]);
            }

            draw_status_bar(f, &state, chunks[3]);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Up => {
                            if state.current_entry.is_some() {
                                if state.selected_index > 0 {
                                    state.selected_index -= 1;
                                    if state.selected_index < state.scroll_offset {
                                        state.scroll_offset = state.selected_index;
                                    }
                                }
                            } else if state.selected_mount > 0 {
                                state.select_mount(state.selected_mount - 1);
                            }
                        }
                        KeyCode::Down => {
                            if let Some(entry) = &state.current_entry {
                                if state.selected_index < entry.children.len().saturating_sub(1) {
                                    state.selected_index += 1;
                                    let max_visible = 18usize;
                                    if state.selected_index >= state.scroll_offset + max_visible {
                                        state.scroll_offset = state.selected_index.saturating_sub(max_visible) + 1;
                                    }
                                }
                            } else if state.selected_mount < state.mounts.len() - 1 {
                                state.select_mount(state.selected_mount + 1);
                            }
                        }
                        KeyCode::Enter => {
                            state.is_scanning = true;
                            if let Some(entry) = &state.current_entry {
                                if let Some(child) = entry.children.get(state.selected_index) {
                                    state.current_path = child.path.clone();
                                    state.selected_index = 0;
                                    state.scroll_offset = 0;
                                    let scanned = state.scanner.scan_dir(&child.path, 2);
                                    state.current_entry = Some(scanned);
                                }
                            } else {
                                state.select_mount(state.selected_mount);
                                let scanned = state.scanner.scan_dir(&state.root_path, 2);
                                state.current_entry = Some(scanned);
                            }
                            state.is_scanning = false;
                        }
                        KeyCode::Backspace => {
                            if let Some(entry) = &state.current_entry {
                                if let Some(parent) = entry.path.parent() {
                                    if parent != state.root_path && !parent.to_string_lossy().is_empty() {
                                        state.current_path = parent.to_path_buf();
                                        state.selected_index = 0;
                                        state.scroll_offset = 0;
                                        state.is_scanning = true;
                                        let scanned = state.scanner.scan_dir(&parent.to_path_buf(), 2);
                                        state.current_entry = Some(scanned);
                                        state.is_scanning = false;
                                    } else {
                                        state.current_entry = None;
                                        state.current_path = state.root_path.clone();
                                    }
                                }
                            }
                        }
                        KeyCode::Char('r') => {
                            state.is_scanning = true;
                            let scanned = state.scanner.scan_dir(&state.current_path, 2);
                            state.current_entry = Some(scanned);
                            state.is_scanning = false;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    terminal.show_cursor()?;
    execute!(std::io::stdout(), LeaveAlternateScreen)?;

    Ok(())
}