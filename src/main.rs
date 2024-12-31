#![feature(duration_millis_float)]

mod trace;

use clap::Parser;
use ratatui::{
    crossterm::{
        ExecutableCommand,
        event::{self, Event, KeyCode, KeyEventKind},
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    prelude::*,
    widgets::{Axis, Block, Chart, Dataset, GraphType, Paragraph},
};
use ratatui::{
    style::{Style, Stylize},
    symbols,
};
use std::io::{self, stdout};
use trace::{Trace, read_trace_file};

#[derive(Parser)]
struct Cli {
    file: std::path::PathBuf,
}

enum InputMode {
    Normal,
    Editing,
}

struct State {
    max: f64,
    data: Vec<(f64, f64)>,

    input: String,
    input_mode: InputMode,
    character_index: usize,
}

struct App {
    state: State,
}

fn main() -> Result<(), String> {
    let args = Cli::parse();

    let trace_data = read_trace_file(&args.file)?;

    enable_raw_mode().map_err(|e| e.to_string())?;
    stdout()
        .execute(EnterAlternateScreen)
        .map_err(|e| e.to_string())?;
    let terminal = Terminal::new(CrosstermBackend::new(stdout())).map_err(|e| e.to_string())?;

    App::new(trace_data)
        .run(terminal)
        .map_err(|e| e.to_string())?;

    disable_raw_mode().map_err(|e| e.to_string())?;
    stdout()
        .execute(LeaveAlternateScreen)
        .map_err(|e| e.to_string())?;
    Ok(())
}

impl App {
    fn new(trace_data: Vec<Trace>) -> App {
        let mut data = Vec::with_capacity(trace_data.len());
        let mut max: f64 = 0.0;
        for trace in &trace_data {
            let duration = trace.fields.time_busy + trace.fields.time_idle;
            let millis = duration.as_millis_f64();
            max = max.max(millis);
            data.push((trace.span.id.unwrap() as f64, millis.log10()));
        }

        App {
            state: State {
                data,
                max,
                input: String::new(),
                input_mode: InputMode::Normal,
                character_index: 0,
            },
        }
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.state.character_index.saturating_sub(1);
        self.state.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.state.character_index.saturating_add(1);
        self.state.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.state.input.chars().count())
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.state.character_index != 0;
        if is_not_cursor_leftmost {
            let current_index = self.state.character_index;
            let from_left_to_current_index = current_index - 1;

            let before_char_to_delete = self.state.input.chars().take(from_left_to_current_index);
            let after_char_to_delete = self.state.input.chars().skip(current_index);

            self.state.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn byte_index(&self) -> usize {
        self.state
            .input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.state.character_index)
            .unwrap_or(self.state.input.len())
    }

    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.state.input.insert(index, new_char);
        self.move_cursor_right();
    }

    fn submit_command(&mut self) -> bool {
        if self.state.input == ":q" {
            return true;
        }

        self.state.input.clear();
        self.state.character_index = 0;

        false
    }

    fn run(&mut self, mut terminal: Terminal<impl Backend>) -> io::Result<()> {
        loop {
            self.draw(&mut terminal)?;
            if let Event::Key(key) = event::read()? {
                match self.state.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char(':') => {
                            self.enter_char(':');
                            self.state.input_mode = InputMode::Editing;
                        }
                        _ => {}
                    },
                    InputMode::Editing if key.kind == KeyEventKind::Press => match key.code {
                        KeyCode::Enter => {
                            if self.submit_command() {
                                return Ok(());
                            }
                        }
                        KeyCode::Char(to_insert) => self.enter_char(to_insert),
                        KeyCode::Backspace => self.delete_char(),
                        KeyCode::Left => self.move_cursor_left(),
                        KeyCode::Right => self.move_cursor_right(),
                        KeyCode::Esc => self.state.input_mode = InputMode::Normal,
                        _ => {}
                    },
                    InputMode::Editing => {}
                }
            }
        }
    }

    fn draw(&mut self, terminal: &mut Terminal<impl Backend>) -> io::Result<()> {
        terminal.draw(|frame| frame.render_widget(self, frame.area()))?;
        Ok(())
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [frame_bar_area, detail_area, cmd_area] = Layout::vertical([
            Constraint::Percentage(30),
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .areas(area);

        // Create the datasets to fill the chart with
        let datasets = vec![
            // Line chart
            Dataset::default()
                //.name("frame duration")
                .marker(symbols::Marker::HalfBlock)
                .graph_type(GraphType::Bar)
                .style(Style::default().magenta())
                .data(&self.state.data),
        ];

        let len_str = self.state.data.len().to_string();
        // Create the X axis and define its properties
        let x_axis = Axis::default()
            .title("frame".red())
            .style(Style::default().white())
            .bounds([0.0, self.state.data.len() as f64])
            .labels(["0.0", &len_str]);

        let max_str = self.state.max.ceil().to_string();

        // Create the Y axis and define its properties
        let y_axis = Axis::default()
            .title("ms (log scale)".red())
            .style(Style::default().white())
            .bounds([0.0, self.state.max.log10()])
            .labels(["0.0", &max_str]);

        // Create the chart and link all the parts together
        let chart = Chart::new(datasets)
            .block(Block::new().title("Chart"))
            .x_axis(x_axis)
            .y_axis(y_axis)
            .render(frame_bar_area, buf);

        Paragraph::new("TODO Detail view")
            .block(Block::bordered().title("Frame Detail"))
            .render(detail_area, buf);

        Paragraph::new(self.state.input.as_str())
            .style(match self.state.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::Editing => Style::default().fg(Color::Yellow),
            })
            .render(cmd_area, buf);
    }
}
