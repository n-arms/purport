#![warn(unsafe_code)]

use super::cursor::*;
use super::editor::*;
use crate::frontend::ui::Colour;

#[derive(Clone, Debug, Default)]
pub struct Pane {
    pub buffer: usize,
    pub width: usize,
    pub height: usize,
    pub offset: Offset,
    pub cursor: Cursor,
}

#[derive(Debug, Copy, Clone)]
pub enum Char {
    Normal(char),
    Foreground(Colour),
    Background(Colour),
}

impl Pane {
    pub fn move_cursor_left_right(&mut self, buffers: &[Buffer], dist: isize) -> Option<()> {
        let buffer = buffers.get(self.buffer)?;
        self.cursor
            .move_left_right(buffer, &mut self.offset, &self.width, dist);
        Some(())
    }

    pub fn move_cursor_up_down(&mut self, buffers: &[Buffer], dist: isize) -> Option<()> {
        let buffer = buffers.get(self.buffer)?;
        self.cursor
            .move_up_down(buffer, &mut self.offset, &self.height, dist);
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
                .take(self.height - 1)
                .map(move |line| line.iter().skip(first_col).take(width - 2)),
            row: 0,
            max_row: self.height,
            max_col: self.width,
            status_bar: StatusBar {
                file_name: buffer.file_name.as_ref().cloned().unwrap_or(String::from("[No Name]")),
                line: self.cursor.row,
                lines: buffer.lines.len()
            }
        })
    }
}

#[derive(Debug, Clone)]
pub struct StatusBar {
    file_name: String,
    line: usize,
    lines: usize
}

pub struct TextIter<'a, L: Iterator<Item = &'a char>, Text: Iterator<Item = L>> {
    text: Text,
    row: usize,
    max_row: usize,
    max_col: usize,
    status_bar: StatusBar
}

impl<'a, L, Text> Iterator for TextIter<'a, L, Text>
where
    L: Iterator<Item = &'a char>,
    Text: Iterator<Item = L>,
{
    type Item = LineIter<'a, L>;
    fn next(&mut self) -> Option<Self::Item> {
        if (self.max_row - 1) > self.row {
            self.row += 1;
            Some(LineIter {
                max_col: self.max_col,
                col: 0,
                line: self.text.next().map(|l| Line::Normal(0, l)).unwrap_or(Line::Empty),
            })
        } else if self.max_row - 1 == self.row {
            self.row += 1;
            Some(LineIter {
                max_col: self.max_col,
                col: 0,
                line: Line::StatusBar(self.status_bar.clone())
            })
        } else {
            None
        }
    }
}

pub enum Line<'a, L: Iterator<Item = &'a char>> {
    StatusBar(StatusBar),
    Normal(u8, L),
    Empty
}

// to be used with syntax highlighting later: we want to reuse the memory for each line, (hence
// holding an iterator instead of just collecting, but copying over the start/end points for
// colours doesnt seem too crazy (although it is still O(n))
pub struct LineIter<'a, L: Iterator<Item = &'a char>> {
    max_col: usize,
    col: usize,
    line: Line<'a, L>
}


impl<'a, L: Iterator<Item = &'a char>> Iterator for LineIter<'a, L> {
    type Item = Char;
    fn next(&mut self) -> Option<Self::Item> {
        if self.max_col > self.col {
            self.col += 1;
            let c = match &mut self.line {
                Line::Empty => Char::Normal(' '),
                Line::Normal(i @ 0, _) => {
                    *i += 1;
                    Char::Normal('~')
                }
                Line::Normal(i @ 1, _) => {
                    *i += 1;
                    Char::Normal(' ')
                }
                Line::Normal(_, l) => Char::Normal(l.next().copied().unwrap_or(' ')),
                Line::StatusBar(StatusBar {file_name, line, lines}) => {
                    let line_str = line.to_string();
                    let lines_str = lines.to_string();
                    if self.col == 1 {
                        Char::Background(Colour::Red)
                    } else if self.col - 1 <= file_name.len() {
                        Char::Normal(file_name.chars().nth(self.col - 2).unwrap())
                    } else if self.col - 2 == file_name.len() {
                        Char::Normal(' ')
                    } else if self.col - file_name.len() - 3 < line_str.len() {
                        Char::Normal(line_str.chars().nth(self.col - file_name.len() - 3).unwrap())
                    } else if self.col - file_name.len() - line_str.len() == 3 {
                        Char::Normal(':')
                    } else if self.col - file_name.len() - line_str.len() - 4 < lines_str.len() {
                        Char::Normal(lines_str.chars().nth(self.col - file_name.len() - line_str.len() - 4).unwrap())
                    } else {
                        Char::Normal(' ')
                    }
                }
            };
            Some(c)
        } else if self.max_col == self.col && matches!(self.line, Line::StatusBar(_)) {
            self.col += 1;
            Some(Char::Background(Colour::Reset))
        } else {
            None
        }
    }
}
