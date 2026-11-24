use super::*;

#[test]
fn pane_size_leaves_room_for_the_tab_bar() {
    assert_eq!(pane_size(80, 24), PaneSize { rows: 22, cols: 80 });
}

#[test]
fn pane_size_never_reaches_zero() {
    assert_eq!(pane_size(0, 0), PaneSize { rows: 1, cols: 1 });
    assert_eq!(pane_size(80, BAR_HEIGHT), PaneSize { rows: 1, cols: 80 });
}
