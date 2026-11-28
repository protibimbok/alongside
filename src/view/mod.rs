//! A terminal screen with scrollback, and where it is scrolled to.

use crate::size::PaneSize;

/// Lines of scrollback kept per view.
pub const SCROLLBACK: usize = 10_000;

pub struct View {
    parser: vt100::Parser,
    /// Lines that have scrolled off the top of the screen.
    history: usize,
    unseen: bool,
}

impl View {
    pub fn new(size: PaneSize) -> Self {
        Self {
            parser: vt100::Parser::new(size.rows, size.cols, SCROLLBACK),
            history: 0,
            unseen: false,
        }
    }

    /// The screen at the current scroll position.
    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }

    pub fn size(&self) -> PaneSize {
        let (rows, cols) = self.screen().size();
        PaneSize { rows, cols }
    }

    pub fn history(&self) -> usize {
        self.history
    }

    pub fn has_unseen_output(&self) -> bool {
        self.unseen
    }

    pub fn resize(&mut self, size: PaneSize) {
        self.parser.screen_mut().set_size(size.rows, size.cols);
        self.measure_history();
    }

    /// Draws terminal output on the screen.
    pub fn process(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
        self.measure_history();
        self.unseen |= self.screen().scrollback() > 0;
    }

    pub fn clear(&mut self) {
        // Keeps the modes that were set, such as application cursor keys.
        let modes = self.screen().input_mode_formatted();
        *self = Self::new(self.size());
        self.parser.process(&modes);
    }

    pub fn scroll(&mut self, lines: isize) -> bool {
        let before = self.screen().scrollback();
        self.scroll_to(before.saturating_add_signed(lines));
        self.screen().scrollback() != before
    }

    /// Scrolls to `offset` lines above the live screen, or as far as there is output.
    pub fn scroll_to(&mut self, offset: usize) {
        let screen = self.parser.screen_mut();
        screen.set_scrollback(offset);
        self.unseen &= screen.scrollback() > 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_to(0);
    }

    pub fn show_line(&mut self, line: usize) {
        let half = usize::from(self.screen().size().0 / 2);
        self.scroll_to((self.history + half).saturating_sub(line));
    }

    pub fn top_line(&self) -> usize {
        self.history.saturating_sub(self.screen().scrollback())
    }

    /// All of the output as text, scrollback included, from the top.
    pub fn lines(&mut self) -> Vec<String> {
        self.all_rows(|screen| screen.rows(0, screen.size().1).collect())
    }

    /// All of the output with its colors, scrollback included, as bytes to
    /// print on a terminal. Rows that wrapped are joined back into one line.
    pub fn formatted(&mut self) -> Vec<u8> {
        let mut rows = self.all_rows(|screen| {
            let wrapped = (0..).map(|row| screen.row_wrapped(row));
            screen
                .rows_formatted(0, screen.size().1)
                .zip(wrapped)
                .collect()
        });
        while rows.last().is_some_and(|(row, _)| row.is_empty()) {
            rows.pop();
        }
        let mut bytes = Vec::new();
        for (row, wrapped) in rows {
            bytes.extend(row);
            if !wrapped {
                bytes.extend(b"\x1b[m\n");
            }
        }
        bytes
    }

    /// Collects every row from the top of the scrollback, a page at a time.
    fn all_rows<T>(&mut self, page: impl Fn(&vt100::Screen) -> Vec<T>) -> Vec<T> {
        let rows = self.screen().size().0;
        let at = self.screen().scrollback();
        let mut all = Vec::with_capacity(self.history + usize::from(rows));
        let mut offset = self.history;
        loop {
            self.parser.screen_mut().set_scrollback(offset);
            // The last page overlaps the one before it unless the history
            // is a whole number of pages.
            let seen = all.len() - (self.history - offset);
            all.extend(page(self.screen()).into_iter().skip(seen));
            if offset == 0 {
                break;
            }
            offset = offset.saturating_sub(usize::from(rows));
        }
        self.parser.screen_mut().set_scrollback(at);
        all
    }

    /// vt100 doesn't expose how much scrollback it holds, but it does clamp
    /// the scroll position to it.
    fn measure_history(&mut self) {
        let screen = self.parser.screen_mut();
        let at = screen.scrollback();
        screen.set_scrollback(usize::MAX);
        self.history = screen.scrollback();
        screen.set_scrollback(at);
    }
}

#[cfg(test)]
mod tests;
