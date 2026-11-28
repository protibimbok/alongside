use super::*;

fn view(cols: u16, output: &str) -> View {
    let mut view = View::new(PaneSize { rows: 3, cols });
    view.process(output.as_bytes());
    view
}

fn formatted(view: &mut View) -> String {
    String::from_utf8(view.formatted()).unwrap()
}

#[test]
fn formatted_starts_at_the_top_of_the_scrollback() {
    let mut view = view(10, "1\r\n2\r\n3\r\n4\r\n5\r\n6\r\n7");
    view.scroll_to(2);
    assert_eq!(
        formatted(&mut view).replace("\x1b[m", ""),
        "1\n2\n3\n4\n5\n6\n7\n"
    );
    assert_eq!(view.screen().scrollback(), 2);
}

#[test]
fn formatted_keeps_colors_and_ends_them_with_the_line() {
    let mut view = view(10, "\x1b[31mred\r\nstill");
    assert_eq!(
        formatted(&mut view),
        "\x1b[31mred\x1b[m\n\x1b[31mstill\x1b[m\n"
    );
}

#[test]
fn formatted_joins_wrapped_rows_and_drops_the_empty_end() {
    let mut view = view(4, "abcdef\r\n");
    assert_eq!(formatted(&mut view), "abcdef\x1b[m\n");
}

#[test]
fn formatted_is_empty_without_output() {
    assert_eq!(formatted(&mut view(4, "")), "");
}
