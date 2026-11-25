mod theme;

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};
use tui_term::widget::{Cursor, PseudoTerminal};

use crate::app::{App, Mode};
use crate::pane::{Pane, Status};
use crate::size::PaneSize;

pub use theme::Theme;

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
pub const SPINNER_FRAME: std::time::Duration = std::time::Duration::from_millis(80);

const MIN_SIDEBAR_WIDTH: u16 = 12;
const MAX_SIDEBAR_WIDTH: u16 = 26;
const MIN_FULL_PANE_COLS: u16 = 30;
const MIN_FULL_HEIGHT: u16 = 10;

pub fn sidebar_width<'a>(names: impl IntoIterator<Item = &'a str>) -> u16 {
    let longest = names
        .into_iter()
        .map(|name| Span::raw(name).width())
        .max()
        .unwrap_or(0);
    u16::try_from(longest + 6)
        .unwrap_or(u16::MAX)
        .clamp(MIN_SIDEBAR_WIDTH, MAX_SIDEBAR_WIDTH)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Areas {
    /// No room for the full layout: only `pane`, `scrollbar` and `footer` are shown.
    pub compact: bool,
    pub header: Rect,
    pub sidebar: Rect,
    pub content: Rect,
    /// The command's terminal screen.
    pub pane: Rect,
    pub scrollbar: Rect,
    pub footer: Rect,
}

impl Areas {
    pub fn new(area: Rect, sidebar_width: u16, header: bool) -> Self {
        let Rect { width, height, .. } = area;
        let body_height = height.saturating_sub(1);
        let footer = Rect::new(0, body_height, width, height.min(1));
        // Borders, a column of padding and the scrollbar with the gap before it.
        let chrome = sidebar_width.saturating_add(5);
        if width < chrome.saturating_add(MIN_FULL_PANE_COLS) || height < MIN_FULL_HEIGHT {
            return Self {
                compact: true,
                header: Rect::ZERO,
                sidebar: Rect::ZERO,
                content: Rect::ZERO,
                pane: Rect::new(0, 0, width.saturating_sub(1), body_height),
                scrollbar: Rect::new(width.saturating_sub(1), 0, width.min(1), body_height),
                footer,
            };
        }
        // Inside the content box: the command line, a rule, then the pane.
        let top = u16::from(header);
        let box_height = body_height - top;
        let pane_height = box_height - 4;
        Self {
            compact: false,
            header: Rect::new(0, 0, width, top),
            sidebar: Rect::new(0, top, sidebar_width, box_height),
            content: Rect::new(sidebar_width, top, width - sidebar_width, box_height),
            pane: Rect::new(sidebar_width + 2, top + 3, width - chrome, pane_height),
            scrollbar: Rect::new(width - 2, top + 3, 1, pane_height),
            footer,
        }
    }

    pub fn pane_size(&self) -> PaneSize {
        PaneSize {
            rows: self.pane.height.max(1),
            cols: self.pane.width.max(1),
        }
    }
}

pub fn draw(frame: &mut Frame, app: &App, theme: &Theme) {
    let areas = Areas::new(frame.area(), app.sidebar_width(), app.title().is_some());
    let pane = app.active_pane();
    let screen = pane.screen();

    if !areas.compact {
        if let Some(title) = app.title() {
            draw_header(frame, areas.header, title, app.cwd(), theme);
        }
        draw_sidebar(frame, areas.sidebar, app, theme);
        draw_content_box(frame, areas.content, app, theme);
    }

    let show_cursor = matches!(app.mode(), Mode::Input | Mode::InputLiteral)
        && screen.scrollback() == 0
        && !screen.hide_cursor();
    let terminal = PseudoTerminal::new(screen).cursor(Cursor::default().visibility(show_cursor));
    frame.render_widget(terminal, areas.pane);
    draw_scrollbar(frame.buffer_mut(), areas.scrollbar, pane, theme);

    let mut left = Vec::new();
    if areas.compact {
        left.push(Span::styled(
            format!(
                " {}/{} {} ",
                app.active() + 1,
                app.panes().len(),
                pane.name()
            ),
            Style::new().fg(Color::Black).bg(theme.tab(app.active())),
        ));
    }
    left.extend(footer(app, theme));
    split_row(frame, areas.footer, Line::from(left), status(app, theme));
}

fn draw_header(frame: &mut Frame, area: Rect, title: &str, cwd: &str, theme: &Theme) {
    let title = Line::styled(title, theme.hint);
    let [_, title_area, _, cwd_area, _] = Layout::horizontal([
        Constraint::Length(1),
        Constraint::Length(u16::try_from(title.width()).unwrap_or(u16::MAX)),
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);
    frame.render_widget(title, title_area);
    frame.render_widget(Line::styled(cwd, theme.muted).right_aligned(), cwd_area);
}

/// Draws `left` and `right` at the two ends of a one-line area. `right` is
/// kept whole when they don't both fit.
fn split_row(frame: &mut Frame, area: Rect, left: Line, right: Line) {
    let area = Rect {
        x: area.x + area.width.min(1),
        width: area.width.saturating_sub(2),
        ..area
    };
    let [left_area, _, right_area] = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(u16::try_from(right.width()).unwrap_or(u16::MAX)),
    ])
    .areas(area);
    frame.render_widget(left, left_area);
    frame.render_widget(right, right_area);
}

