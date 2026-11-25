//! One command and the terminal screen it draws on.

use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::app::{Id, Msg};
use crate::process::{self, Proc};
use crate::size::PaneSize;

/// Lines of scrollback kept per command.
const SCROLLBACK: usize = 10_000;

const STARTUP: Duration = Duration::from_secs(2);

#[derive(Clone, Copy)]
pub enum Status {
    Running,
    Exited(u32),
}

pub struct Pane {
    name: String,
    parser: vt100::Parser,
    status: Status,
    command: String,
    proc: Proc,
    /// The run this pane is showing; bumped on every restart.
    id: Id,
    started: Instant,
    /// Lines that have scrolled off the top of the screen.
    history: usize,
    unseen: bool,
}

impl Pane {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn command(&self) -> &str {
        &self.command
    }

    pub fn status(&self) -> Status {
        self.status
    }

    /// Time since the command was spawned, while it is still starting up.
    pub fn starting(&self) -> Option<Duration> {
        let uptime = self.started.elapsed();
        (matches!(self.status, Status::Running) && uptime < STARTUP).then_some(uptime)
    }

    pub fn history(&self) -> usize {
        self.history
    }

    pub fn has_unseen_output(&self) -> bool {
        self.unseen
    }

    /// The command's terminal screen, at the current scroll position.
    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }

    pub fn spawn(
        index: usize,
        name: String,
        command: String,
        size: PaneSize,
        tx: &Sender<Msg>,
    ) -> Result<Self> {
        let id = Id {
            pane: index,
            run: 0,
        };
        Ok(Self {
            name,
            parser: new_parser(size),
            status: Status::Running,
            proc: Proc::spawn(id, &command, size, tx.clone())?,
            command,
            id,
            started: Instant::now(),
            history: 0,
            unseen: false,
        })
    }

    /// Stops the command if needed and starts it again on a clean screen.
    pub fn restart(&mut self, size: PaneSize, tx: &Sender<Msg>) {
        process::stop_all([&mut self.proc]);
        self.id.run += 1;
        self.parser = new_parser(size);
        self.started = Instant::now();
        self.history = 0;
        self.unseen = false;
        match Proc::spawn(self.id, &self.command, size, tx.clone()) {
            Ok(proc) => {
                self.proc = proc;
                self.status = Status::Running;
            }
            Err(err) => {
                self.parser
                    .process(format!("concurtui: {err:#}\r\n").as_bytes());
                self.status = Status::Exited(1);
            }
        }
    }

    pub fn resize(&mut self, size: PaneSize) {
        self.parser.screen_mut().set_size(size.rows, size.cols);
        self.proc.resize(size);
        self.measure_history();
    }

    /// Whether `id` is the run this pane is showing, rather than an earlier
    /// run of a command that was restarted since.
    pub fn is_current(&self, id: Id) -> bool {
        self.id == id
    }

    /// Draws the command's output on its screen.
    pub fn process(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
        self.measure_history();
        self.unseen |= self.screen().scrollback() > 0;
    }

    pub fn clear(&mut self) {
        let screen = self.screen();
        let (rows, cols) = screen.size();
        // Keeps the modes the command set, such as application cursor keys.
        let modes = screen.input_mode_formatted();
        self.parser = new_parser(PaneSize { rows, cols });
        self.parser.process(&modes);
        self.history = 0;
        self.unseen = false;
    }

    pub fn set_exited(&mut self, code: u32) {
        self.status = Status::Exited(code);
    }

    pub fn scroll(&mut self, lines: isize) -> bool {
        let before = self.screen().scrollback();
        self.scroll_to(before.saturating_add_signed(lines));
        self.screen().scrollback() != before
    }

    /// Scrolls to `offset` lines above the live screen, or as far as there is output.
    pub fn scroll_to(&mut self, offset: usize) {
        let screen = self.parser.screen_mut();
        screen.set_scrollback(offset);
        self.unseen &= screen.scrollback() > 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_to(0);
    }

    /// Scrolls so that `line`, counted from the top of the output, is in the
    /// middle of the pane.
    pub fn show_line(&mut self, line: usize) {
        let half = usize::from(self.screen().size().0 / 2);
        self.scroll_to((self.history + half).saturating_sub(line));
    }

    pub fn top_line(&self) -> usize {
        self.history.saturating_sub(self.screen().scrollback())
    }

    /// All of the output as text, scrollback included, from the top.
    pub fn lines(&mut self) -> Vec<String> {
        let (rows, cols) = self.screen().size();
        let at = self.screen().scrollback();
        let mut lines = Vec::with_capacity(self.history + usize::from(rows));
        let mut offset = self.history;
        loop {
            self.parser.screen_mut().set_scrollback(offset);
            // The last page overlaps the one before it unless the history
            // is a whole number of pages.
            let seen = lines.len() - (self.history - offset);
            lines.extend(self.screen().rows(0, cols).skip(seen));
            if offset == 0 {
                break;
            }
            offset = offset.saturating_sub(usize::from(rows));
        }
        self.parser.screen_mut().set_scrollback(at);
        lines
    }

    /// vt100 doesn't expose how much scrollback it holds, but it does clamp
    /// the scroll position to it.
    fn measure_history(&mut self) {
        let screen = self.parser.screen_mut();
        let at = screen.scrollback();
        screen.set_scrollback(usize::MAX);
        self.history = screen.scrollback();
        screen.set_scrollback(at);
    }

    /// Sends bytes to the command, as if they were typed.
    pub fn write(&mut self, bytes: &[u8]) {
        self.proc.write(bytes);
    }
}

/// Stops the command of every pane that is still running.
pub fn stop_all(panes: &mut [Pane]) {
    process::stop_all(panes.iter_mut().map(|pane| &mut pane.proc));
}

fn new_parser(size: PaneSize) -> vt100::Parser {
    vt100::Parser::new(size.rows, size.cols, SCROLLBACK)
}
