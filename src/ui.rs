use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use tui_term::widget::{Cursor, PseudoTerminal};

use crate::app::App;
use crate::pane::Status;
use crate::size::PaneSize;

/// Size of the command pane for a terminal of the given size.
pub fn pane_size(width: u16, height: u16) -> PaneSize {
    PaneSize {
        rows: height.saturating_sub(2).max(1),
        cols: width.max(1),
    }
}

pub fn draw(frame: &mut Frame, app: &App) {
    let [bar, body] =
        Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(frame.area());

    let block = Block::new()
        .borders(Borders::BOTTOM)
        .border_style(Style::new().fg(Color::DarkGray));
    let inner = block.inner(bar);
    frame.render_widget(block, bar);

    let screen = app.panes[app.active].parser.screen();

    let status = status(app, screen);
    let [tabs, right] = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(status.width() as u16),
    ])
    .areas(inner);
    frame.render_widget(tab_bar(app), tabs);
    frame.render_widget(Paragraph::new(status), right);

    let show_cursor = app.input_mode && screen.scrollback() == 0 && !screen.hide_cursor();
    let terminal = PseudoTerminal::new(screen).cursor(Cursor::default().visibility(show_cursor));
    frame.render_widget(terminal, body);
}

/// Right side of the tab bar: the scroll position and the input mode.
fn status(app: &App, screen: &vt100::Screen) -> Line<'static> {
    let mut spans = Vec::new();
    if screen.scrollback() > 0 {
        spans.push(Span::styled(
            format!(" ↑{} ", screen.scrollback()),
            Style::new().fg(Color::Yellow),
        ));
    }
    let mode = match (app.input_mode, app.send_to_main) {
        (false, _) => Span::styled(" NORMAL ", Style::new().fg(Color::Black).bg(Color::Blue)),
        (true, false) => Span::styled(" INPUT ", Style::new().fg(Color::Black).bg(Color::Magenta)),
        (true, true) => Span::styled(
            " INPUT ^A ",
            Style::new().fg(Color::Black).bg(Color::Magenta),
        ),
    };
    spans.push(mode);
    Line::from(spans)
}

fn tab_bar(app: &App) -> Line<'_> {
    let mut spans = Vec::new();
    for (i, pane) in app.panes.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("│", Style::new().fg(Color::DarkGray)));
        }
        let (mark, color) = match pane.status {
            Status::Running => ("●".to_string(), Color::Reset),
            Status::Exited(0) => ("✓".to_string(), Color::Green),
            Status::Exited(code) => (format!("✗ {code}"), Color::Red),
        };
        let mut style = Style::new().fg(color);
        if i == app.active {
            style = style.bold().reversed();
        }
        spans.push(Span::styled(
            format!(" {} {} {mark} ", i + 1, pane.name),
            style,
        ));
    }
    Line::from(spans)
}
