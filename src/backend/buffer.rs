use super::highlight::{Highlighter, TextHighlighting};
use std::cell::RefCell;
use std::fmt;
use std::iter;
use std::str::Bytes;
use unicode_segmentation::UnicodeSegmentation;

pub struct Buffer {
    lines: Vec<Line>,
    pub file_name: Option<String>,
    pub file_type: Option<String>,
    pub dirty: bool,
    pub is_norm: bool,
    pub highlighter: Option<RefCell<Box<dyn Highlighter>>>,
}

impl fmt::Debug for Buffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Buffer[lines: {:?} file_name: {:?}, file_type: {:?}, dirty: {:?}, is normal: {:?}",
            self.lines, self.file_name, self.file_type, self.dirty, self.is_norm
        )
    }
}

impl Buffer {
    pub fn new(
        lines: Vec<Line>,
        is_norm: bool,
        file_name: Option<String>,
        highlighter: Option<RefCell<Box<dyn Highlighter>>>,
    ) -> Self {
        Buffer {
            lines,
            is_norm,
            file_name,
            file_type: None, // TODO
            dirty: false,
            highlighter,
        }
    }
    pub fn clear(&mut self) {
        self.lines = vec![Line::default()];
    }
    pub fn insert_char(&mut self, row: usize, col: usize, g: &str) {
        debug_assert!(self.lines() > row);
        let line = &mut self.lines[row];
        line.insert_grapheme(col, g);
        for line in &mut self.lines[row + 1..] {
            line.offset += 1;
        }
    }
    pub fn delete_char(&mut self, row: usize, col: usize) {
        debug_assert!(col != 0);
        if let Some(line) = self.lines.get_mut(row) {
            line.remove(col - 1);
        }
        for line in &mut self.lines[row + 1..] {
            line.offset -= 1;
        }
    }
    pub fn get(&self, index: usize) -> Option<&Line> {
        self.lines.get(index)
    }
    pub fn merge_with_above(&mut self, index: usize) {
        debug_assert!(index < self.lines());
        if index == 0 {
            return;
        }
        let line = &self.lines[index].clone();
        let prev = self.lines.get_mut(index - 1).unwrap();
        prev.merge(&line);
        self.lines.remove(index);
        for line in &mut self.lines[index..] {
            line.offset -= 1;
        }
    }
    pub fn as_slice(&self) -> &[Line] {
        &self.lines[..]
    }
    pub fn is_empty(&self) -> bool {
        self.lines() == 0 && self.lines[0] == Line::default()
    }
    pub fn lines(&self) -> usize {
        self.lines.len()
    }
    pub fn to_chunk(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        for line in &self.lines {
            buffer.reserve(line.len());
            for byte in line.bytes() {
                buffer.push(byte);
            }
            buffer.push(b'\n');
        }
        buffer
    }
    pub fn is_norm(&self) -> bool {
        self.is_norm
    }

    pub fn highlight(&self) -> Option<TextHighlighting> {
        let mut h = self.highlighter.as_ref()?.borrow_mut();
        Some(h.highlight(self))
    }

    // take a byte offset from the start of the buffer and produce a (row, col) position
    // this is relatively unoptimized: it is currently O(n) but could be O(log n) using binary
    // search (the lines are guaranteed to be sorted by offset)
    pub fn to_pos(&self, offset: usize) -> (usize, usize) {
        for (i, line) in self.lines.iter().enumerate().rev() {
            if line.offset <= offset {
                return (i, offset - line.offset);
            }
        }
        if let Some(last) = self.lines.last() {
            (self.lines.len() - 1, last.offset + last.bytes().len())
        } else {
            (0, 0)
        }
    }

    pub fn append_string(&mut self, s: String) {
        if let Some(last) = self.lines.last() {
            let new_offset = last.offset + last.text.len() + 1; // the +1 is to account for the newline
            self.lines.push(Line::new(s, new_offset));
        } else {
            self.lines.push(Line::new(s, 0));
        }
    }

