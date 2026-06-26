use crate::tui::app::App;
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

    if app.pairs.is_empty() {
        let empty = Paragraph::new("No pairs match current filter/search")
            .block(block)
            .wrap(Wrap { trim: true });
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = app
        .pairs
        .iter()
        .enumerate()
        .map(|(idx, pair)| {
            let marker = app.subset_annotation_marker(&pair.subset_path);
            let size = format_size(pair.total_size);
            let prefix = if idx == app.selected { "▶ " } else { "  " };
            let line = Line::from(vec![
                Span::raw(prefix),
                Span::styled(
                    format!("{} ", pair.subset_path),
                    if idx == app.selected {
                        Style::default().add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
                Span::styled(marker, Style::default().fg(Color::Yellow)),
                Span::raw(format!("  {size}")),
            ]);
            ListItem::new(line).into()
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.selected));

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Color::DarkGray));

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_detail(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let block = Block::default().title(" Detail ").borders(Borders::ALL);

    let content = if let Some(pair) = app.selected_pair() {
        let (status, notes) = app
            .annotation_for_selected()
            .ok()
            .flatten()
            .map(|a| (a.status, a.notes))
            .unwrap_or_else(|| ("unreviewed".to_string(), String::new()));

        let mut lines = vec![
            Line::from(vec![
                Span::styled("Subset:  ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(pair.subset_path.clone()),
            ]),
            Line::from(vec![
                Span::styled("Superset:", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!(" {}", pair.superset_path)),
            ]),
            Line::from(format!(
                "Files: {}  |  Size: {}",
                pair.file_count,
                format_size(pair.total_size)
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(status.clone()),
            ]),
        ];

        if !notes.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Notes:  ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(notes),
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

    parts.push(format!("{} pairs", app.pairs.len()));

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
