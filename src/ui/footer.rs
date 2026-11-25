use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::app::{App, Mode};

use super::Theme;

/// Left side of the footer: the keys that do something right now.
pub(super) fn footer(app: &App, theme: &Theme) -> Vec<Span<'static>> {
    match app.mode() {
        Mode::Input | Mode::InputLiteral => {
            hints(&[("esc", "normal mode"), ("^A esc", "send esc")], theme)
        }
        Mode::Normal => hints(
            &[
                ("tab", "switch"),
                ("↑/↓", "scroll"),
                ("r", "restart"),
                ("c", "clear"),
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
pub(super) fn status(app: &App, theme: &Theme) -> Line<'static> {
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
