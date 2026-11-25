//! Application state and input handling.

use std::sync::mpsc::Sender;

use anyhow::Result;
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use ratatui::layout::{Position, Rect};

use crate::pane::{self, Pane, Status};
use crate::search::Search;
use crate::ui::{self, Areas, Thumb};

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
    /// Keys edit the search query.
    Search,
}

pub struct App {
    panes: Vec<Pane>,
    active: usize,
    quit: bool,
    mode: Mode,
    /// While the scrollbar is dragged: the row of the thumb that is held.
    drag: Option<usize>,
    search: Search,
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
            drag: None,
            search: Search::default(),
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

    pub fn search(&self) -> &Search {
        &self.search
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
                MouseEventKind::Down(MouseButton::Left)
                    if self
                        .areas
                        .scrollbar
                        .contains(Position::new(mouse.column, mouse.row)) =>
                {
                    self.grab_scrollbar(mouse.row)
                }
                MouseEventKind::Drag(MouseButton::Left) => self.drag_scrollbar(mouse.row),
                MouseEventKind::Up(MouseButton::Left) => {
                    self.drag = None;
                    false
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    match self.areas.sidebar_entry(mouse.column, mouse.row) {
                        Some(i) if i < self.panes.len() => self.select(i),
                        _ => return false,
                    }
                    true
                }
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
        match self.mode {
            Mode::Normal => {}
            Mode::Search => return self.on_search_key(key),
            Mode::Input | Mode::InputLiteral => return self.on_input_key(key),
        }
        let count = self.panes.len();
        let page = isize::try_from(self.areas.pane_size().rows).unwrap_or(isize::MAX);
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.quit = true,
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char('r') => self.restart(),
            KeyCode::Char('i') => self.mode = Mode::Input,
            KeyCode::Char('c') => {
                self.panes[self.active].clear();
                self.find();
            }
            KeyCode::Char('/') => {
                self.search.clear();
                self.mode = Mode::Search;
            }
            KeyCode::Char('n') if self.search.is_active() => self.step_match(true),
            KeyCode::Char('N') if self.search.is_active() => self.step_match(false),
            KeyCode::Esc if self.search.is_active() => self.search.clear(),
            KeyCode::Up | KeyCode::Char('k') => return self.scroll(1),
            KeyCode::Down | KeyCode::Char('j') => return self.scroll(-1),
            KeyCode::PageUp => return self.scroll(page),
            KeyCode::PageDown => return self.scroll(-page),
            KeyCode::Char('g') => self.panes[self.active].scroll_to(usize::MAX),
            KeyCode::Char('G') => self.panes[self.active].scroll_to_bottom(),
            KeyCode::Tab => self.select((self.active + 1) % count),
            KeyCode::BackTab => self.select((self.active + count - 1) % count),
            KeyCode::Char(c @ '1'..='9') => {
                let i = c as usize - '1' as usize;
                if i < count {
                    self.select(i);
                }
            }
            _ => return false,
        }
        true
    }

    /// Search mode: typing edits the query and marks its matches as it
    /// changes. `Enter` keeps the search and jumps to the most recent match,
    /// `Esc` drops it.
    fn on_search_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.search.clear();
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                self.mode = Mode::Normal;
                self.find();
                if let Some(line) = self.search.select_last() {
                    self.panes[self.active].show_line(line);
                }
            }
            KeyCode::Backspace => {
                self.search.pop();
                self.find();
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.search.push(c);
                self.find();
            }
            _ => return false,
        }
        true
    }

    /// Looks the search up again in the active pane, whose output may have
    /// changed since.
    fn find(&mut self) {
        if self.search.is_active() {
            self.search.update(&self.panes[self.active].lines());
        }
    }

    fn step_match(&mut self, forward: bool) {
        self.find();
        if let Some(line) = self.search.step(forward) {
            self.panes[self.active].show_line(line);
        }
    }

    fn select(&mut self, pane: usize) {
        self.active = pane;
        self.find();
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

    fn scroll(&mut self, lines: isize) -> bool {
        self.panes[self.active].scroll(lines)
    }

    /// Starts a drag. Pressing the track brings the middle of the thumb there;
    /// pressing the thumb holds it where it is.
    fn grab_scrollbar(&mut self, row: u16) -> bool {
        let pane = &self.panes[self.active];
        let track = self.areas.scrollbar;
        let Some(thumb) = Thumb::new(track.height.into(), pane.history(), pane.top_line()) else {
            return false;
        };
        let at = usize::from(row - track.y);
        if (thumb.top..thumb.top + thumb.len).contains(&at) {
            self.drag = Some(at - thumb.top);
            return false;
        }
        self.drag = Some(thumb.len / 2);
        self.drag_scrollbar(row)
    }

    fn drag_scrollbar(&mut self, row: u16) -> bool {
        let Some(held) = self.drag else {
            return false;
        };
        let pane = &mut self.panes[self.active];
        let track = self.areas.scrollbar;
        let height = usize::from(track.height);
        let history = pane.history();
        let Some(thumb) = Thumb::new(height, history, pane.top_line()) else {
            return false;
        };
        let top = usize::from(row.saturating_sub(track.y)).saturating_sub(held);
        let before = pane.screen().scrollback();
        pane.scroll_to(history - thumb.top_line(height, history, top));
        pane.screen().scrollback() != before
    }

    fn is_current(&self, id: Id) -> bool {
        self.panes[id.pane].is_current(id)
    }

    fn restart(&mut self) {
        self.panes[self.active].restart(self.areas.pane_size(), &self.tx);
        self.find();
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
        self.find();
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
