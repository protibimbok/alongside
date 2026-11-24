use super::*;

fn encode(code: KeyCode, modifiers: KeyModifiers) -> Option<Vec<u8>> {
    encode_key(KeyEvent::new(code, modifiers), false)
}

#[test]
fn truncate_leaves_text_that_fits() {
    assert_eq!(truncate("npm run dev", 20), "npm run dev");
    assert_eq!(truncate(&"a".repeat(20), 20), "a".repeat(20));
}

#[test]
fn truncate_shortens_to_max_chars_with_an_ellipsis() {
    assert_eq!(
        truncate(&"a".repeat(30), 20),
        format!("{}…", "a".repeat(19))
    );
    assert_eq!(truncate("ééééé", 3), "éé…");
}

#[test]
fn plain_chars_are_sent_as_utf8() {
    assert_eq!(
        encode(KeyCode::Char('a'), KeyModifiers::NONE).unwrap(),
        b"a"
    );
    assert_eq!(
        encode(KeyCode::Char('é'), KeyModifiers::NONE).unwrap(),
        "é".as_bytes()
    );
}

#[test]
fn ctrl_chars_become_control_codes() {
    let ctrl = |c| encode(KeyCode::Char(c), KeyModifiers::CONTROL);
    assert_eq!(ctrl('a').unwrap(), [0x01]);
    assert_eq!(ctrl('c').unwrap(), [0x03]);
    assert_eq!(ctrl('C').unwrap(), [0x03]);
    assert_eq!(ctrl('z').unwrap(), [0x1a]);
    assert_eq!(ctrl(' ').unwrap(), [0x00]);
    assert_eq!(ctrl('[').unwrap(), [0x1b]);
    assert_eq!(ctrl('1'), None);
}

#[test]
fn editing_keys() {
    assert_eq!(encode(KeyCode::Enter, KeyModifiers::NONE).unwrap(), b"\r");
    assert_eq!(encode(KeyCode::Tab, KeyModifiers::NONE).unwrap(), b"\t");
    assert_eq!(
        encode(KeyCode::BackTab, KeyModifiers::SHIFT).unwrap(),
        b"\x1b[Z"
    );
    assert_eq!(
        encode(KeyCode::Backspace, KeyModifiers::NONE).unwrap(),
        [0x7f]
    );
    assert_eq!(encode(KeyCode::Esc, KeyModifiers::NONE).unwrap(), [0x1b]);
    assert_eq!(
        encode(KeyCode::Delete, KeyModifiers::NONE).unwrap(),
        b"\x1b[3~"
    );
}

#[test]
fn cursor_keys_follow_application_cursor_mode() {
    let up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(encode_key(up, false).unwrap(), b"\x1b[A");
    assert_eq!(encode_key(up, true).unwrap(), b"\x1bOA");
    let end = KeyEvent::new(KeyCode::End, KeyModifiers::NONE);
    assert_eq!(encode_key(end, false).unwrap(), b"\x1b[F");
    assert_eq!(encode_key(end, true).unwrap(), b"\x1bOF");
}

#[test]
fn function_keys() {
    let f = |n| encode(KeyCode::F(n), KeyModifiers::NONE);
    assert_eq!(f(1).unwrap(), b"\x1bOP");
    assert_eq!(f(4).unwrap(), b"\x1bOS");
    assert_eq!(f(5).unwrap(), b"\x1b[15~");
    assert_eq!(f(6).unwrap(), b"\x1b[17~");
    assert_eq!(f(12).unwrap(), b"\x1b[24~");
    assert_eq!(f(13), None);
}

#[test]
fn alt_prefixes_an_escape() {
    assert_eq!(
        encode(KeyCode::Char('x'), KeyModifiers::ALT).unwrap(),
        b"\x1bx"
    );
    assert_eq!(
        encode(KeyCode::Up, KeyModifiers::ALT).unwrap(),
        b"\x1b\x1b[A"
    );
}

#[test]
fn keys_without_an_encoding_are_dropped() {
    assert_eq!(encode(KeyCode::CapsLock, KeyModifiers::NONE), None);
}
