use ratatui::layout::Rect;

use crate::size::PaneSize;

use super::layout::{MAX_SIDEBAR_WIDTH, MIN_SIDEBAR_WIDTH};
use super::*;

fn areas(width: u16, height: u16) -> Areas {
    Areas::new(Rect::new(0, 0, width, height), 12, true)
}

#[test]
fn pane_takes_the_header_row_when_there_is_no_title() {
    let areas = Areas::new(Rect::new(0, 0, 80, 24), 12, false);
    assert!(areas.header.is_empty());
    assert_eq!(areas.sidebar, Rect::new(0, 0, 12, 23));
    assert_eq!(areas.content, Rect::new(12, 0, 68, 23));
    assert_eq!(areas.pane, Rect::new(14, 3, 63, 19));
    assert_eq!(areas.scrollbar, Rect::new(78, 3, 1, 19));
}

#[test]
fn sidebar_fits_the_longest_name() {
    assert_eq!(sidebar_width(["web", "a-longer-name"]), 19);
    assert_eq!(sidebar_width(["web"]), MIN_SIDEBAR_WIDTH);
}

#[test]
fn sidebar_stops_growing_at_its_maximum() {
    assert_eq!(sidebar_width(["a".repeat(20).as_str()]), MAX_SIDEBAR_WIDTH);
    assert_eq!(sidebar_width(["a".repeat(200).as_str()]), MAX_SIDEBAR_WIDTH);
    assert_eq!(sidebar_width(["宽".repeat(20).as_str()]), MAX_SIDEBAR_WIDTH);
}

#[test]
fn pane_sits_inside_the_content_box() {
    let areas = areas(80, 24);
    assert!(!areas.compact);
    assert_eq!(areas.pane_size(), PaneSize { rows: 18, cols: 63 });
    assert_eq!(areas.pane, Rect::new(14, 4, 63, 18));
    assert_eq!(areas.scrollbar, Rect::new(78, 4, 1, 18));
    assert_eq!(areas.footer, Rect::new(0, 23, 80, 1));
}

#[test]
fn small_terminals_get_the_compact_layout() {
    for (width, height) in [(46, 24), (80, 9)] {
        let areas = areas(width, height);
        assert!(areas.compact);
        assert_eq!(
            areas.pane_size(),
            PaneSize {
                rows: height - 1,
                cols: width - 1
            }
        );
    }
    assert!(!areas(47, 10).compact);
}

#[test]
fn pane_size_never_reaches_zero() {
    assert_eq!(areas(0, 0).pane_size(), PaneSize { rows: 1, cols: 1 });
    assert_eq!(areas(1, 1).pane_size(), PaneSize { rows: 1, cols: 1 });
}

#[test]
fn sidebar_entries_are_found_by_position() {
    let areas = areas(80, 24);
    assert_eq!(areas.sidebar_entry(1, 2), Some(0));
    assert_eq!(areas.sidebar_entry(10, 4), Some(2));
    assert_eq!(areas.sidebar_entry(0, 2), None);
    assert_eq!(areas.sidebar_entry(1, 1), None);
    assert_eq!(areas.sidebar_entry(12, 2), None);
    assert_eq!(self::areas(40, 24).sidebar_entry(1, 2), None);
}

#[test]
fn thumb_spans_the_share_of_output_on_screen() {
    assert_eq!(Thumb::new(10, 0, 0), None);
    assert_eq!(Thumb::new(0, 5, 0), None);
    assert_eq!(Thumb::new(10, 10, 10), Some(Thumb { top: 5, len: 5 }));
    assert_eq!(Thumb::new(10, 10, 0), Some(Thumb { top: 0, len: 5 }));
    assert_eq!(
        Thumb::new(10, 10_000, 5_000),
        Some(Thumb { top: 4, len: 1 })
    );
}

#[test]
fn dragging_the_thumb_to_a_row_draws_it_on_that_row() {
    for (height, history) in [(1, 3), (10, 1), (10, 10), (18, 37), (40, 10_000)] {
        let len = Thumb::new(height, history, 0).unwrap().len;
        for top in 0..=height - len {
            let line = Thumb { top, len }.top_line(height, history, top);
            assert!(line <= history);
            assert_eq!(Thumb::new(height, history, line).unwrap().top, top);
        }
    }
}

#[test]
fn dragging_past_the_ends_reaches_the_first_and_last_line() {
    let thumb = Thumb::new(10, 100, 50).unwrap();
    assert_eq!(thumb.top_line(10, 100, 0), 0);
    assert_eq!(thumb.top_line(10, 100, 99), 100);
}
