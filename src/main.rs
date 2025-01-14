mod app_state;
mod command;
mod error;
mod log_line;

use app_state::AppState;
use crossterm::event;
use error::Result;
use ratatui::{
    layout::{Constraint, Layout, Margin},
    text::Text,
    widgets::Block,
    DefaultTerminal, Frame,
};

use clap::{command, Parser};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// File names to open
    file_names: Vec<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.file_names.is_empty() {
        eprintln!("Supply at least one file name");
        return Ok(());
    }
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = run(terminal, args);
    ratatui::restore();
    result
}

fn run(mut terminal: DefaultTerminal, args: Args) -> Result<()> {
    let first_file_name = args
        .file_names
        .first()
        .expect("First file name must exist")
        .as_str();

    let text = String::from_utf8(std::fs::read(first_file_name).expect("Can read file"))
        .expect("File is a valid utf-8 text file");

    let mut state = AppState::new(&text, first_file_name);
    loop {
        terminal.draw(|f| render(f, &mut state))?;
        let event = event::read()?;
        match state.read_event(event) {
            app_state::AppAction::EndApp => break Ok(()),
            app_state::AppAction::NoAction => continue,
        }
    }
}

fn render(frame: &mut Frame, state: &mut AppState) {
    use Constraint::{Length, Min};

    let vertical = Layout::vertical([
        Min(3),
        Length(2 + state.state_bar_text_number_of_lines() as u16),
    ]);
    let [main_area, status_area] = vertical.areas(frame.area());

    frame.render_widget(
        Block::bordered().title("Status Bar").title_bottom(format!(
            "{} filters are currently active",
            state.total_filters_enabled()
        )),
        status_area,
    );

    let mut status_bar_styled = state.status_bar_text();
    status_bar_styled.push_line(state.status_bar_completions());

    frame.render_widget(status_bar_styled, status_area.inner(Margin::new(1, 1)));
    frame.render_widget(Block::bordered().title(state.main_area_title()), main_area);
    frame.render_widget(
        Text::from_iter(state.lines_iter()),
        main_area.inner(Margin::new(1, 1)),
    );
}
