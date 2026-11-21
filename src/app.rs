//! Application state and input handling.

use std::sync::mpsc::Sender;

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::process::Proc;
use crate::ui;

/// Lines of scrollback kept per command.
const SCROLLBACK: usize = 10_000;
const MAX_NAME: usize = 20;

pub enum Msg {
    Input(Event),
    Output(usize, Vec<u8>), // (index, content)
    Exited(usize, u32),     // (index, code)
}

pub enum Status {
    Running,
    Exited(u32),
}

pub struct Pane {
    pub name: String,
    pub parser: vt100::Parser,
    pub status: Status,
    proc: Proc,
}

pub struct App {
    pub panes: Vec<Pane>,
    pub active: usize,
    pub quit: bool,
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
                Ok(Pane {
                    name,
                    parser: vt100::Parser::new(size.0, size.1, SCROLLBACK),
                    status: Status::Running,
                    proc: Proc::spawn(i, command, size, tx.clone())?,
                })
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            panes,
            active: 0,
            quit: false,
        })
    }

    /// Applies a message and returns whether the screen needs a redraw.
    pub fn handle(&mut self, msg: Msg) -> bool {
        match msg {
            Msg::Output(i, bytes) => {
                self.panes[i].parser.process(&bytes);
                i == self.active
            }
            Msg::Exited(i, code) => {
                self.panes[i].status = Status::Exited(code);
                if self.panes.iter().all(|p| matches!(p.status, Status::Exited(_))) {
                    self.quit = true;
                }
                true
            }
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

    fn resize(&mut self, size: (u16, u16)) {
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
