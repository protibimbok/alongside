//! Application state and input handling.

use std::sync::mpsc::Sender;

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind};
use ratatui::layout::Rect;

use crate::pane::{self, Pane, Status};
use crate::ui::{self, Areas};

/// Lines scrolled per mouse wheel step.
const WHEEL_LINES: i16 = 3;
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
    Output { id: Id, bytes: Vec<u8> },
    Exited { id: Id, code: u32 },
}

/// What key presses do.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Keys are shortcuts.
    Normal,
    /// Keys go to the active command.
    Input,
    /// `Ctrl+A` was pressed in input mode; the next key is sent literally.
    InputLiteral,
}

pub struct App {
    panes: Vec<Pane>,
    active: usize,
    quit: bool,
    mode: Mode,
    areas: Areas,
    sidebar_width: u16,
    title: Option<String>,
    cwd: String,
    tx: Sender<Msg>,
}

impl App {
    pub fn new(
        commands: &[String],
        names: &[String],
        title: Option<String>,
        (cols, rows): (u16, u16),
        tx: Sender<Msg>,
    ) -> Result<Self> {
        let names: Vec<String> = commands
            .iter()
            .enumerate()
            .map(|(i, command)| match names.get(i).map(|n| n.trim()) {
                Some(name) if !name.is_empty() => truncate(name, MAX_NAME),
                _ => truncate(command, MAX_NAME),
            })
            .collect();
        let sidebar_width = ui::sidebar_width(names.iter().map(String::as_str));
        let areas = Areas::new(Rect::new(0, 0, cols, rows), sidebar_width, title.is_some());
        let panes = names
            .into_iter()
            .zip(commands)
            .enumerate()
            .map(|(i, (name, command))| {
                Pane::spawn(i, name, command.clone(), areas.pane_size(), &tx)
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            panes,
            active: 0,
            quit: false,
            mode: Mode::Normal,
            areas,
            sidebar_width,
            title,
            cwd: working_dir(),
            tx,
        })
    }

    pub fn panes(&self) -> &[Pane] {
        &self.panes
    }

    pub fn active(&self) -> usize {
        self.active
    }

    pub fn active_pane(&self) -> &Pane {
        &self.panes[self.active]
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn sidebar_width(&self) -> u16 {
        self.sidebar_width
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn cwd(&self) -> &str {
        &self.cwd
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    pub fn animating(&self) -> bool {
        self.panes.iter().any(|pane| pane.starting().is_some())
    }

    /// Applies a message and returns whether the screen needs a redraw.
    pub fn handle(&mut self, msg: Msg) -> bool {
        match msg {
            Msg::Output { id, bytes } if self.is_current(id) => {
                self.panes[id.pane].process(&bytes);
                id.pane == self.active
            }
            Msg::Exited { id, code } if self.is_current(id) => {
                self.panes[id.pane].set_exited(code);
                if self
                    .panes
                    .iter()
                    .all(|p| matches!(p.status(), Status::Exited(_)))
                {
                    self.quit = true;
                }
                true
            }
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
                self.resize(cols, rows);
                true
            }
            // Unhandled input, and events from an earlier run of a restarted command.
            Msg::Output { .. } | Msg::Exited { .. } | Msg::Input(_) => false,
        }
    }

    fn on_key(&mut self, key: KeyEvent) -> bool {
        if self.mode != Mode::Normal {
            return self.on_input_key(key);
        }
        let count = self.panes.len();
        let page = i16::try_from(self.areas.pane_size().rows).unwrap_or(i16::MAX);
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.quit = true,
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char('r') => self.restart(),
            KeyCode::Char('i') => self.mode = Mode::Input,
            KeyCode::PageUp => return self.scroll(page),
            KeyCode::PageDown => return self.scroll(-page),
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
        if std::mem::replace(&mut self.mode, Mode::Input) == Mode::InputLiteral {
            if key.code != KeyCode::Esc && !ctrl_a {
                self.panes[self.active].write(&[0x01]);
            }
        } else if key.code == KeyCode::Esc {
            self.mode = Mode::Normal;
            return true;
        } else if ctrl_a {
            self.mode = Mode::InputLiteral;
            return true;
        }

        let pane = &mut self.panes[self.active];
        if let Some(bytes) = encode_key(key, pane.screen().application_cursor()) {
            pane.write(&bytes);
        }
        // Typing jumps back to the live screen.
        pane.scroll_to_bottom();
        true
    }

    fn scroll(&mut self, lines: i16) -> bool {
        self.panes[self.active].scroll(lines)
    }

    fn is_current(&self, id: Id) -> bool {
        self.panes[id.pane].is_current(id)
    }

    fn restart(&mut self) {
        self.panes[self.active].restart(self.areas.pane_size(), &self.tx);
    }

    fn resize(&mut self, cols: u16, rows: u16) {
        self.areas = Areas::new(
            Rect::new(0, 0, cols, rows),
            self.sidebar_width,
            self.title.is_some(),
        );
        for pane in &mut self.panes {
            pane.resize(self.areas.pane_size());
        }
    }
}

/// Stops every command that is still running. They are stopped together, so
/// they share one grace period instead of each getting its own.
impl Drop for App {
    fn drop(&mut self) {
        pane::stop_all(&mut self.panes);
    }
}

fn working_dir() -> String {
    let Ok(dir) = std::env::current_dir() else {
        return String::new();
    };
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" });
    match home.map(|home| dir.strip_prefix(home).map(std::path::Path::to_path_buf)) {
        Some(Ok(rest)) if rest.as_os_str().is_empty() => "~".to_string(),
        Some(Ok(rest)) => std::path::Path::new("~").join(rest).display().to_string(),
        _ => dir.display().to_string(),
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

#[cfg(test)]
mod tests;
