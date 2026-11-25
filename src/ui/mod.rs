mod footer;
mod highlight;
mod layout;
mod scrollbar;
mod sidebar;
mod theme;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType};
use tui_term::widget::{Cursor, PseudoTerminal};

use crate::app::{App, Mode};
use crate::pane::Status;

use footer::{footer, status};
use highlight::highlight;
use scrollbar::draw_scrollbar;
use sidebar::draw_sidebar;

pub use layout::{Areas, sidebar_width};
pub use scrollbar::Thumb;
pub use sidebar::SPINNER_FRAME;
pub use theme::Theme;

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
    if app.search().is_active() {
        highlight(
            frame.buffer_mut(),
            areas.pane,
            app.search(),
            pane.top_line(),
        );
    }
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

fn draw_content_box(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let pane = app.active_pane();
    let border = match app.mode() {
        Mode::Input | Mode::InputLiteral => theme.input,
        Mode::Normal | Mode::Search => theme.accent,
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

#[cfg(test)]
mod tests;
