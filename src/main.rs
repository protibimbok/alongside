mod app;
mod pane;
mod process;
mod size;
mod ui;

use std::io::{self, Stdout};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
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
use ui::Theme;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Tab labels, comma-separated and in command order [default: the command text]
    #[arg(short, long, value_delimiter = ',', value_name = "NAME")]
    names: Vec<String>,

    /// Adds a top row showing this text and the working directory
    #[arg(short, long)]
    title: Option<String>,

    /// Commands to run, each one quoted as a single argument
    #[arg(required = true, value_name = "COMMAND")]
    commands: Vec<String>,
}

type Term = Terminal<CrosstermBackend<Stdout>>;

/// Minimum time between two redraws (about 60 fps).
const FRAME: Duration = Duration::from_millis(16);

fn main() -> Result<()> {
    let cli = Cli::parse();
    let (tx, rx) = mpsc::channel();

    let mut app = App::new(
        &cli.commands,
        &cli.names,
        cli.title,
        crossterm::terminal::size()?,
        tx.clone(),
    )?;

    // The TUI is left at the end of this block, before `app` is dropped and
    // stops the commands, so a slow shutdown doesn't look like a hang.
    {
        let mut tui = Tui::enter()?;
        run(&mut tui.terminal, &mut app, tx, &rx)
    }
}

fn run(
    terminal: &mut Term,
    app: &mut App,
    input_tx: Sender<Msg>,
    rx: &Receiver<Msg>,
) -> Result<()> {
    thread::spawn(move || {
        while let Ok(event) = event::read() {
            if input_tx.send(Msg::Input(event)).is_err() {
                break;
            }
        }
    });

    let theme = Theme::detect();
    let mut dirty = true;
    let mut last_draw: Option<Instant> = None;
    while !app.should_quit() {
        let wait = last_draw.map_or(Duration::ZERO, |t| FRAME.saturating_sub(t.elapsed()));
        if dirty && wait.is_zero() {
            terminal.draw(|frame| ui::draw(frame, app, theme))?;
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

    Ok(())
}

/// The terminal in TUI mode. Dropping it restores the terminal, whichever way
/// the program leaves the TUI.
struct Tui {
    terminal: Term,
}

impl Tui {
    fn enter() -> Result<Self> {
        // Restore the terminal before the default hook prints the panic message.
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore_terminal();
            default_hook(info);
        }));

        // Built first, so a failure half way through still restores the terminal.
        let tui = Self {
            terminal: Terminal::new(CrosstermBackend::new(io::stdout()))?,
        };
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
        Ok(tui)
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        restore_terminal();
    }
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
