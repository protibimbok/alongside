//! Every command's output in one log, each line labelled with its command.

mod splitter;

use std::collections::VecDeque;

use crossterm::style::SetForegroundColor;
use ratatui::backend::IntoCrossterm;
use ratatui::style::Color;
use ratatui::text::Span;

use crate::size::PaneSize;
use crate::view::{SCROLLBACK, View};
use splitter::Splitter;

const RESET: &[u8] = b"\x1b[m";

pub struct Stream {
    view: View,
    sources: Vec<Source>,
    /// Oldest first. Kept so the view can be drawn again.
    lines: VecDeque<Line>,
    drawn: bool,
}

struct Source {
    /// The command's name in its color, padded to the longest name.
    label: Vec<u8>,
    splitter: Splitter,
    hidden: bool,
}

struct Line {
    source: usize,
    text: Vec<u8>,
}

impl Stream {
    pub fn new<'a>(commands: impl IntoIterator<Item = (&'a str, Color)>, size: PaneSize) -> Self {
        let commands: Vec<_> = commands.into_iter().collect();
        let width = |name: &str| Span::raw(name).width();
        let longest = commands.iter().map(|(name, _)| width(name)).max();
        let sources = commands
            .into_iter()
            .map(|(name, color)| {
                let pad = longest.unwrap_or(0) - width(name);
                let color = SetForegroundColor(color.into_crossterm());
                Source {
                    label: format!("{color}{name}{:pad$}\x1b[m \x1b[2m│\x1b[m ", "").into_bytes(),
                    splitter: Splitter::default(),
                    hidden: false,
                }
            })
            .collect();
        Self {
            view: View::new(size),
            sources,
            lines: VecDeque::new(),
            drawn: false,
        }
    }

    pub fn view(&self) -> &View {
        &self.view
    }

    pub fn view_mut(&mut self) -> &mut View {
        &mut self.view
    }

    pub fn push(&mut self, source: usize, bytes: &[u8]) {
        let mut lines = Vec::new();
        self.sources[source]
            .splitter
            .feed(bytes, |text| lines.push(text));
        self.append(source, lines);
    }

    /// Ends the line command `source` was in the middle of, and adds `note`
    pub fn end(&mut self, source: usize, note: &str) {
        let mut lines: Vec<_> = self.sources[source].splitter.finish().into_iter().collect();
        lines.push(format!("\x1b[2m{note}").into_bytes());
        self.append(source, lines);
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.redraw();
    }

    pub fn resize(&mut self, size: PaneSize) {
        if size == self.view.size() {
            return;
        }
        let at = self.view.screen().scrollback();
        self.view = View::new(size);
        self.redraw();
        self.view.scroll_to(at);
    }

    pub fn is_hidden(&self, source: usize) -> bool {
        self.sources[source].hidden
    }

    pub fn is_filtered(&self) -> bool {
        self.sources.iter().any(|source| source.hidden)
    }

    /// Hides or shows the lines of command `source`. The last command still
    /// shown can't be hidden.
    pub fn toggle(&mut self, source: usize) -> bool {
        let shown = self.sources.iter().filter(|s| !s.hidden).count();
        let Some(toggled) = self.sources.get_mut(source) else {
            return false;
        };
        if !toggled.hidden && shown == 1 {
            return false;
        }
        toggled.hidden = !toggled.hidden;
        self.redraw();
        true
    }

    pub fn formatted(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for line in &self.lines {
            let source = &self.sources[line.source];
            if !source.hidden {
                bytes.extend(&source.label);
                bytes.extend(&line.text);
                bytes.extend(RESET);
                bytes.push(b'\n');
            }
        }
        bytes
    }

    fn append(&mut self, source: usize, lines: Vec<Vec<u8>>) {
        let mut bytes = Vec::new();
        for text in lines {
            let line = Line { source, text };
            Self::format(&self.sources, &mut self.drawn, &line, &mut bytes);
            self.lines.push_back(line);
        }
        let excess = self.lines.len().saturating_sub(SCROLLBACK);
        self.lines.drain(..excess);
        self.view.process(&bytes);
    }

    fn redraw(&mut self) {
        self.view = View::new(self.view.size());
        self.drawn = false;
        let mut bytes = Vec::new();
        for line in &self.lines {
            Self::format(&self.sources, &mut self.drawn, line, &mut bytes);
        }
        self.view.process(&bytes);
    }

    /// Writes `line` after what is drawn already, unless its command is hidden.
    fn format(sources: &[Source], drawn: &mut bool, line: &Line, bytes: &mut Vec<u8>) {
        let source = &sources[line.source];
        if source.hidden {
            return;
        }
        // The line break goes before the line, not after it, so the view
        // doesn't end in an empty row.
        if std::mem::replace(drawn, true) {
            bytes.extend(b"\r\n");
        }
        bytes.extend(&source.label);
        bytes.extend(&line.text);
        bytes.extend(RESET);
    }
}

#[cfg(test)]
mod tests;