fn draw_sidebar(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme.border);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    for ((i, pane), row) in app.panes().iter().enumerate().zip(inner.rows()) {
        let color = theme.tab(i);
        let selected = i == app.active();
        let number = if i < 9 {
            (i + 1).to_string()
        } else {
            " ".to_string()
        };
        let (mark, mark_color, name_color) = match (pane.status(), pane.starting()) {
            (Status::Exited(0), _) => ("✓".to_string(), theme.ok, color),
            (Status::Exited(_), _) => ("✕".to_string(), theme.error, theme.error),
            (Status::Running, Some(uptime)) => {
                let frame = uptime.as_millis() / SPINNER_FRAME.as_millis();
                (
                    SPINNER[frame as usize % SPINNER.len()].to_string(),
                    color,
                    color,
                )
            }
            (Status::Running, None) => (number, theme.muted, color),
        };
        if selected {
            // The entry is filled with the command's color, so everything
            // on it is drawn in black instead.
            let fill = Style::new().fg(Color::Black).bg(color);
            let line = Line::raw(format!(" {mark} {}", pane.name()));
            frame.render_widget(Paragraph::new(line).style(fill), row);
        } else {
            let line = Line::from(vec![
                Span::styled(format!(" {mark} "), mark_color),
                Span::styled(pane.name(), name_color),
            ]);
            frame.render_widget(line, row);
        }
    }
}

fn draw_content_box(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let pane = app.active_pane();
    let border = match app.mode() {
        Mode::Input | Mode::InputLiteral => theme.input,
        Mode::Normal => theme.accent,
    };
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(border);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [title, rule] = [0, 1].map(|row| Rect {
        y: inner.y + row,
        height: 1,
        ..inner
    });
    let state = match pane.status() {
        Status::Running => Line::styled("● running", theme.muted),
        Status::Exited(0) => Line::styled("✓ exited 0", theme.ok),
        Status::Exited(code) => Line::styled(format!("✕ exited {code}"), theme.error),
    };
    // Only the first line of a multi-line command fits.
    let command = pane.command().lines().next().unwrap_or_default();
    split_row(frame, title, Line::styled(command, theme.muted), state);
    frame.render_widget(
        Line::styled("─".repeat(usize::from(rule.width)), theme.rule),
        rule,
    );
}

fn draw_scrollbar(buf: &mut Buffer, area: Rect, pane: &Pane, theme: &Theme) {
    let history = pane.history();
    if history == 0 || area.is_empty() {
        return;
    }
    let height = usize::from(area.height);
    let thumb = (height * height / (history + height)).clamp(1, height);
    let top = pane.top_line() * (height - thumb) / history;
    for (i, position) in area.positions().enumerate() {
        let (symbol, color) = if (top..top + thumb).contains(&i) {
            ("┃", theme.muted)
        } else {
            ("│", theme.border)
        };
        buf[position].set_symbol(symbol).set_fg(color);
    }
}

/// Left side of the footer: the keys that do something right now.
fn footer(app: &App, theme: &Theme) -> Vec<Span<'static>> {
    match app.mode() {
        Mode::Input | Mode::InputLiteral => {
            hints(&[("esc", "normal mode"), ("^A esc", "send esc")], theme)
        }
        Mode::Normal => hints(
            &[
                ("tab", "switch"),
                ("↑/↓", "scroll"),
                ("r", "restart"),
                ("i", "input"),
                ("q", "quit"),
            ],
            theme,
        ),
    }
}

fn hints(keys: &[(&'static str, &'static str)], theme: &Theme) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (i, (key, action)) in keys.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("   "));
        }
        spans.push(Span::styled(*key, Style::new().fg(theme.key).bold()));
        spans.push(Span::styled(format!(" {action}"), theme.hint));
    }
    spans
}

/// Right side of the footer: the scroll position and the input mode.
fn status(app: &App, theme: &Theme) -> Line<'static> {
    let pane = app.active_pane();
    let mut spans = Vec::new();
    if pane.has_unseen_output() {
        spans.push(Span::styled("↓ new output ", theme.warn));
    }
    let scrollback = pane.screen().scrollback();
    if scrollback > 0 {
        spans.push(Span::styled(format!("↑{scrollback} "), theme.warn));
    }
    let (label, color) = match app.mode() {
        Mode::Normal => (" NORMAL ", theme.accent),
        Mode::Input => (" INPUT ", theme.input),
        Mode::InputLiteral => (" INPUT ^A ", theme.input),
    };
    spans.push(Span::styled(label, Style::new().fg(Color::Black).bg(color)));
    Line::from(spans)
}

#[cfg(test)]
mod tests;
