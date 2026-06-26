mod app;
mod ui;

use crate::cli::TuiArgs;
use crate::db::open_db;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, stdout};
use std::path::PathBuf;
use std::time::Duration;

pub fn run(args: TuiArgs, db: Option<PathBuf>) -> Result<()> {
    let database = open_db(db.as_deref())?;
    let mut app = app::App::new(database, args.full_detail)?;

    enable_raw_mode()?;
    let mut stdout = stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut app::App,
) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                if app.note_mode {
                    handle_note_key(app, key.code, key.modifiers)?;
                    continue;
                }

                if app.search_mode {
                    handle_search_key(app, key.code)?;
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('k') => app.mark_selected("keep")?,
                    KeyCode::Char('d') => app.mark_selected("delete_candidate")?,
                    KeyCode::Char('u') => app.mark_selected("undecided")?,
                    KeyCode::Char('n') => app.toggle_note_mode()?,
                    KeyCode::Char('f') => app.cycle_filter()?,
                    KeyCode::Char('/') => app.start_search(),
                    KeyCode::Up => {
                        app.selected = app.selected.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        app.selected = (app.selected + 1).min(app.pairs.len().saturating_sub(1));
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn handle_note_key(
    app: &mut app::App,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Result<()> {
    match code {
        KeyCode::Enter => app.save_note()?,
        KeyCode::Esc => app.cancel_note(),
        KeyCode::Backspace => {
            app.note_buffer.pop();
        }
        KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.note_buffer.clear();
            app.save_note()?;
        }
        KeyCode::Char(c) => app.note_buffer.push(c),
        _ => {}
    }
    Ok(())
}

fn handle_search_key(app: &mut app::App, code: KeyCode) -> Result<()> {
    match code {
        KeyCode::Enter => app.apply_search()?,
        KeyCode::Esc => app.clear_search()?,
        KeyCode::Backspace => {
            app.search.pop();
            app.refresh_pairs()?;
        }
        KeyCode::Char(c) => {
            app.search.push(c);
            app.refresh_pairs()?;
        }
        _ => {}
    }
    Ok(())
}
