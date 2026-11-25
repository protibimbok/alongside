use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};

use crate::app::App;
use crate::pane::Status;

use super::Theme;

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
pub const SPINNER_FRAME: std::time::Duration = std::time::Duration::from_millis(80);

pub(super) fn draw_sidebar(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
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
