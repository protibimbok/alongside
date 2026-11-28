use super::*;

fn split(chunks: &[&str]) -> Vec<String> {
    let mut splitter = Splitter::default();
    let mut lines = Vec::new();
    for chunk in chunks {
        splitter.feed(chunk.as_bytes(), |line| {
            lines.push(String::from_utf8(line).unwrap());
        });
    }
    lines
}

fn stream(names: &[&str], cols: u16) -> Stream {
    let commands = names.iter().map(|name| (*name, Color::Red));
    Stream::new(commands, PaneSize { rows: 5, cols })
}

fn text(stream: &mut Stream) -> Vec<String> {
    let mut lines = stream.view_mut().lines();
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}

#[test]
fn lines_end_at_a_line_feed_even_across_chunks() {
    assert_eq!(split(&["one\r\ntw", "o\nthree"]), ["one", "two"]);
}

#[test]
fn a_carriage_return_starts_the_line_over() {
    assert_eq!(split(&["10%\r50%\r100%\n", "done\r\n"]), ["100%", "done"]);
}

#[test]
fn colors_are_kept_and_other_sequences_dropped() {
    assert_eq!(
        split(&[
            "\x1b[2K\x1b[1;32mok\x1b[0m \x1b]0;title\x07\x1b(B\x1b[?25lfine\x1b[3",
            "A\n"
        ]),
        ["\x1b[1;32mok\x1b[0m fine"]
    );
}

#[test]
fn colors_carry_over_to_the_next_line() {
    let lines = split(&["\x1b[31mred\nstill red\x1b[m\nplain\n"]);
    assert_eq!(lines[0], "\x1b[31mred");
    assert_eq!(lines[1], "\x1b[m\x1b[31mstill red\x1b[m");
    assert_eq!(lines[2], "plain");
}

#[test]
fn lines_are_labelled_with_their_command() {
    let mut stream = stream(&["web", "db"], 40);
    stream.push(0, b"listening\r\nrequ");
    stream.push(1, b"\x1b[32mready\x1b[m\r\n");
    stream.push(0, b"est\r\n");
    assert_eq!(
        text(&mut stream),
        ["web │ listening", "db  │ ready", "web │ request"]
    );
}

#[test]
fn ending_a_command_flushes_its_unfinished_line() {
    let mut stream = stream(&["web"], 40);
    stream.push(0, b"bye");
    stream.end(0, "exited with code 0");
    stream.push(0, b"again\n");
    assert_eq!(
        text(&mut stream),
        ["web │ bye", "web │ exited with code 0", "web │ again"]
    );
}

#[test]
fn hidden_commands_come_back_in_order() {
    let mut stream = stream(&["a", "b"], 40);
    stream.push(0, b"1\n");
    stream.push(1, b"2\n");
    stream.push(0, b"3\n");
    assert!(stream.toggle(0));
    assert!(stream.is_filtered());
    stream.push(0, b"4\n");
    assert_eq!(text(&mut stream), ["b │ 2"]);
    assert!(!stream.toggle(1), "the last command shown stays");
    assert!(stream.toggle(0));
    assert_eq!(text(&mut stream), ["a │ 1", "b │ 2", "a │ 3", "a │ 4"]);
}

#[test]
fn resizing_wraps_the_lines_again() {
    let mut stream = stream(&["a"], 10);
    stream.push(0, b"123456789\n");
    assert_eq!(text(&mut stream), ["a │ 123456", "789"]);
    stream.resize(PaneSize { rows: 5, cols: 20 });
    assert_eq!(text(&mut stream), ["a │ 123456789"]);
}

#[test]
fn clearing_drops_every_line() {
    let mut stream = stream(&["a"], 20);
    stream.push(0, b"1\n");
    stream.clear();
    stream.push(0, b"2\n");
    assert_eq!(text(&mut stream), ["a │ 2"]);
}
