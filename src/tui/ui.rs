use crate::tui::app::App;
use crate::tui::display::render_slice_bounds;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    render_list(frame, app, main_chunks[0]);
    render_detail(frame, app, main_chunks[1]);
    render_status_bar(frame, app, chunks[1]);
    render_help(frame, chunks[2]);
}

fn render_list(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let filter_label = app.filter.label();
    let block = Block::default()
        .title(format!(" Subset Pairs [{filter_label}] "))
        .borders(Borders::ALL);

    if app.rows.is_empty() {
        let empty = Paragraph::new("No pairs match current filter/search")
            .block(block)
            .wrap(Wrap { trim: true });
        frame.render_widget(empty, area);
        return;
    }

    let viewport_height = area.height.saturating_sub(2) as usize;
    let (start, count) = render_slice_bounds(
        app.selected,
        app.window_offset,
        app.rows.len(),
        viewport_height,
    );

    let items: Vec<ListItem> = app.rows[start..start + count]
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let global_idx = app.window_offset + start + i;
            let size = format_size(row.total_size);
            let prefix = if global_idx == app.selected {
                "▶ "
            } else {
                "  "
            };
            let line = Line::from(vec![
                Span::raw(prefix),
                Span::styled(
                    format!("{} ", row.subset_path),
                    if global_idx == app.selected {
                        Style::default().add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
                Span::styled(row.annotation_marker.clone(), Style::default().fg(Color::Yellow)),
                Span::raw(format!("  {size}")),
            ]);
            ListItem::new(line).into()
        })
        .collect();

    let mut state = ListState::default();
    let local_selected = app.selected.saturating_sub(app.window_offset);
    let list_selected = if local_selected >= start && local_selected < start + count {
        local_selected - start
    } else {
        0
    };
    state.select(Some(list_selected));

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Color::DarkGray));

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_detail(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let block = Block::default().title(" Detail ").borders(Borders::ALL);

    let content = if let (Some(row), Some(detail)) = (app.selected_row(), app.selection_detail.as_ref()) {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Subset:  ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(row.subset_path.clone()),
            ]),
            Line::from(vec![
                Span::styled("Superset:", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!(" {}", row.superset_path)),
            ]),
            Line::from(format!(
                "Files: {}  |  Size: {}",
                row.file_count,
                format_size(row.total_size)
            )),
        ];

        if detail.is_exact_duplicate {
            lines.push(Line::from(vec![Span::styled(
                "[exact duplicate]",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )]));
        }

        lines.extend([
            Line::from(""),
            Line::from(vec![
                Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(detail.status.clone()),
            ]),
        ]);

        if !detail.notes.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Notes:  ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(detail.notes.clone()),
            ]));
        }

        if app.note_mode {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Editing note: ", Style::default().fg(Color::Cyan)),
                Span::raw(app.note_buffer.clone()),
                Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)),
            ]));
        }

        lines
    } else {
        vec![Line::from("Select a pair to view details")]
    };

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn render_status_bar(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let mut parts = vec![app.status_message.clone()];

    if app.search_mode {
        parts.push(format!("Search: /{}", app.search));
    } else if !app.search.is_empty() {
        parts.push(format!("Filter: \"{}\"", app.search));
    }

    let showing_end = (app.window_offset + app.rows.len()).min(app.total_pairs);
    let showing_start = if app.total_pairs == 0 {
        0
    } else {
        app.window_offset + 1
    };
    parts.push(format!(
        "{} pairs ({}-{})",
        app.total_pairs, showing_start, showing_end
    ));

    let text = parts.join("  |  ");
    let paragraph = Paragraph::new(text).block(Block::default().borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}

fn render_help(frame: &mut Frame, area: ratatui::layout::Rect) {
    let help = "↑/↓ select  k keep  d delete  u undecided  n note  f filter  / search  q quit";
    let paragraph = Paragraph::new(help);
    frame.render_widget(paragraph, area);
}

fn format_size(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = KB * 1024;
    const GB: i64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}
