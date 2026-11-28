mod footer;
mod help;
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
use help::draw_help;
use highlight::highlight;
use scrollbar::draw_scrollbar;
use sidebar::draw_sidebar;

pub use layout::{Areas, sidebar_width};
pub use scrollbar::Thumb;
pub use sidebar::SPINNER_FRAME;
pub use theme::Theme;

pub fn draw(frame: &mut Frame, app: &App, theme: &Theme) {
    let areas = Areas::new(frame.area(), app.sidebar_width(), app.title().is_some());
    let view = app.view();
    let screen = view.screen();

    if !areas.compact {
        if let Some(title) = app.title() {
            draw_header(frame, areas.header, title, app.cwd(), theme);
        }
        if !app.streaming() {
            draw_sidebar(frame, areas.sidebar, app, theme);
        }
        draw_content_box(frame, &areas, app, theme);
    }
    draw_command(frame, &areas, app, theme);

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
            view.top_line(),
        );
    }
    draw_scrollbar(frame.buffer_mut(), areas.scrollbar, view, theme);

    if app.mode() == Mode::Help {
        draw_help(frame, &areas, app.help_scroll(), theme);
    }
    let left = Line::from(footer(app, theme, &areas));
    split_row(frame, areas.footer, left, status(app, theme));
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

fn draw_content_box(frame: &mut Frame, areas: &Areas, app: &App, theme: &Theme) {
    let border = match app.mode() {
        Mode::Input | Mode::InputLiteral => theme.input,
        Mode::Normal | Mode::Search | Mode::Help | Mode::Filter => theme.accent,
    };
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(border);
    frame.render_widget(block, areas.content);

    let rule = Rect {
        y: areas.command.y + 1,
        ..areas.command
    };
    frame.render_widget(
        Line::styled("─".repeat(usize::from(rule.width)), theme.rule),
        rule,
    );
}

/// The active command and its status, or in stream mode how many commands
/// are shown and running. The compact layout has no sidebar, so there the row
/// names the command instead of showing its command line.
fn draw_command(frame: &mut Frame, areas: &Areas, app: &App, theme: &Theme) {
    let chip = |text: String, color| Span::styled(text, Style::new().fg(Color::Black).bg(color));
    let panes = app.panes();
    if app.streaming() {
        let count = panes.len();
        let shown = (0..count).filter(|&i| !app.stream().is_hidden(i)).count();
        let running = panes
            .iter()
            .filter(|pane| matches!(pane.status(), Status::Running))
            .count();
        let left = match (areas.compact, shown == count) {
            (true, _) => chip(format!(" stream {shown}/{count} "), theme.accent),
            (false, true) => Span::styled("output of every command", theme.muted),
            (false, false) => Span::styled(
                format!("output of {shown} of {count} commands"),
                theme.muted,
            ),
        };
        let state = Line::styled(format!("● {running}/{count} running"), theme.muted);
        return split_row(frame, areas.command, Line::from(left), state);
    }

    let pane = app.active_pane();
    let state = match pane.status() {
        Status::Running => Line::styled("● running", theme.muted),
        Status::Exited(0) => Line::styled("✓ exited 0", theme.ok),
        Status::Exited(code) => Line::styled(format!("✕ exited {code}"), theme.error),
    };
    let left = if areas.compact {
        chip(
            format!(" {}/{} {} ", app.active() + 1, panes.len(), pane.name()),
            theme.tab(app.active()),
        )
    } else {
        // Only the first line of a multi-line command fits.
        let command = pane.command().lines().next().unwrap_or_default();
        Span::styled(command, theme.muted)
    };
    split_row(frame, areas.command, Line::from(left), state);
}

#[cfg(test)]
mod tests;
