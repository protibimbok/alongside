//! One command and the terminal screen it draws on.

use std::sync::mpsc::Sender;

use anyhow::Result;

use crate::app::{Id, Msg};
use crate::process::{self, Proc};
use crate::size::PaneSize;

/// Lines of scrollback kept per command.
const SCROLLBACK: usize = 10_000;

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
}

impl Pane {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn status(&self) -> Status {
        self.status
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
        })
    }

    /// Stops the command if needed and starts it again on a clean screen.
    pub fn restart(&mut self, size: PaneSize, tx: &Sender<Msg>) {
        process::stop_all([&mut self.proc]);
        self.id.run += 1;
        self.parser = new_parser(size);
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
    }

    /// Whether `id` is the run this pane is showing, rather than an earlier
    /// run of a command that was restarted since.
    pub fn is_current(&self, id: Id) -> bool {
        self.id == id
    }

    /// Draws the command's output on its screen.
    pub fn process(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
    }

    pub fn set_exited(&mut self, code: u32) {
        self.status = Status::Exited(code);
    }

    pub fn scroll(&mut self, lines: isize) -> bool {
        let screen = self.parser.screen_mut();
        let before = screen.scrollback();
        screen.set_scrollback(before.saturating_add_signed(lines));
        screen.scrollback() != before
    }

    pub fn scroll_to_bottom(&mut self) {
        self.parser.screen_mut().set_scrollback(0);
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
