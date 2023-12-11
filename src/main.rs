use std::{io, time};

use chrono::{DateTime, Local};
use clap::Parser;
use crossterm::{event, terminal};
use notify_rust::Notification;
use ratatui::{prelude::*, widgets::*};

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    initialize(&mut terminal)?;
    let app_result = run_timer(cli, &mut terminal);
    exit(&mut terminal)?;
    if let Err(e) = app_result {
        eprintln!("{}", e);
    }
    Ok(())
}

enum Error {
    UnknownUnit(String),
    Draw(String),
    Terminal(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::UnknownUnit(unit) => write!(f, "[Cli] {}", unit),
            Error::Draw(msg) => write!(f, "[Draw] {}", msg),
            Error::Terminal(msg) => write!(f, "[Terminal] {}", msg),
        }
    }
}

type AppError = Result<(), Error>;

#[derive(Parser)]
#[command(version, about)]
pub struct Cli {
    #[clap(help = "The duration of the timer. You can use the following formats:
    - h (hours), 
    - m (minutes)
    - s (seconds)
    - ms (milliseconds).
    
    If no unit is provided, seconds will be used.
    Examples:
    - timer 50 -> Runs a timer for 50 seconds (default).
    - timer 45m -> Runs a timer for 45 minutes.
    - timer 1h30m -> Runs a timer for 1 hour and 30 minutes.")]
    duration: String,
    #[arg(short, long, help = "A name for the timer.")]
    name: Option<String>,
    #[arg(
        long, help = "Send a notification when the timer begins and ends.", 
        default_value = "true", action = clap::ArgAction::Set,
        require_equals(true)
    )]
    notify: bool,
    #[arg(
        long, short, help = "The format of the timer. You can use the following formats:
        - 24h (24 hour format) (e.g. 23:59:59)
        - 12h (12 hour format) (e.g. 11:59:59 PM)",
        default_value = "24h",
        value_parser = clap::builder::PossibleValuesParser::new(&["24h", "12h"])
    )]
    format: String,
}

fn initialize<B: Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
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

fn notify(message: &str)  {
    let sound = match std::env::consts::OS {
        "macos" => "Boop",
        // TODO: see what works best for windows
        // Assume an XDG-compliant os
        _ => "message-new-email",
    };
    // ignore errors
    let _ = Notification::new()
        .summary("Timer")
        .body(message)
        .icon("terminal")
        .sound_name(sound)
        .show();
}
fn run_timer<B: Backend>(cli: Cli, terminal: &mut Terminal<B>) -> AppError {
    let name = match cli.name.clone() {
        Some(name) => name,
        None => "Timer".to_owned(),
    };
   
    let start_time = time::Instant::now();
    let timer_started_at = chrono::Local::now();
    let duration = match cli.duration.parse::<u64>() {
        Ok(duration) => time::Duration::from_secs(duration),
        Err(_) => match humantime::parse_duration(&cli.duration) {
            Ok(duration) => duration,
            Err(e) => return Err(Error::UnknownUnit(e.to_string())),
        },
    };
    
    // If Cli was parsed correctly, notify the user that the timer has started
    if cli.notify {
        notify(&format!("{} has started.", name));
    }

    // parse the duration from the cli
    while start_time.elapsed() < duration {
        let event_available = event::poll(time::Duration::from_millis(20))
            .map_err(|_| Error::Terminal("Unable to poll for events".to_owned()))?;
        if event_available {
            let event =
                event::read().map_err(|_| Error::Terminal("Unable to read events".to_owned()))?;

            if let event::Event::Key(event::KeyEvent { code, .. }) = event {
                match code {
                    event::KeyCode::Esc | event::KeyCode::Char('q') | event::KeyCode::Char('Q') => {
                        return Ok(());
                    }
                    _ => (),
                }
            }
        }
        terminal
            .draw(|f| {
                let elapsed_time = start_time.elapsed();
                let percent = (elapsed_time.as_secs_f32() / duration.as_secs_f32() * 100.0) as u16;

                if elapsed_time > duration {
                    return;
                }
                let time_left = duration - elapsed_time;
                draw_timer(f, percent, time_left, timer_started_at, &cli);
            })
            .map_err(|_| Error::Draw("Something went very wrong.".to_owned()))?;
    }
    if cli.notify {
        notify(&format!("{} is over!", name));
    }
    Ok(())
}

pub fn draw_timer(
    frame: &mut Frame<'_>,
    percent: u16,
    time_left: time::Duration,
    started_at: DateTime<Local>,
    cli: &Cli,
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
        percent,
        time_left,
        start: started_at,
        name: cli.name.clone(),
        format_12h: cli.format == "12h",
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
    terminal.show_cursor()?;
    reset()?;
    Ok(())
}

pub struct Timer {
    gradient: Vec<Color>,
    percent: u16,
    time_left: time::Duration,
    start: DateTime<Local>,
    name: Option<String>,
    format_12h: bool,
}

impl Widget for Timer {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let title = match self.name {
            Some(name) => name,
            None => "Timer".to_owned(),
        };
        let style: Style = Style::default();
        let time_left = if self.time_left.as_secs_f32() < 1.0 {
            format!("{:.2}s", self.time_left.as_secs_f32())
        } else {
            format!(
                "{:02}h:{:02}m:{:02}s",
                self.time_left.as_secs() / 3600,
                self.time_left.as_secs() / 60 % 60,
                self.time_left.as_secs() % 60
            )
        };
        let format = if self.format_12h { "%r" } else { "%T" };
        let started_at = format!("Started at: {}", self.start.format(format));
        buf.set_string(0, 1, started_at, style);
        buf.set_string(0, 2, format!("Time left: {}", time_left), style);

        Block::default()
            .title(format!("{}\n", title))
            .style(style.bold())
            .render(area, buf);

        let offset_y = 3;
        let percent = format!("{}%", self.percent);
        buf.set_string(
            area.width - percent.len() as u16,
            offset_y,
            percent,
            style,
        );

        for i in 0..area.width {
            buf.get_mut(i, offset_y).set_bg(Color::Rgb(45, 45, 45));
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
