use unicode_segmentation::UnicodeSegmentation;
use std::iter;
use std::fmt;

#[derive(Clone, Debug)]
pub struct Buffer {
    pub lines: Vec<Line>,
    pub file_name: Option<String>,
    pub dirty: bool,
    pub is_norm: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Line {
    text: String, 
    graphemes: usize
}

impl Line {
    pub fn new(text: String) -> Line {
        let graphemes = text[..].graphemes(true).count();
        Line{text, graphemes}
    }

    pub fn insert_grapheme(&mut self, idx: usize, grapheme: &str) {
        debug_assert!(idx <= self.graphemes);
        self.graphemes += 1;
        if idx == self.graphemes - 1 {
            self.text.push_str(grapheme);
        } else {
            let mut new_row = String::with_capacity(self.text.len() + grapheme.len());
            new_row.extend(self.text[..].graphemes(true).take(idx));
            new_row.extend(grapheme.graphemes(true));
            new_row.extend(self.text[..].graphemes(true).skip(idx));
            self.text = new_row;
        }
    }
    pub fn split_at(&mut self, idx: usize) -> Line {
        debug_assert!(idx <= self.graphemes);
        if idx == self.graphemes {
            Line::new(String::new())
        } else {
            let rest = self.text[..].graphemes(true).skip(idx).collect();
            let g_idx = self.to_byte_idx(idx);
            self.text.truncate(g_idx);
            let new_len = self.graphemes - idx;
            self.graphemes = idx;
            Line{text: rest, graphemes: new_len}
        }
    }
    pub fn remove(&mut self, idx: usize) {
        debug_assert!(idx < self.graphemes);
        debug_assert!(self.graphemes > 0);
        let rest: String = self.text[..].graphemes(true).skip(idx + 1).collect();
        let g_idx = self.to_byte_idx(idx);
        self.text.truncate(g_idx);
        self.text.extend(rest[..].graphemes(true));
        self.graphemes -= 1;
    }

    fn to_byte_idx(&self, idx: usize) -> usize {
        self.text[..].grapheme_indices(true).enumerate()
            .fold(None, |acc, (i, (g, _))| acc.or(if i == idx {Some(g)} else {None})).unwrap()
    }

    pub fn skip<'a>(&'a self, idx: usize) -> &'a str {
        debug_assert!(idx <= self.graphemes);
        let g_idx = self.to_byte_idx(idx);
        &self.text[g_idx..]
    }

    pub fn get<'a>(&'a self, idx: usize) -> Option<&'a str> {
        self.text[..].graphemes(true).nth(idx)
    }

    pub fn len(&self) -> usize {
        self.graphemes
    }

    pub fn merge(&mut self, other: &Self) {
        self.text.extend(other.text[..].graphemes(true));
        self.graphemes += other.graphemes;
    }
}

impl<'a> iter::Extend<&'a str> for Line {
    fn extend<T: IntoIterator<Item = &'a str>>(&mut self, iter: T) {
        for s in iter {
            for g in s.graphemes(true) {
                self.text.extend(g.graphemes(true));
                self.graphemes += 1;
            }
        }
    }
}

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn insert() {
        let mut l = Line::default();
        l.insert_grapheme(0, "a");
        l.insert_grapheme(1, "b");
        assert_eq!(l.graphemes, 2);
        assert_eq!(l.text, String::from("ab"));

        let mut l = Line::default();
        l.insert_grapheme(0, "☆");
        l.insert_grapheme(0, "!");
        l.insert_grapheme(2, "!");
        assert_eq!(l.graphemes, 3);
        assert_eq!(l.text, String::from("!☆!"));
    }

    #[test]
    fn split() {
        let mut l = Line::new(String::from("abcdefg"));
        let rest = l.split_at(4);

        assert_eq!(l, Line::new(String::from("abcd")));
        assert_eq!(rest, Line::new(String::from("efg")));

        let mut l = Line::new(String::from("☆bcd☆fg"));
        let rest = l.split_at(4);
        assert_eq!(l, Line::new(String::from("☆bcd")));
        assert_eq!(rest, Line::new(String::from("☆fg")));
    }

    #[test]
    fn remove() {
        let mut l = Line::new(String::from("abcdefg"));
        l.remove(3);
        assert_eq!(l, Line::new(String::from("abcefg")));
    }

    #[test]
    fn to_byte_index() {
        let l = Line::new(String::from("abc"));
        assert_eq!(l.to_byte_idx(2), 2);
        let l = Line::new(String::from("☆bc"));
        assert_eq!(l.to_byte_idx(2), 4);
    }
}
