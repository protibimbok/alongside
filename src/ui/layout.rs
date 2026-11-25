use ratatui::layout::{Position, Rect};
use ratatui::text::Span;
use ratatui::widgets::Block;

use crate::size::PaneSize;

use super::help;

pub(super) const MIN_SIDEBAR_WIDTH: u16 = 12;
pub(super) const MAX_SIDEBAR_WIDTH: u16 = 26;
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
    /// No room for the full layout: only `command`, `pane`, `scrollbar` and
    /// `footer` are shown.
    pub compact: bool,
    pub header: Rect,
    pub sidebar: Rect,
    pub content: Rect,
    /// The row naming the command and its status.
    pub command: Rect,
    pub help: Rect,
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
            let top = body_height.min(1);
            let pane_height = body_height - top;
            return Self {
                compact: true,
                header: Rect::ZERO,
                sidebar: Rect::ZERO,
                content: Rect::ZERO,
                command: Rect::new(0, 0, width, top),
                help: Rect::new(width.min(1), 0, width.saturating_sub(2), body_height),
                pane: Rect::new(0, top, width.saturating_sub(1), pane_height),
                scrollbar: Rect::new(width.saturating_sub(1), top, width.min(1), pane_height),
                footer,
            };
        }
        // Inside the content box: the command line, a rule, then the pane.
        let top = u16::from(header);
        let box_height = body_height - top;
        let pane_height = box_height - 4;

        let (help_width, help_height) = help::size();
        let help_width = help_width.min(width - 4);
        let help_height = help_height.min(body_height - 2);
        Self {
            compact: false,
            header: Rect::new(0, 0, width, top),
            sidebar: Rect::new(0, top, sidebar_width, box_height),
            content: Rect::new(sidebar_width, top, width - sidebar_width, box_height),
            command: Rect::new(sidebar_width + 1, top + 1, width - sidebar_width - 2, 1),
            help: Rect::new(
                (width - help_width) / 2,
                (body_height - help_height) / 2,
                help_width,
                help_height,
            ),
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

    pub fn max_help_scroll(&self) -> u16 {
        help::size().1.saturating_sub(self.help.height)
    }

    pub fn sidebar_entry(&self, column: u16, row: u16) -> Option<usize> {
        let inner = Block::bordered().inner(self.sidebar);
        inner
            .contains(Position::new(column, row))
            .then(|| usize::from(row - inner.y))
    }
}
