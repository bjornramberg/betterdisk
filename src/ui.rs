use std::io;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect, Margin},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, List, ListItem, Clear},
    Frame, Terminal,
};

use crate::state::AppState;

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

const COLOR_PALETTE: [Color; 8] = [
    Color::Blue,
    Color::Green,
    Color::Yellow,
    Color::Magenta,
    Color::Cyan,
    Color::LightBlue,
    Color::LightGreen,
    Color::LightYellow,
];

fn draw_top_bar(f: &mut Frame, state: &AppState, area: Rect) {
    let mount = state.get_selected_mount();
    let used_percent = if let Some(m) = mount {
        if m.total_space > 0 {
            (m.used_space as f64 / m.total_space as f64) * 100.0
        } else {
            0.0
        }
    } else {
        0.0
    };

    let bar_width = (area.width as f64 * used_percent / 100.0) as u16;
    let w = (area.width as usize).saturating_sub(30);
    let used_bar: String = "█".repeat(bar_width as usize).chars().take(w).collect();
    let free_bar: String = "░".repeat(w.saturating_sub(bar_width as usize)).chars().take(w).collect();

    let path_text = state.current_path.to_string_lossy();
    let status = if let Some(m) = mount {
        format!(
            " {} {}{} {} / {} {} ",
            path_text,
            used_bar,
            free_bar,
            format_size(m.used_space),
            format_size(m.total_space),
            if state.is_scanning { "[Scanning...]" } else { "" }
        )
    } else {
        format!(" {} ", path_text)
    };

    let paragraph = Paragraph::new(status)
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .block(Block::default().borders(Borders::BOTTOM));

    f.render_widget(paragraph, area);
}

fn draw_treemap(f: &mut Frame, state: &AppState, area: Rect) {
    let entry = match &state.current_entry {
        Some(e) => e,
        None => {
            let p = Paragraph::new("Press Enter to scan or / to select drive")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().title(" Treemap ").borders(Borders::ALL));
            f.render_widget(p, area);
            return;
        }
    };

    if state.treemap_cells.is_empty() {
        let p = Paragraph::new("No subdirectories found")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().title(" Treemap ").borders(Borders::ALL));
        f.render_widget(p, area);
        return;
    }

    for (i, cell) in state.treemap_cells.iter().enumerate() {
        let cell_rect = Rect::new(cell.x + 1, cell.y + 1, cell.x + cell.width, cell.y + cell.height);

        if cell_rect.width < 2 || cell_rect.height < 2 {
            continue;
        }

        let intersects = area.intersects(cell_rect);
        if !intersects {
            continue;
        }

        let color = COLOR_PALETTE[i % COLOR_PALETTE.len()];
        let is_selected = i == state.selected_cell_idx;

        let name = cell.path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| cell.path.to_string_lossy().to_string());

        let name_trunc: String = name.chars().take((cell_rect.width as usize).saturating_sub(2)).collect();

        let style = if is_selected {
            Style::default().fg(color).bg(color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(color).bg(Color::Black)
        };

        let p = Paragraph::new(format!("{}\n{}", name_trunc, format_size(cell.size)))
            .style(style)
            .block(Block::default().borders(if is_selected { Borders::ALL } else { Borders::NONE }));

        f.render_widget(p, cell_rect);
    }
}

fn draw_status_bar(f: &mut Frame, state: &AppState, area: Rect) {
    let (selected_name, selected_size, selected_pct) = if let Some(cell) = state.treemap_cells.get(state.selected_cell_idx) {
        (
            cell.path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| cell.path.to_string_lossy().to_string()),
            format_size(cell.size),
            if let Some(e) = &state.current_entry {
                if e.size > 0 {
                    format!("{}%", (cell.size as f64 / e.size as f64 * 100.0) as u32)
                } else {
                    "0%".to_string()
                }
            } else {
                "0%".to_string()
            },
        )
    } else {
        (String::new(), String::new(), String::new())
    };

    let status = format!(
        " {} │ {} │ {} │ ↑Enter drill │ /drive ",
        if selected_name.is_empty() { "-" } else { &selected_name },
        if selected_size.is_empty() { "-" } else { &selected_size },
        if selected_pct.is_empty() { "-" } else { &selected_pct }
    );

    let p = Paragraph::new(status)
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .block(Block::default().borders(Borders::TOP));

    f.render_widget(p, area);
}