    pub fn split_line(&mut self, index: usize, split_col: usize) {
        debug_assert!(index < self.lines());
        let line = &mut self.lines[index];
        let rest = line.split_at(split_col);
        self.lines.insert(index + 1, rest);
        for line in &mut self.lines[index + 1..] {
            line.offset += 1;
        }
    }

    pub fn from_bytes(bytes: &[u8], file_name: Option<String>, highlighter: Option<RefCell<Box<dyn Highlighter>>>) -> Self {
        let mut lines = Vec::new();
        bytes.split(|b| *b == b'\n').fold(0, |offset, line_bytes| {
            let line = String::from_utf8_lossy(line_bytes).to_string();
            lines.push(Line::new(line, offset));
            offset + lines.last().unwrap().text.len()
        });
        Buffer {
            dirty: false,
            file_name,
            file_type: None,
            highlighter,
            is_norm: true,
            lines,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Line {
    text: String,
    graphemes: usize,
    // offset is the starting position of the line from the start of the file.
    // This includes newlines (which aren't included in Line::text), so calculating this is prone
    // to off by 1 errors
    offset: usize,
}

impl Line {
    pub fn new(text: String, offset: usize) -> Line {
        let graphemes = text[..].graphemes(true).count();
        Line {
            text,
            graphemes,
            offset,
        }
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
            Line::new(String::new(), self.offset + self.len())
        } else {
            let rest = self.text[..].graphemes(true).skip(idx).collect();
            let g_idx = self.to_byte_idx(idx);
            self.text.truncate(g_idx);
            let new_len = self.graphemes - idx;
            self.graphemes = idx;
            Line {
                text: rest,
                graphemes: new_len,
                offset: g_idx,
            }
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
        self.text[..]
            .grapheme_indices(true)
            .enumerate()
            .fold(None, |acc, (i, (g, _))| {
                acc.or(if i == idx { Some(g) } else { None })
            })
            .unwrap_or(0)
    }

    pub fn skip(&self, idx: usize) -> &str {
        debug_assert!(idx <= self.graphemes);
        let g_idx = self.to_byte_idx(idx);
        &self.text[g_idx..]
    }

    pub fn len(&self) -> usize {
        self.graphemes
    }

    pub fn merge(&mut self, other: &Self) {
        self.text.extend(other.text[..].graphemes(true));
        self.graphemes += other.graphemes;
    }

    pub fn bytes(&self) -> Bytes {
        self.text.bytes()
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
        l.insert_grapheme(0, "\u{2606}");
        l.insert_grapheme(0, "!");
        l.insert_grapheme(2, "!");
        assert_eq!(l.graphemes, 3);
        assert_eq!(l.text, String::from("!\u{2606}!"));
    }

    #[test]
    fn split() {
        let mut l = Line::new(String::from("abcdefg"), 0);
        let rest = l.split_at(4);

        assert_eq!(l, Line::new(String::from("abcd"), 0));
        assert_eq!(rest, Line::new(String::from("efg"), 0));

        let mut l = Line::new(String::from("\u{2606}bcd\u{2606}fg"), 0);
        let rest = l.split_at(4);
        assert_eq!(l, Line::new(String::from("\u{2606}bcd"), 0));
        assert_eq!(rest, Line::new(String::from("\u{2606}fg"), 0));
    }

    #[test]
    fn remove() {
        let mut l = Line::new(String::from("abcdefg"), 0);
        l.remove(3);
        assert_eq!(l, Line::new(String::from("abcefg"), 0));
    }

    #[test]
    fn to_byte_index() {
        let l = Line::new(String::from("abc"), 0);
        assert_eq!(l.to_byte_idx(2), 2);
        let l = Line::new(String::from("\u{2606}bc"), 0);
        assert_eq!(l.to_byte_idx(2), 4);
    }
}
