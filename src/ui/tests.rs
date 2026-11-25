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
