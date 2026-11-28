use super::*;

fn search(query: &str, lines: &[&str]) -> Search {
    let mut search = Search::default();
    query.chars().for_each(|c| search.push(c));
    let lines: Vec<String> = lines.iter().map(ToString::to_string).collect();
    search.update(&lines);
    search
}

fn at(line: usize, nth_of_line: usize) -> CurrentMatch {
    CurrentMatch { line, nth_of_line }
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
    assert_eq!(search.current(), Some(at(2, 1)));
    assert_eq!(search.step(true), Some(0));
    assert_eq!(search.current(), Some(at(0, 0)));
    assert_eq!(search.step(false), Some(2));
    assert_eq!(search.step(false), Some(2));
    assert_eq!(search.current(), Some(at(2, 0)));
    assert_eq!(search.position(), 2);
}

#[test]
fn editing_the_query_keeps_the_current_match_in_range() {
    let mut search = search("a", &["a", "ab"]);
    search.select_last();
    search.push('b');
    search.update(&["a".to_string(), "ab".to_string()]);
    assert_eq!(search.current(), Some(at(1, 0)));
    search.pop();
    assert_eq!(search.query(), "a");
}
