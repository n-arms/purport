#![warn(unsafe_code)]

use super::editor::*;
use crate::frontend::ui::Colour;
use std::io::Write;
use super::cursor::*;

#[derive(Clone, Debug, Default)]
pub struct Pane {
    pub buffer: usize,
    pub width: usize,
    pub height: usize,
    pub offset: Offset,
    pub cursor: Cursor
}

#[derive(Debug, Copy, Clone)]
pub enum Char {
    Normal(char),
    Foreground(Colour),
    Background(Colour),
}

pub struct TextIter<'a, Line: Iterator<Item = &'a char>, Text: Iterator<Item = Line>> {
    text: Text,
    row: usize,
    max_row: usize,
    max_col: usize
}

impl<'a, Line, Text> Iterator for TextIter<'a, Line, Text>
where 
    Line: Iterator<Item = &'a char>,
    Text: Iterator<Item = Line>

{
    type Item = LineIter<'a, Line>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.max_row > self.row {
            self.row += 1;
            Some(LineIter {
                max_col: self.max_col,
                col: 0,
                line: self.text.next()
            })
        } else {
            None
        }
    }
}
// to be used with syntax highlighting later: we want to reuse the memory for each line, (hence
// holding an iterator instead of just collecting, but copying over the start/end points for
// colours doesnt seem too crazy (although it is still O(n))
pub struct LineIter<'a, Line: Iterator<Item = &'a char>> {
    max_col: usize,
    col: usize,
    line: Option<Line>
}

impl<'a, Line: Iterator<Item = &'a char>> Iterator for LineIter<'a, Line> {
    type Item = Char;
    fn next(&mut self) -> Option<Self::Item> {
        if self.max_col > self.col {
            self.col += 1;
            let c = self.line.as_mut().and_then(|line| line.next().copied()).unwrap_or(' ');
            Some(Char::Normal(c))
        } else {
            None
        }
    }
}

impl Pane {
    pub fn display<'a>(
        &self,
        buffers: &'a [Buffer],
    ) -> Option<impl Iterator<Item = impl Iterator<Item = Char> + 'a> + 'a> {
        let buffer = buffers.get(self.buffer)?;
        let first_col = self.offset.col;
        let width = self.width;
        Some(TextIter {
            text: buffer
                .lines
                .iter()
                .skip(self.offset.row)
                .take(self.height)
                .map(move |line| line.iter().skip(first_col).take(width)),
            row: 0,
            max_row: self.height,
            max_col: self.width
        })
    }
//abX
//len = 3
//cursor = 2
//3 - 2
    pub fn move_cursor_left_right(&mut self, buffers: &[Buffer], dist: isize) -> Option<()> {
        let buffer = buffers.get(self.buffer)?;
        self.cursor.move_left_right(buffer, &mut self.offset, &self.width, dist);
        Some(())
    }

    pub fn move_cursor_up_down(&mut self, buffers: &[Buffer], dist: isize) -> Option<()> {
        let buffer = buffers.get(self.buffer)?;
        self.cursor.move_up_down(buffer, &mut self.offset, &self.height, dist);
        Some(())
    }
    pub unsafe fn set_cursor(&mut self, row: usize, col: usize) {
        self.cursor.col = col;
        self.cursor.row = row;
    }

    pub fn insert_char(&mut self, buffers: &mut [Buffer], c: char) -> Option<()> {
        if c == '\r' {
            let buffer = buffers.get_mut(self.buffer)?;
            if let Some(line) = buffer.lines.get_mut(self.cursor.row) {
                let rest = line[self.cursor.col..].to_vec();
                line.truncate(self.cursor.col);
                buffer.lines.insert(self.cursor.row + 1, rest);
                unsafe {
                    self.set_cursor(self.cursor.row + 1, 0);
                }
            } else {
                buffer.lines.push(Vec::new());
                self.move_cursor_up_down(buffers, 1);
            }
            Some(())
        } else {
            let buffer = buffers.get_mut(self.buffer)?;
            if let Some(line) = buffer.lines.get_mut(self.cursor.row) {
                let rest = &line[self.cursor.col..].to_owned();
                line.truncate(self.cursor.col);
                line.push(c);
                line.extend(rest);
                self.move_cursor_left_right(buffers, 1);
            } else {
                buffer.lines.push(vec![c]);
            }

            Some(())
        }
    }

    pub fn backspace(&mut self, buffers: &mut [Buffer]) -> Option<()> {
        let buffer = buffers.get_mut(self.buffer)?;
        if let Some(line) = buffer.lines.get_mut(self.cursor.row) {
            if self.cursor.col == 0 && self.cursor.row == 0 {
                Some(())
            } else if self.cursor.col == 0 {
                let old = line.to_owned();
                if let Some(prev) = buffer.lines.get_mut(self.cursor.row - 1) {
                    unsafe {
                        self.set_cursor(self.cursor.row - 1, prev.len());
                    }
                    prev.extend(old.iter());
                    buffer.lines.remove(self.cursor.row + 1);
                    Some(())
                } else {
                    Some(())
                }
            } else {
                line.remove(self.cursor.col - 1);
                self.move_cursor_left_right(buffers, -1)
            }
        } else {
            self.move_cursor_up_down(buffers, -1)
        }
    }
}
