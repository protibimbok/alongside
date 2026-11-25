use ratatui::Frame;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear};

use super::{Areas, Theme};

const KEYS: [(&str, &str); 14] = [
    ("tab / shift+tab", "next / previous command"),
    ("1–9", "nth command"),
    ("r", "restart command"),
    ("c", "clear output"),
    ("↑ / ↓  k / j", "scroll one line"),
    ("pgup / pgdn", "scroll one page"),
    ("g / G", "top / bottom"),
    ("/", "search output"),
    ("n / N", "next / previous match"),
    ("i", "input mode"),
    ("esc", "leave input mode"),
    ("^A esc", "send esc to the command"),
    ("h", "toggle this help"),
    ("q  ^C", "quit"),
];
const GAP: usize = 2;

fn key_width() -> usize {
    KEYS.iter()
        .map(|(key, _)| Span::raw(*key).width())
        .max()
        .unwrap_or(0)
}

/// Columns and rows the whole help text takes.
pub(super) fn size() -> (u16, u16) {
    let action = KEYS
        .iter()
        .map(|(_, action)| Span::raw(*action).width())
        .max()
        .unwrap_or(0);
    (
        u16::try_from(key_width() + GAP + action).unwrap_or(u16::MAX),
        u16::try_from(KEYS.len()).unwrap_or(u16::MAX),
    )
}

pub(super) fn draw_help(frame: &mut Frame, areas: &Areas, scroll: u16, theme: &Theme) {
    let text = areas.help;
    let (pad_x, pad_y) = if areas.compact { (1, 0) } else { (2, 1) };
    let outer = ratatui::layout::Rect {
        x: text.x.saturating_sub(pad_x),
        y: text.y.saturating_sub(pad_y),
        width: text.width + 2 * pad_x,
        height: text.height + 2 * pad_y,
    }
    .intersection(frame.area());
    frame.render_widget(Clear, outer);
    if !areas.compact {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(theme.accent)
            .title(Span::styled(" keys ", theme.accent));
        frame.render_widget(block, outer);
    }

    let column = key_width() + GAP;
    let keys = KEYS.iter().skip(usize::from(scroll));
    for ((key, action), row) in keys.zip(text.rows()) {
        let pad = column - Span::raw(*key).width();
        let line = Line::from(vec![
            Span::styled(*key, Style::new().fg(theme.key).bold()),
            Span::raw(" ".repeat(pad)),
            Span::styled(*action, theme.hint),
        ]);
        frame.render_widget(line, row);
    }
}
