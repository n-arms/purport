#![warn(unsafe_code)]

use super::editor::*;
use crate::frontend::ui::Colour;
use std::io::Write;

#[derive(Clone, Debug, Default)]
pub struct Pane {
    pub buffer: usize,
    pub first_row: usize,
    pub first_col: usize,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub width: usize,
    pub height: usize,
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
        let first_col = self.first_col;
        let width = self.width;
        Some(TextIter {
            text: buffer
                .lines
                .iter()
                .skip(self.first_row)
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
        if let Some(line) = buffer.lines.get(self.cursor_row) {
            if dist > 0 {
                // the distance from the screen pos of the cursor to the right edge of the screen
                let old_edge_dist = self.width - (self.cursor_col - self.first_col);
                // the distance the cursor needs to be moved to the right
                let dist_right = (dist as usize).min(line.len() - self.cursor_col);
                // if the distance to be moved is more than the available space to move
                if old_edge_dist <= dist_right {
                    self.first_col += 1 + dist_right - old_edge_dist; // scroll by the overflow
                }
                self.cursor_col += dist_right;
                log!("cursor col = {}", self.cursor_col);
            } else {
                // the distance from the screen pos of the cursor to the left edge of the screen
                let old_edge_dist = self.cursor_col - self.first_col;
                // the distance the cursor needs to be moved to the left
                let dist_left = -(dist.max(-(self.cursor_col as isize))) as usize;
                // if the distance to be moved is more than the available space to move
                if old_edge_dist < dist_left {
                    self.first_col -= dist_left - old_edge_dist // scroll by the overflow
                }
                self.cursor_col -= dist_left;
            }
        }
        Some(())
    }

    pub fn move_cursor_up_down(&mut self, buffers: &[Buffer], dist: isize) -> Option<()> {
        let buffer = buffers.get(self.buffer)?;
        if dist > 0 {
            let old_edge_dist = self.height - (self.cursor_row - self.first_row);
            let dist_down = (dist as usize).min(buffer.lines.len() - self.cursor_row - 1);
            if old_edge_dist <= dist_down {
                self.first_row += 1 + dist_down - old_edge_dist;
            }
            self.cursor_row += dist_down;
            let new_line_len = buffer.lines.get(self.cursor_row).map(|line| line.len()).unwrap_or(0);
            if new_line_len < self.cursor_col {
                self.cursor_col = new_line_len;
            }
        } else {
            let old_edge_dist = self.cursor_row - self.first_row;
            let dist_up = -(dist.max(-(self.cursor_row as isize))) as usize;
            if old_edge_dist <= dist_up {
                self.first_row -= dist_up - old_edge_dist;
            }
            self.cursor_row -= dist_up;
            let new_line_len = buffer.lines.get(self.cursor_row).map(|line| line.len()).unwrap_or(0);
            if new_line_len < self.cursor_col {
                self.cursor_col = new_line_len;
            }
        }
        Some(())
    }
    pub unsafe fn set_cursor(&mut self, row: usize, col: usize) {
        self.cursor_col = col;
        self.cursor_row = row;
    }

    pub fn insert_char(&mut self, buffers: &mut [Buffer], c: char) -> Option<()> {
        if c == '\r' {
            let buffer = buffers.get_mut(self.buffer)?;
            if let Some(line) = buffer.lines.get_mut(self.cursor_row) {
                let rest = line[self.cursor_col..].to_vec();
                line.truncate(self.cursor_col);
                buffer.lines.insert(self.cursor_row + 1, rest);
                unsafe {
                    self.set_cursor(self.cursor_row + 1, 0);
                }
            } else {
                buffer.lines.push(Vec::new());
                self.move_cursor_up_down(buffers, 1);
            }
            Some(())
        } else {
            let buffer = buffers.get_mut(self.buffer)?;
            if let Some(line) = buffer.lines.get_mut(self.cursor_row) {
                let rest = &line[self.cursor_col..].to_owned();
                line.truncate(self.cursor_col);
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
        if let Some(line) = buffer.lines.get_mut(self.cursor_row) {
            if self.cursor_col == 0 && self.cursor_row == 0 {
                Some(())
            } else if self.cursor_col == 0 {
                let old = line.to_owned();
                if let Some(prev) = buffer.lines.get_mut(self.cursor_row - 1) {
                    unsafe {
                        self.set_cursor(self.cursor_row - 1, prev.len());
                    }
                    prev.extend(old.iter());
                    buffer.lines.remove(self.cursor_row + 1);
                    Some(())
                } else {
                    Some(())
                }
            } else {
                line.remove(self.cursor_col - 1);
                self.move_cursor_left_right(buffers, -1)
            }
        } else {
            self.move_cursor_up_down(buffers, -1)
        }
    }
}