fn draw_drive_selector(f: &mut Frame, state: &AppState, area: Rect) {
    let modal_width = 40u16;
    let modal_height = (state.mounts.len() as u16 + 4).min(20);

    let modal_x = (area.width - modal_width) / 2;
    let modal_y = (area.height - modal_height) / 2;

    let modal_rect = Rect::new(modal_x, modal_y, modal_x + modal_width, modal_y + modal_height);

    let clear = Clear;
    f.render_widget(clear, area);

    let border = Block::default()
        .borders(Borders::ALL)
        .title(" Select Drive ")
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    f.render_widget(border, modal_rect);

    let inner_rect = modal_rect.inner(Margin { horizontal: 1, vertical: 1 });

    let items: Vec<ListItem> = state.mounts.iter().enumerate().map(|(i, m)| {
        let label = format!(
            "{} {} ({})",
            if i == state.selected_mount { ">" } else { " " },
            m.mount_point.to_string_lossy(),
            format_size(m.total_space)
        );
        ListItem::new(label)
    }).collect();

    let list = List::new(items)
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    f.render_widget(list, inner_rect);

    let hint = Paragraph::new(" /filter  ↑↓select  Enter:scan  q:quit")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(hint, Rect::new(modal_x, modal_y + modal_height, modal_x + modal_width, modal_y + modal_height + 1));
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
                    Constraint::Length(1),
                    Constraint::Min(3),
                    Constraint::Length(1),
                ])
                .split(f.area());

            if !state.show_drive_selector {
                let area = f.area();
                state.build_treemap(area.width, area.height.saturating_sub(2));
            }

            draw_top_bar(f, &state, chunks[0]);
            draw_treemap(f, &state, chunks[1]);
            draw_status_bar(f, &state, chunks[2]);

            if state.show_drive_selector {
                draw_drive_selector(f, &state, f.area());
            }
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                if state.show_drive_selector {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('/') => state.toggle_drive_selector(),
                        KeyCode::Tab => state.toggle_drive_selector(),
                        KeyCode::Up => {
                            if state.selected_mount > 0 {
                                state.selected_mount -= 1;
                            }
                        }
                        KeyCode::Down => {
                            if state.selected_mount < state.mounts.len() - 1 {
                                state.selected_mount += 1;
                            }
                        }
                        KeyCode::Enter => {
                            state.is_scanning = true;
                            let scan_path = state.mounts[state.selected_mount].mount_point.clone();
                            let scanned = state.scanner.scan_dir(&scan_path, 2);
                            state.current_entry = Some(scanned);
                            state.root_path = scan_path.clone();
                            state.current_path = scan_path;
                            state.is_scanning = false;
                            state.show_drive_selector = false;
                        }
                        KeyCode::Esc => {
                            state.show_drive_selector = false;
                        }
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('/') => state.toggle_drive_selector(),
                        KeyCode::Tab => state.toggle_drive_selector(),
                        KeyCode::Left => {
                            if state.selected_cell_idx > 0 {
                                state.selected_cell_idx -= 1;
                            }
                        }
                        KeyCode::Right => {
                            if state.selected_cell_idx < state.treemap_cells.len().saturating_sub(1) {
                                state.selected_cell_idx += 1;
                            }
                        }
                        KeyCode::Up => {
                            let cols = (state.treemap_cells.len() as f64 / 4.0).ceil() as usize;
                            if state.selected_cell_idx >= cols {
                                state.selected_cell_idx -= cols;
                            }
                        }
                        KeyCode::Down => {
                            let cols = (state.treemap_cells.len() as f64 / 4.0).ceil() as usize;
                            if state.selected_cell_idx + cols < state.treemap_cells.len() {
                                state.selected_cell_idx += cols;
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(cell) = state.treemap_cells.get(state.selected_cell_idx) {
                                state.is_scanning = true;
                                let scanned = state.scanner.scan_dir(&cell.path, 2);
                                state.current_path = cell.path.clone();
                                state.current_entry = Some(scanned);
                                state.selected_cell_idx = 0;
                                state.is_scanning = false;
                            }
                        }
                        KeyCode::Backspace => {
                            if let Some(entry) = &state.current_entry {
                                if let Some(parent) = entry.path.parent() {
                                    if parent != state.root_path && !parent.to_string_lossy().is_empty() {
                                        state.current_path = parent.to_path_buf();
                                        state.is_scanning = true;
                                        let scanned = state.scanner.scan_dir(&parent.to_path_buf(), 2);
                                        state.current_entry = Some(scanned);
                                        state.selected_cell_idx = 0;
                                        state.is_scanning = false;
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
                        KeyCode::Esc => {
                            state.toggle_drive_selector();
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