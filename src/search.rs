#[derive(Default)]
pub struct Search {
    query: String,
    /// The query, case-folded.
    needle: Vec<char>,
    /// The line of every match, top to bottom. A line with several matches
    /// is listed once per match.
    lines: Vec<usize>,
    current: usize,
}

impl Search {
    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn is_active(&self) -> bool {
        !self.query.is_empty()
    }

    pub fn count(&self) -> usize {
        self.lines.len()
    }

    pub fn position(&self) -> usize {
        self.current + 1
    }

    /// The current match, as its line and its index among that line's matches.
    pub fn current(&self) -> Option<(usize, usize)> {
        let line = *self.lines.get(self.current)?;
        let first = self.lines.partition_point(|&l| l < line);
        Some((line, self.current - first))
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

    /// Finds the query again in `lines`, the pane's output from the top.
    pub fn update(&mut self, lines: &[String]) {
        self.lines.clear();
        for (i, line) in lines.iter().enumerate() {
            let line: Vec<char> = line.chars().map(fold).collect();
            self.lines.extend(find(&line, &self.needle).map(|_| i));
        }
        self.current = self.current.min(self.lines.len().saturating_sub(1));
    }

    /// Selects the most recent match and returns its line.
    pub fn select_last(&mut self) -> Option<usize> {
        self.current = self.lines.len().checked_sub(1)?;
        Some(self.lines[self.current])
    }

    /// Moves to the next or previous match, wrapping around, and returns its line.
    pub fn step(&mut self, forward: bool) -> Option<usize> {
        let count = self.lines.len();
        if count == 0 {
            return None;
        }
        let step = if forward { 1 } else { count - 1 };
        self.current = (self.current + step) % count;
        Some(self.lines[self.current])
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
mod tests {
    use super::*;

    fn search(query: &str, lines: &[&str]) -> Search {
        let mut search = Search::default();
        query.chars().for_each(|c| search.push(c));
        let lines: Vec<String> = lines.iter().map(ToString::to_string).collect();
        search.update(&lines);
        search
    }

    #[test]
    fn matches_ignore_case_and_do_not_overlap() {
        let search = search("aa", &[]);
        let line: Vec<char> = "AaaA aa".chars().map(fold).collect();
        assert_eq!(search.find_in(&line).collect::<Vec<_>>(), [0, 2, 5]);
    }

    #[test]
    fn an_empty_query_matches_nothing() {
        let search = search("", &["anything"]);
        assert_eq!(search.count(), 0);
        assert_eq!(search.current(), None);
    }

    #[test]
    fn stepping_wraps_around_in_both_directions() {
        let mut search = search("x", &["x", "", "x x"]);
        assert_eq!(search.count(), 3);
        assert_eq!(search.select_last(), Some(2));
        assert_eq!(search.current(), Some((2, 1)));
        assert_eq!(search.step(true), Some(0));
        assert_eq!(search.current(), Some((0, 0)));
        assert_eq!(search.step(false), Some(2));
        assert_eq!(search.step(false), Some(2));
        assert_eq!(search.current(), Some((2, 0)));
        assert_eq!(search.position(), 2);
    }

    #[test]
    fn editing_the_query_keeps_the_current_match_in_range() {
        let mut search = search("a", &["a", "ab"]);
        search.select_last();
        search.push('b');
        search.update(&["a".to_string(), "ab".to_string()]);
        assert_eq!(search.current(), Some((1, 0)));
        search.pop();
        assert_eq!(search.query(), "a");
    }
}
