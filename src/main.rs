mod app_state;
mod log_line;

use app_state::AppState;
use color_eyre::Result;
use crossterm::event;
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
    use Constraint::{Fill, Length, Min};

    let vertical = Layout::vertical([Min(3), Length(3)]);
    let [main_area, status_area] = vertical.areas(frame.area());
    let horizontal = Layout::horizontal([Fill(1); 1]);
    let [left_area] = horizontal.areas(main_area);

    frame.render_widget(
        Block::bordered().title("Status Bar").title_bottom(format!(
            "{} filters are currently active",
            state.total_filters_enabled()
        )),
        status_area,
    );
    frame.render_widget(
        Text::styled(
            state.status_bar_text(),
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::Rgb(255, 255, 255))
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        status_area.inner(Margin::new(1, 1)),
    );
    frame.render_widget(Block::bordered().title(state.main_area_title()), left_area);
    frame.render_widget(
        Text::from_iter(state.lines_iter()),
        left_area.inner(Margin::new(1, 1)),
    );
}
