use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders};
use tui_term::widget::{Cursor, PseudoTerminal};

use crate::app::{App, Status};

/// Size (rows, cols) of the command pane for a terminal of the given size.
pub fn pane_size(cols: u16, rows: u16) -> (u16, u16) {
    (rows.saturating_sub(2).max(1), cols.max(1))
}

pub fn draw(frame: &mut Frame, app: &App) {
    let [bar, body] =
        Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(frame.area());

    let block = Block::new()
        .borders(Borders::BOTTOM)
        .border_style(Style::new().fg(Color::DarkGray));
    let inner = block.inner(bar);
    frame.render_widget(block, bar);
    frame.render_widget(tab_bar(app), inner);

    let screen = app.panes[app.active].parser.screen();
    let terminal = PseudoTerminal::new(screen).cursor(Cursor::default().visibility(false));
    frame.render_widget(terminal, body);
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
