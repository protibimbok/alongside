use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::pane::Pane;

use super::Theme;

/// The part of the scrollbar that stands for the lines on screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Thumb {
    pub top: usize,
    pub len: usize,
}

impl Thumb {
    /// `None` when there is nothing to scroll or no room for a scrollbar.
    pub fn new(height: usize, history: usize, top_line: usize) -> Option<Self> {
        if history == 0 || height == 0 {
            return None;
        }
        let len = (height * height / (history + height)).clamp(1, height);
        Some(Self {
            top: top_line * (height - len) / history,
            len,
        })
    }

    /// The first line on screen that puts a thumb of this length at row `top`.
    pub fn top_line(self, height: usize, history: usize, top: usize) -> usize {
        let travel = height - self.len;
        if travel == 0 {
            return history;
        }
        // Rounded up, so that drawing the thumb for this line lands on `top` again.
        (top.min(travel) * history).div_ceil(travel)
    }
}

pub(super) fn draw_scrollbar(buf: &mut Buffer, area: Rect, pane: &Pane, theme: &Theme) {
    let height = usize::from(area.height);
    let Some(thumb) = Thumb::new(height, pane.history(), pane.top_line()) else {
        return;
    };
    for (i, position) in area.positions().enumerate() {
        let (symbol, color) = if (thumb.top..thumb.top + thumb.len).contains(&i) {
            ("┃", theme.muted)
        } else {
            ("│", theme.border)
        };
        buf[position].set_symbol(symbol).set_fg(color);
    }
}
