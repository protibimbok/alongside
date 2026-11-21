mod app;
mod process;
mod ui;

use std::io::{self, Stdout};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use clap::Parser;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use app::{App, Msg};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[arg(short, long, value_delimiter = ',')]
    names: Vec<String>,

    #[arg(required = true)]
    commands: Vec<String>,
}

type Term = Terminal<CrosstermBackend<Stdout>>;

/// Minimum time between two redraws (about 60 fps).
const FRAME: Duration = Duration::from_millis(16);

fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut terminal = setup_terminal()?;
    let result = run(&mut terminal, cli);
    restore_terminal();
    // Stop the commands after leaving the TUI, so a slow shutdown doesn't
    // look like a hang.
    result?.shutdown();
    Ok(())
}

fn run(terminal: &mut Term, cli: Cli) -> Result<App> {
    let (tx, rx) = mpsc::channel();

    let input_tx = tx.clone();
    thread::spawn(move || {
        while let Ok(event) = event::read() {
            if input_tx.send(Msg::Input(event)).is_err() {
                break;
            }
        }
    });

    let size = terminal.size()?;
    let mut app = App::new(
        &cli.commands,
        &cli.names,
        ui::pane_size(size.width, size.height),
        tx,
    )?;

    let mut dirty = true;
    let mut last_draw: Option<Instant> = None;
    while !app.quit {
        let wait = last_draw.map_or(Duration::ZERO, |t| FRAME.saturating_sub(t.elapsed()));
        if dirty && wait.is_zero() {
            terminal.draw(|frame| ui::draw(frame, &app))?;
            last_draw = Some(Instant::now());
            dirty = false;
        }

        // Block until something happens; if a redraw is pending, only until
        // the next frame is due.
        let msg = if dirty {
            match rx.recv_timeout(wait.max(Duration::from_millis(1))) {
                Ok(msg) => msg,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        } else {
            match rx.recv() {
                Ok(msg) => msg,
                Err(_) => break,
            }
        };
        dirty |= app.handle(msg);
        // Drain whatever else is queued so bursts of output cost one redraw.
        for msg in rx.try_iter().take(1024) {
            dirty |= app.handle(msg);
        }
    }

    Ok(app)
}

fn setup_terminal() -> Result<Term> {
    // Restore the terminal before the default hook prints the panic message.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        default_hook(info);
    }));

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    Ok(Terminal::new(CrosstermBackend::new(io::stdout()))?)
}

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(
        io::stdout(),
        DisableMouseCapture,
        LeaveAlternateScreen,
        crossterm::cursor::Show
    );
}
