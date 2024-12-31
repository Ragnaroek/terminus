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
    style::palette::tailwind::SLATE,
    text::Span,
    widgets::{Axis, Block, Chart, Dataset, GraphType, List, ListState, Paragraph},
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

struct State {
    max: f64,
    data: Vec<(f64, f64)>,
}

struct App {
    state: State,
}

fn main() -> Result<(), String> {
    let args = Cli::parse();

    let trace_data = read_trace_file(&args.file)?;
    println!("num traces = {}", trace_data.len());

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
            state: State { data, max },
        }
    }

    fn run(&mut self, mut terminal: Terminal<impl Backend>) -> io::Result<()> {
        loop {
            self.draw(&mut terminal)?;
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        _ => {}
                    }
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
        let [frame_bar_area, bottom_area] =
            Layout::vertical([Constraint::Percentage(30), Constraint::Percentage(70)]).areas(area);

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
    }
}
