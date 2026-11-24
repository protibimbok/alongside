//! The size of a command pane.

use portable_pty::PtySize;

/// Dimensions of a command pane, in character cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaneSize {
    pub rows: u16,
    pub cols: u16,
}

impl From<PaneSize> for PtySize {
    fn from(size: PaneSize) -> Self {
        Self {
            rows: size.rows,
            cols: size.cols,
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}
