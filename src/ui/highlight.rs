use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

use crate::search::{Search, fold};

pub(super) fn highlight(buf: &mut Buffer, area: Rect, search: &Search, top_line: usize) {
    let found = Style::new().fg(Color::Black).bg(Color::Yellow);
    let current = Style::new().fg(Color::Black).bg(Color::LightRed).bold();
    for (line, row) in (top_line..).zip(area.rows()) {
        // The second cell of a wide character holds no symbol of its own.
        let (columns, text): (Vec<u16>, Vec<char>) = row
            .positions()
            .filter_map(|pos| Some((pos.x, fold(buf[pos].symbol().chars().next()?))))
            .unzip();
        for (nth, start) in search.find_in(&text).enumerate() {
            let style = if search.current() == Some((line, nth)) {
                current
            } else {
                found
            };
            for &x in &columns[start..start + search.len()] {
                buf[(x, row.y)].set_style(style);
            }
        }
    }
}
