use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{io, thread, time::Duration};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::Style,
    widgets::{Block, Borders, Paragraph, Widget},
    Frame, Terminal,
};

fn ui<B: Backend>(f: &mut Frame<B>, file_names: &[String]) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Percentage(10),
                Constraint::Percentage(80),
                Constraint::Percentage(10),
            ]
            .as_ref(),
        )
        .split(f.size());
    let block = Block::default().title("status bar").borders(Borders::ALL);
    f.render_widget(block, chunks[0]);

    let block = Block::default()
        .title(file_names[0].as_str())
        .borders(Borders::ALL);
    f.render_widget(block, chunks[1]);

    let block = Block::default().title("filters").borders(Borders::ALL);
    let para = Paragraph::new("filters go here")
        .style(Style::default())
        .block(block)
        .alignment(Alignment::Left);
    f.render_widget(para, chunks[2]);
}

use clap::{command, Parser};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// File names to open
    file_names: Vec<String>,
}

fn main() -> Result<(), io::Error> {
    let args = Args::parse();
    if args.file_names.is_empty() {
        eprintln!("Supply at least one file name");
        return Ok(());
    }

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.draw(|f| ui(f, &args.file_names))?;

    thread::sleep(Duration::from_millis(5000));

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
