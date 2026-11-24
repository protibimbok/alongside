//! Application state and input handling.

use std::sync::mpsc::Sender;

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind};

use crate::process::{self, Proc};
use crate::size::PaneSize;
use crate::ui;

/// Lines of scrollback kept per command.
const SCROLLBACK: usize = 10_000;
/// Lines scrolled per mouse wheel step.
const WHEEL_LINES: isize = 3;
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
    /// Keys go to the active command instead of being shortcuts.
    pub input_mode: bool,
    /// Ctrl+A was pressed in input mode; the next key is sent literally.
    pub send_to_main: bool,
    size: PaneSize,
    tx: Sender<Msg>,
}

impl App {
    pub fn new(
        commands: &[String],
        names: &[String],
        size: PaneSize,
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
                    parser: vt100::Parser::new(size.rows, size.cols, SCROLLBACK),
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
            input_mode: false,
            send_to_main: false,
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
            Msg::Input(Event::Mouse(mouse)) => match mouse.kind {
                MouseEventKind::ScrollUp => self.scroll(WHEEL_LINES),
                MouseEventKind::ScrollDown => self.scroll(-WHEEL_LINES),
                _ => false,
            },
            Msg::Input(Event::Resize(cols, rows)) => {
                self.resize(ui::pane_size(cols, rows));
                true
            }
            Msg::Input(_) => false,
        }
    }

    fn on_key(&mut self, key: KeyEvent) -> bool {
        if self.input_mode {
            return self.on_input_key(key);
        }
        let count = self.panes.len();
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.quit = true,
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char('r') => self.restart(),
            KeyCode::Char('i') => self.input_mode = true,
            KeyCode::PageUp => return self.scroll(self.size.rows as isize),
            KeyCode::PageDown => return self.scroll(-(self.size.rows as isize)),
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

    /// Input mode: `Esc` leaves it, `Ctrl+A` then `Esc` sends a literal `Esc`
    /// (and `Ctrl+A` twice a literal `Ctrl+A`). Everything else is forwarded.
    fn on_input_key(&mut self, key: KeyEvent) -> bool {
        let ctrl_a =
            key.code == KeyCode::Char('a') && key.modifiers.contains(KeyModifiers::CONTROL);
        if std::mem::take(&mut self.send_to_main) {
            if key.code != KeyCode::Esc && !ctrl_a {
                self.panes[self.active].proc.write(&[0x01]);
            }
        } else if key.code == KeyCode::Esc {
            self.input_mode = false;
            return true;
        } else if ctrl_a {
            self.send_to_main = true;
            return true;
        }

        let pane = &mut self.panes[self.active];
        if let Some(bytes) = encode_key(key, pane.parser.screen().application_cursor()) {
            pane.proc.write(&bytes);
        }
        // Typing jumps back to the live screen.
        pane.parser.screen_mut().set_scrollback(0);
        true
    }

    fn scroll(&mut self, lines: isize) -> bool {
        let screen = self.panes[self.active].parser.screen_mut();
        let before = screen.scrollback();
        screen.set_scrollback(before.saturating_add_signed(lines));
        screen.scrollback() != before
    }

    fn is_current(&self, id: Id) -> bool {
        self.panes[id.pane].run == id.run
    }

    /// Stops the active command if needed and starts it again on a clean screen.
    fn restart(&mut self) {
        let pane = &mut self.panes[self.active];
        process::stop_all([&mut pane.proc]);
        let id = Id {
            pane: self.active,
            run: pane.run + 1,
        };
        pane.run = id.run;
        pane.parser = vt100::Parser::new(self.size.rows, self.size.cols, SCROLLBACK);
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

    fn resize(&mut self, size: PaneSize) {
        self.size = size;
        for pane in &mut self.panes {
            pane.parser.screen_mut().set_size(size.rows, size.cols);
            pane.proc.resize(size);
        }
    }

    /// Stops every command that is still running.
    pub fn shutdown(&mut self) {
        process::stop_all(self.panes.iter_mut().map(|pane| &mut pane.proc));
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

/// Encodes a key press as the bytes a terminal would send for it.
fn encode_key(key: KeyEvent, application_cursor: bool) -> Option<Vec<u8>> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let cursor = |c: char| {
        let prefix = if application_cursor { "\x1bO" } else { "\x1b[" };
        format!("{prefix}{c}").into_bytes()
    };
    let mut bytes = match key.code {
        KeyCode::Char(c) if ctrl => match c.to_ascii_lowercase() {
            c @ 'a'..='z' => vec![c as u8 - b'a' + 1],
            ' ' | '@' | '2' => vec![0x00],
            '[' | '3' => vec![0x1b],
            '\\' | '4' => vec![0x1c],
            ']' | '5' => vec![0x1d],
            '^' | '6' => vec![0x1e],
            '_' | '-' | '7' => vec![0x1f],
            _ => return None,
        },
        KeyCode::Char(c) => c.to_string().into_bytes(),
        KeyCode::Enter => b"\r".to_vec(),
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Tab => b"\t".to_vec(),
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => cursor('A'),
        KeyCode::Down => cursor('B'),
        KeyCode::Right => cursor('C'),
        KeyCode::Left => cursor('D'),
        KeyCode::Home => cursor('H'),
        KeyCode::End => cursor('F'),
        KeyCode::Insert => b"\x1b[2~".to_vec(),
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        KeyCode::PageUp => b"\x1b[5~".to_vec(),
        KeyCode::PageDown => b"\x1b[6~".to_vec(),
        KeyCode::F(n @ 1..=4) => format!("\x1bO{}", (b'P' + n - 1) as char).into_bytes(),
        KeyCode::F(n @ 5..=12) => {
            let code = [15, 17, 18, 19, 20, 21, 23, 24][usize::from(n) - 5];
            format!("\x1b[{code}~").into_bytes()
        }
        _ => return None,
    };
    if key.modifiers.contains(KeyModifiers::ALT) {
        bytes.insert(0, 0x1b);
    }
    Some(bytes)
}
