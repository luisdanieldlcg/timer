use std::{io, time};

use chrono::{DateTime, Local};
use clap::Parser;
use crossterm::{event, terminal};
use ratatui::{prelude::*, widgets::*};

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    initialize(&mut terminal)?;
    run_timer(cli, &mut terminal)?;
    exit(&mut terminal)?;
    Ok(())
}

#[derive(Parser)]
#[command(version, about)]
pub struct Cli {
    #[clap(help = "The duration of the timer. You can use the following formats:
    - h (hours)
    - m (minutes)
    - s (seconds)
    - ms (milliseconds).
    
    If no unit is provided, seconds will be used.
    Examples:
    - timer 45m -> Runs a timer for 45 minutes.
    - timer 1h30m -> Runs a timer for 1 hour and 30 minutes.")]
    duration: String,
}

pub fn initialize<B: Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
    terminal::enable_raw_mode()?;
    crossterm::execute!(io::stdout(), terminal::EnterAlternateScreen)?;
    let panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        reset().expect("Failed to reset terminal");
        panic_hook(panic);
    }));
    terminal.hide_cursor()?;
    terminal.clear()?;
    Ok(())
}

pub fn run_timer<B: Backend>(cli: Cli, terminal: &mut Terminal<B>) -> io::Result<()> {
    let start_time = time::Instant::now();
    let timer_started_at = chrono::Local::now();
    let duration = match humantime::parse_duration(&cli.duration) {
        Ok(duration) => duration,
        Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidInput, e)),
    };

    // parse the duration from the cli
    while start_time.elapsed() < duration {
        if event::poll(time::Duration::from_millis(100)).expect("Failed to poll events") {
            if let event::Event::Key(event::KeyEvent { code, .. }) =
                event::read().expect("Unable to read event")
            {
                match code {
                    event::KeyCode::Esc | event::KeyCode::Char('q') | event::KeyCode::Char('Q') => {
                        return Ok(());
                    }
                    _ => (),
                }
            }
        }
        terminal.draw(|frame: &mut Frame<'_>| {
            let elapsed_time = start_time.elapsed();
            let percent = (elapsed_time.as_secs_f32() / duration.as_secs_f32() * 100.0) as u16;

            if elapsed_time > duration {
                return;
            }

            draw_timer(frame, percent, duration - elapsed_time, timer_started_at);
        })?;
    }
    Ok(())
}

pub fn draw_timer(
    frame: &mut Frame<'_>,
    percent: u16,
    time_left: time::Duration,
    started_at: DateTime<Local>,
) {
    let steps = 100;
    let base = (102, 63, 242);
    let end = (245, 65, 204);
    let mut gradient = Vec::with_capacity(steps as usize);

    for i in 0..steps {
        let r = base.0 as f32 + (end.0 as f32 - base.0 as f32) * (i as f32 / steps as f32);
        let g = base.1 as f32 + (end.1 as f32 - base.1 as f32) * (i as f32 / steps as f32);
        let b = base.2 as f32 + (end.2 as f32 - base.2 as f32) * (i as f32 / steps as f32);
        gradient.push(Color::Rgb(r as u8, g as u8, b as u8));
    }

    let timer = Timer {
        gradient,
        background: Color::Rgb(45, 45, 45),
        percent,
        time_left,
        started_at,
    };

    let render_area = Rect::new(0, 0, frame.size().width, 1);
    frame.render_widget(timer, render_area);
}

pub fn reset() -> io::Result<()> {
    terminal::disable_raw_mode()?;
    crossterm::execute!(io::stdout(), terminal::LeaveAlternateScreen)?;
    Ok(())
}

pub fn exit<B: Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
    reset()?;
    terminal.show_cursor()?;
    Ok(())
}

pub struct Timer {
    gradient: Vec<Color>,
    background: Color,
    percent: u16,
    time_left: time::Duration,
    started_at: DateTime<Local>,
}

impl Widget for Timer {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let title = format!(
            "Start Time: {} - Time Left: {:02}h:{:02}m:{:02}s",
            self.started_at.format("%r"),
            self.time_left.as_secs() / 3600,
            self.time_left.as_secs() / 60 % 60,
            self.time_left.as_secs() % 60
        );

        Block::default()
            .title(title)
            .style(Style::default().bg(Color::Reset))
            .render(area, buf);

        let offset_y = 1; // leave the first line for the title

        for i in 0..area.width {
            buf.get_mut(i, offset_y).set_bg(self.background);
        }
        // calculate how many cells to draw, based on the percentage
        let cells_to_draw = (self.percent as f32 / 100.0 * area.width as f32) as u16;
        for i in 0..cells_to_draw {
            let color =
                self.gradient[(i as f32 / area.width as f32 * self.gradient.len() as f32) as usize];
            buf.get_mut(i, offset_y).set_bg(color);
        }
    }
}
