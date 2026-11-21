//! Application state and input handling.

use std::sync::mpsc::Sender;

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::process::Proc;
use crate::ui;

/// Lines of scrollback kept per command.
const SCROLLBACK: usize = 10_000;
const MAX_NAME: usize = 20;

/// Identifies one run of a command: its pane, and how often it was restarted.
/// Events from an earlier run of a restarted command are ignored.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Id {
    pub pane: usize,
    pub run: u64,
}

pub enum Msg {
    Input(Event),
    Output(Id, Vec<u8>), // (index, content)
    Exited(Id, u32),     // (index, code)
}

pub enum Status {
    Running,
    Exited(u32),
}

pub struct Pane {
    pub name: String,
    pub parser: vt100::Parser,
    pub status: Status,
    command: String,
    proc: Proc,
    run: u64,
}

pub struct App {
    pub panes: Vec<Pane>,
    pub active: usize,
    pub quit: bool,
    size: (u16, u16),
    tx: Sender<Msg>,
}

impl App {
    pub fn new(
        commands: &[String],
        names: &[String],
        size: (u16, u16),
        tx: Sender<Msg>,
    ) -> Result<Self> {
        let panes = commands
            .iter()
            .enumerate()
            .map(|(i, command)| {
                let name = match names.get(i).map(|n| n.trim()) {
                    Some(name) if !name.is_empty() => name.to_string(),
                    _ => truncate(command, MAX_NAME),
                };
                let id = Id { pane: i, run: 0 };
                Ok(Pane {
                    name,
                    parser: vt100::Parser::new(size.0, size.1, SCROLLBACK),
                    status: Status::Running,
                    command: command.clone(),
                    proc: Proc::spawn(id, command, size, tx.clone())?,
                    run: 0,
                })
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            panes,
            active: 0,
            quit: false,
            size,
            tx,
        })
    }

    /// Applies a message and returns whether the screen needs a redraw.
    pub fn handle(&mut self, msg: Msg) -> bool {
        match msg {
            Msg::Output(id, bytes) if self.is_current(id) => {
                self.panes[id.pane].parser.process(&bytes);
                id.pane == self.active
            }
            Msg::Exited(id, code) if self.is_current(id) => {
                self.panes[id.pane].status = Status::Exited(code);
                if self
                    .panes
                    .iter()
                    .all(|p| matches!(p.status, Status::Exited(_)))
                {
                    self.quit = true;
                }
                true
            }
            Msg::Output(..) | Msg::Exited(..) => false,
            // Windows (and Unix with keyboard enhancement enabled) reports
            // a Release event after every Press. Handling it would run every
            // key twice; Repeat is kept so held keys keep repeating.
            Msg::Input(Event::Key(key)) if key.kind != KeyEventKind::Release => self.on_key(key),
            Msg::Input(Event::Resize(cols, rows)) => {
                self.resize(ui::pane_size(cols, rows));
                true
            }
            Msg::Input(_) => false,
        }
    }

    fn on_key(&mut self, key: KeyEvent) -> bool {
        let count = self.panes.len();
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.quit = true,
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char('r') => self.restart(),
            KeyCode::Tab => self.active = (self.active + 1) % count,
            KeyCode::BackTab => self.active = (self.active + count - 1) % count,
            KeyCode::Char(c @ '1'..='9') => {
                let i = c as usize - '1' as usize;
                if i < count {
                    self.active = i;
                }
            }
            _ => return false,
        }
        true
    }

    fn is_current(&self, id: Id) -> bool {
        self.panes[id.pane].run == id.run
    }

    /// Stops the active command if needed and starts it again on a clean screen.
    fn restart(&mut self) {
        let pane = &mut self.panes[self.active];
        if let Status::Running = pane.status {
            pane.proc.kill();
        }
        let id = Id {
            pane: self.active,
            run: pane.run + 1,
        };
        let (rows, cols) = self.size;
        pane.run = id.run;
        pane.parser = vt100::Parser::new(rows, cols, SCROLLBACK);
        match Proc::spawn(id, &pane.command, self.size, self.tx.clone()) {
            Ok(proc) => {
                pane.proc = proc;
                pane.status = Status::Running;
            }
            Err(err) => {
                pane.parser
                    .process(format!("concurtui: {err:#}\r\n").as_bytes());
                pane.status = Status::Exited(1);
            }
        }
    }

    fn resize(&mut self, size: (u16, u16)) {
        self.size = size;
        for pane in &mut self.panes {
            pane.parser.screen_mut().set_size(size.0, size.1);
            pane.proc.resize(size);
        }
    }

    /// Stops every command that is still running.
    pub fn shutdown(&mut self) {
        for pane in &mut self.panes {
            if let Status::Running = pane.status {
                pane.proc.kill();
            }
        }
    }
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        let mut short: String = text.chars().take(max - 1).collect();
        short.push('…');
        short
    }
}
