use super::*;
use log::*;
use crate::*;

use ropey::Rope;

struct SearchFsm<'a> {
    needle: &'a str,
    count: usize,
    chars: std::str::Chars<'a>,
    n: Option<char>,
    start: usize,
}

impl<'a> SearchFsm<'a> {
    fn new(needle: &'a str) -> Self {
        let mut chars = needle.chars();
        let n = chars.next();
        Self {
            needle,
            count: 0,
            chars,
            n,
            start: 0,
        }
    }

    fn advance(&mut self, c: usize) -> Option<Substring> {
        self.next(c)
    }

    fn next(&mut self, c: usize) -> Option<Substring> {
        if self.count == 0 {
            self.start = c;
        }
        match self.chars.next() {
            Some(x) => {
                self.n = Some(x);
                self.count += 1;
                None
            }
            None => {
                // reset and return result
                self.reset();
                Some(Substring(self.start, c + 1))
            }
        }
    }

    fn reset(&mut self) {
        self.chars = self.needle.chars();
        self.n = self.chars.next();
        self.count = 0;
    }

    fn add(&mut self, c: usize, ch: char) -> Option<Substring> {
        if Some(ch) == self.n {
            self.advance(c)
        } else {
            self.reset();
            None
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Substring(usize, usize);
impl Substring {
    pub fn start(&self) -> usize {
        self.0
    }
    pub fn end(&self) -> usize {
        self.1
    }
}

#[derive(Debug, Clone)]
pub struct SearchResults {
    results: Vec<Substring>,
    reverse: bool,
}
impl Default for SearchResults {
    fn default() -> Self {
        Self {
            results: Vec::new(),
            reverse: false,
        }
    }
}
impl SearchResults {
    fn new(results: Vec<Substring>, reverse: bool) -> Self {
        Self { results, reverse }
    }

    pub fn new_search(text: &Rope, s: &str, reverse: bool) -> Self {
        let results = search(text, s);
        Self { results, reverse }
    }

    pub fn next_from_position(&self, c: usize, reps: i32) -> Option<Substring> {
        info!("results: {:?}", self.results);
        if self.results.len() == 0 {
            return None;
        }

        let r = if self.reverse { -reps } else { reps };

        // increment and wrap
        let p = (self.results.partition_point(|s| s.start() < c) as i32 + r)
            .rem_euclid(self.results.len() as i32);
        self.results.get(p as usize).map(|p| p.clone())
    }

    pub fn next_cursor(&self, text: &Rope, sx: usize, cursor: &Cursor, reps: i32) -> Cursor {
        let mut c = cursor.c;
        let result;
        result = self.next_from_position(c, reps);

        match result {
            Some(sub) => {
                c = sub.start();
            }
            None => (),
        }
        if c != cursor.c {
            cursor_from_char(text, sx, &cursor.config, c, 0)
        } else {
            cursor.clone()
        }
    }
}

pub fn search(text: &Rope, s: &str) -> Vec<Substring> {
    let c = 0;
    let end = text.len_chars();
    search_range(text, s, c, end)
}

pub fn search_range(text: &Rope, s: &str, start: usize, end: usize) -> Vec<Substring> {
    let mut fsm = SearchFsm::new(s);
    let mut out = Vec::new();
    let mut c = start;
    while c < end {
        match fsm.add(c, text.char(c)) {
            Some(s) => {
                out.push(s);
            }
            None => (),
        }
        c += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_1() {
        let text = Rope::from_str("asdf");
        let result = search(&text, "sd");
        assert_eq!(result, vec![Substring(1, 3)]);
    }

    #[test]
    fn test_search_2() {
        let text = Rope::from_str("asdf");
        let result = search(&text, "asdf");
        assert_eq!(result, vec![Substring(0, 4)]);
    }

    #[test]
    fn test_search_3() {
        let text = Rope::from_str("asdf");
        let result = search(&text, "fasd");
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_search_4() {
        let text = Rope::from_str("_asdf_asdf");
        let result = search(&text, "asdf");
        assert_eq!(result, vec![Substring(1, 5), Substring(6, 10)]);
    }
}
