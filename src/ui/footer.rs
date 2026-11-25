use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::app::{App, Mode};

use super::Theme;

/// Left side of the footer: the keys that do something right now, or the search.
pub(super) fn footer<'a>(app: &'a App, theme: &Theme) -> Vec<Span<'a>> {
    let search = app.search();
    let prompt = Style::new().fg(theme.warn).bold();
    match app.mode() {
        Mode::Search => {
            let mut spans = vec![
                Span::styled("/ ", prompt),
                Span::raw(search.query()),
                Span::styled("█", theme.warn),
            ];
            if search.is_active() {
                spans.push(match search.count() {
                    0 => Span::styled(" no matches", theme.error),
                    1 => Span::styled(" 1 match", theme.muted),
                    n => Span::styled(format!(" {n} matches"), theme.muted),
                });
            }
            spans
        }
        Mode::Input | Mode::InputLiteral => {
            hints(&[("esc", "normal mode"), ("^A esc", "send esc")], theme)
        }
        Mode::Normal if search.is_active() => {
            let (label, style) = if search.count() == 0 {
                ("[no matches]".to_string(), Style::new().fg(theme.error))
            } else {
                (
                    format!("[{}/{}]", search.position(), search.count()),
                    prompt,
                )
            };
            let mut spans = vec![
                Span::styled(label, style),
                Span::styled(format!(" {}   ", search.query()), theme.muted),
            ];
            spans.extend(hints(
                &[("n", "next"), ("N", "prev"), ("esc", "clear")],
                theme,
            ));
            spans
        }
        Mode::Normal => hints(
            &[
                ("tab", "switch"),
                ("↑/↓", "scroll"),
                ("r", "restart"),
                ("c", "clear"),
                ("/", "search"),
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
        Mode::Search => (" SEARCH ", theme.warn),
        Mode::Input => (" INPUT ", theme.input),
        Mode::InputLiteral => (" INPUT ^A ", theme.input),
    };
    spans.push(Span::styled(label, Style::new().fg(Color::Black).bg(color)));
    Line::from(spans)
}
