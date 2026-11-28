use crate::stream::RESET;

pub(super) struct Splitter {
    state: State,
    line: Vec<u8>,
    params: Vec<u8>,
    attributes: vt100::Parser,
    returned: bool,
}

#[derive(Clone, Copy)]
pub(super) enum State {
    Text,
    Escape,
    /// `ESC` and an intermediate byte, as in a charset selection.
    Intermediate,
    /// `ESC [`.
    Control,
    /// A string up to its terminator, such as a window title.
    String,
}

impl Default for Splitter {
    fn default() -> Self {
        Self {
            state: State::Text,
            line: Vec::new(),
            params: Vec::new(),
            attributes: vt100::Parser::new(1, 1, 0),
            returned: false,
        }
    }
}

impl Splitter {
    pub(super) fn feed(&mut self, bytes: &[u8], mut emit: impl FnMut(Vec<u8>)) {
        for &byte in bytes {
            self.state = match self.state {
                State::Text => {
                    self.text(byte, &mut emit);
                    if byte == 0x1b {
                        State::Escape
                    } else {
                        State::Text
                    }
                }
                State::Escape => match byte {
                    b'[' => {
                        self.params.clear();
                        State::Control
                    }
                    b']' | b'P' | b'X' | b'^' | b'_' => State::String,
                    0x20..=0x2f => State::Intermediate,
                    _ => State::Text,
                },
                State::Intermediate => match byte {
                    0x20..=0x2f => State::Intermediate,
                    _ => State::Text,
                },
                State::Control if (0x20..=0x3f).contains(&byte) => {
                    self.params.push(byte);
                    State::Control
                }
                State::Control => {
                    if byte == b'm' {
                        self.set_attributes();
                    }
                    State::Text
                }
                State::String => match byte {
                    0x07 => State::Text,
                    0x1b => State::Escape,
                    _ => State::String,
                },
            };
        }
    }

    fn text(&mut self, byte: u8, emit: &mut impl FnMut(Vec<u8>)) {
        match byte {
            b'\n' => {
                self.returned = false;
                emit(self.take_line());
            }
            b'\r' => self.returned = true,
            b'\t' | 0x20.. => {
                if std::mem::take(&mut self.returned) {
                    self.take_line();
                }
                self.line.push(byte);
            }
            _ => {}
        }
    }

    pub(super) fn finish(&mut self) -> Option<Vec<u8>> {
        let line = std::mem::take(self).line;
        (!line.is_empty()).then_some(line)
    }

    fn set_attributes(&mut self) {
        // Anything else is a private sequence that only happens to end in `m`.
        if !self
            .params
            .iter()
            .all(|byte| matches!(byte, b'0'..=b'9' | b';' | b':'))
        {
            return;
        }
        let mut sequence = b"\x1b[".to_vec();
        sequence.extend(&self.params);
        sequence.push(b'm');
        self.attributes.process(&sequence);
        self.line.extend(sequence);
    }

    /// Takes the line and starts the next one with the attributes in effect.
    fn take_line(&mut self) -> Vec<u8> {
        let active = self.attributes.screen().attributes_formatted();
        let active = if active == RESET { Vec::new() } else { active };
        std::mem::replace(&mut self.line, active)
    }
}
