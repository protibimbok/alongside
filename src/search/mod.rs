#[derive(Default)]
pub struct Search {
    query: String,
    needle: Vec<char>,
    matches: Vec<usize>,
    current: usize,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CurrentMatch {
    pub line: usize,
    pub nth_of_line: usize,
}

impl Search {
    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn is_active(&self) -> bool {
        !self.query.is_empty()
    }

    pub fn count(&self) -> usize {
        self.matches.len()
    }

    pub fn position(&self) -> usize {
        self.current + 1
    }

    pub fn current(&self) -> Option<CurrentMatch> {
        let line = *self.matches.get(self.current)?;
        let first = self.matches.partition_point(|&l| l < line);
        Some(CurrentMatch {
            line,
            nth_of_line: self.current - first,
        })
    }

    pub fn push(&mut self, c: char) {
        self.query.push(c);
        self.needle.push(fold(c));
    }

    pub fn pop(&mut self) {
        self.query.pop();
        self.needle.pop();
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn update(&mut self, lines: &[String]) {
        self.matches.clear();
        for (i, line) in lines.iter().enumerate() {
            let line: Vec<char> = line.chars().map(fold).collect();
            self.matches.extend(find(&line, &self.needle).map(|_| i));
        }
        self.current = self.current.min(self.matches.len().saturating_sub(1));
    }

    pub fn select_last(&mut self) -> Option<usize> {
        self.current = self.matches.len().checked_sub(1)?;
        Some(self.matches[self.current])
    }

    pub fn step(&mut self, forward: bool) -> Option<usize> {
        let count = self.matches.len();
        if count == 0 {
            return None;
        }
        let step = if forward { 1 } else { count - 1 };
        self.current = (self.current + step) % count;
        Some(self.matches[self.current])
    }

    /// Start of every match in `line`, which must be case-folded.
    pub fn find_in<'a>(&'a self, line: &'a [char]) -> impl Iterator<Item = usize> + 'a {
        find(line, &self.needle)
    }

    pub fn len(&self) -> usize {
        self.needle.len()
    }
}

/// Start of every non-overlapping occurrence of `needle` in `line`.
fn find<'a>(line: &'a [char], needle: &'a [char]) -> impl Iterator<Item = usize> + 'a {
    let len = needle.len();
    let mut at = 0;
    std::iter::from_fn(move || {
        if len == 0 {
            return None;
        }
        while at + len <= line.len() {
            if line[at..at + len] == *needle {
                at += len;
                return Some(at - len);
            }
            at += 1;
        }
        None
    })
}

/// Folds case one char to one char, so positions in the folded text are
/// positions in the original.
pub fn fold(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

#[cfg(test)]
mod tests;
