mod app_state;
mod error;
mod log;
mod mode;

use app_state::{AppMode, AppState};
use crossterm::event;
use error::Result;
use log::log_file::LogFile;
use num_format::ToFormattedString;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Wrap},
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
    let files: Vec<_> = args
        .file_names
        .iter()
        .map(|n| LogFile::new(n.as_str().into()))
        .collect();

    let mut state = AppState::new(&files);
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
    use Constraint::{Length, Min, Percentage};

    let vertical = Layout::vertical([
        Length(1),
        Min(3),
        Length(1),
        Length(state.state_bar_text_number_of_lines() as u16),
    ]);
    let [title_bar, main_area, status_area, command_bar] = vertical.areas(frame.area());

    let [left_status_area, right_status_area] =
        Layout::horizontal([Percentage(80), Percentage(20)]).areas(status_area);

    frame.render_widget(
        Line::styled(
            format!(
                " L{}",
                (state.get_line_offset() + 1).to_formatted_string(&num_format::Locale::en),
            ),
            (
                ratatui::style::Color::White,
                ratatui::style::Color::DarkGray,
            ),
        ),
        left_status_area,
    );

    frame.render_widget(
        Line::styled(
            "?:View Help",
            (
                ratatui::style::Color::White,
                ratatui::style::Color::DarkGray,
            ),
        ),
        right_status_area,
    );

    match state.mode() {
        AppMode::Command => {
            frame.render_widget(state.command_bar_text(), command_bar);
        }
        AppMode::FiltersMenu => {
            let (info_lines, table, mut table_state) = state.filters.filters_menu_text();

            let table_area = Rect::new(
                command_bar.x,
                command_bar.y + state.filters.filters_menu_info_lines_size() as u16,
                command_bar.width,
                command_bar.height - 1,
            );
            frame.render_widget(info_lines, command_bar);
            table.render(table_area, frame.buffer_mut(), &mut table_state);
        }
        AppMode::Logs => (),
    }

    let b = Block::new().borders(Borders::RIGHT);
    frame.render_widget(&b, main_area);
    let mut logs = Paragraph::new(Text::from_iter(state.lines_iter()));
    if state.wrapping() {
        logs = logs.wrap(Wrap { trim: false })
    }
    frame.render_widget(logs, b.inner(main_area));

    frame.render_widget(state.top_log_line_title_bar_text(), title_bar);
}
