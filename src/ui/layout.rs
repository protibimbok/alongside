use ratatui::layout::{Position, Rect};
use ratatui::text::Span;
use ratatui::widgets::Block;

use crate::size::PaneSize;

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

    pub fn sidebar_entry(&self, column: u16, row: u16) -> Option<usize> {
        let inner = Block::bordered().inner(self.sidebar);
        inner
            .contains(Position::new(column, row))
            .then(|| usize::from(row - inner.y))
    }
}
