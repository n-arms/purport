#![warn(unsafe_code)]

use super::editor::*;
use crate::frontend::ui::Colour;

#[derive(Clone, Debug, Default)]
pub struct Pane {
    pub buffer: usize,
    pub first_row: usize,
    pub first_col: usize,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub width: usize,
    pub height: usize
}

#[derive(Debug, Copy, Clone)]
pub enum Char {
    Normal(char),
    Foreground(Colour),
    Background(Colour)
}

pub struct TextIter<'a, Line: Iterator<Item = &'a char>, Text: Iterator<Item = Line>> {
    text: Text,
    cursor_screen_y: usize,
    current_screen_y: usize,
    cursor_screen_x: usize,
    max_screen_y: usize
}

impl<'a, Line: Iterator<Item = &'a char>, Text: Iterator<Item = Line>> Iterator for TextIter<'a, Line, Text> {
    type Item = LineIter<'a, Line>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.max_screen_y > self.current_screen_y {
            if let Some(line) = self.text.next() {
                if self.current_screen_y == self.cursor_screen_y {
                    self.current_screen_y += 1;
                    Some(LineIter::CursorLine {
                        line,
                        cursor_screen_x: self.cursor_screen_x,
                        current_screen_x: 0,
                        cursor_drawn: false,
                        cursor_colour: false
                    })
                } else {
                    self.current_screen_y += 1;
                    Some(LineIter::NormalLine(line))
                }
            } else if self.cursor_screen_y >= self.current_screen_y {
                self.current_screen_y += 1;
                Some(LineIter::EmptyLine(0))
            } else {
                self.current_screen_y += 1;
                Some(LineIter::EmptyLine(3))
            }
        } else {
            None
        }
    }
}

pub enum LineIter<'a, Line: Iterator<Item = &'a char>> {
    CursorLine {
        line: Line,
        cursor_screen_x: usize,
        current_screen_x: usize,
        cursor_drawn: bool,
        cursor_colour: bool,
    },
    NormalLine(Line),
    EmptyLine(u8)
}

impl<'a, Line: Iterator<Item = &'a char>> Iterator for LineIter<'a, Line> {
    type Item = Char;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::CursorLine {line, current_screen_x, cursor_screen_x, cursor_drawn, cursor_colour} => 
                if cursor_screen_x == current_screen_x && !*cursor_colour && !*cursor_drawn {
                    *cursor_colour = true;
                    Some(Char::Background(Colour::White))
                } else if *cursor_colour && !*cursor_drawn {
                    *cursor_drawn = true;
                    *current_screen_x += 1;
                    line.next().map(|c| Char::Normal(*c)).or(Some(Char::Normal(' ')))
                } else if *cursor_colour && *cursor_drawn {
                    *cursor_colour = false;
                    Some(Char::Background(Colour::Reset))
                } else if let Some(c) = line.next() {
                    *current_screen_x += 1;
                    Some(Char::Normal(*c))
                } else if !*cursor_drawn {
                    *cursor_colour = true;
                    Some(Char::Background(Colour::White))
                } else {
                    None
                }
            Self::NormalLine(line) => line.next().map(|c| Char::Normal(*c)),
            Self::EmptyLine(drawn @ 0) => {
                *drawn += 1;
                Some(Char::Background(Colour::White))
            }
            Self::EmptyLine(drawn @ 1) => {
                *drawn += 1;
                Some(Char::Normal(' '))
            }
            Self::EmptyLine(drawn @ 2) => {
                *drawn += 1;
                Some(Char::Background(Colour::Reset))
            }
            Self::EmptyLine(_) => None
        }
    }
}

impl Pane {
    pub fn display<'a>(&self, buffers: &'a [Buffer]) -> Option<impl Iterator<Item = impl Iterator<Item = Char> + 'a> + 'a> {
        let buffer = buffers.get(self.buffer)?;
        let first_col = self.first_col;
        let width = self.width;
        Some(TextIter {
            text: buffer.lines.iter()
                .skip(self.first_row)
                .take(self.height)
                .map(move |line| line.iter()
                     .skip(first_col)
                     .take(width)),
            cursor_screen_x: self.cursor_col - self.first_col,
            cursor_screen_y: self.cursor_row - self.first_row,
            current_screen_y: 0,
            max_screen_y: self.height
        })
    }

    pub fn move_cursor_left_right(&mut self, buffers: &[Buffer], dist: isize) -> Option<()> {
        let buffer = buffers.get(self.buffer)?;
        if let Some(line) = buffer.lines.get(self.cursor_row) {
            if dist > 0 {
                // the distance from the screen pos of the cursor to the right edge of the screen
                let old_edge_dist = self.width - (self.cursor_col - self.first_col); 
                // the distance the cursor needs to be moved to the right
                let dist_right = (dist as usize).min(line.len() - self.cursor_col - 1);
                // if the distance to be moved is more than the available space to move
                if old_edge_dist <= dist_right {
                    self.first_col += 1 + dist_right - old_edge_dist; // scroll by the overflow
                }
                self.cursor_col += dist_right;
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
            log!("height {}", self.height);
            let old_edge_dist = self.height - (self.cursor_row - self.first_row);
            let dist_down = (dist as usize).min(buffer.lines.len() - self.cursor_row);
            if old_edge_dist <= dist_down {
                self.first_row += 1 + dist_down - old_edge_dist;
            }
            self.cursor_row += dist_down;
        } else {
            log!("cursor up {}", -dist);
            let old_edge_dist = self.cursor_row - self.first_row;
            let dist_up = -(dist.max(-(self.cursor_row as isize))) as usize;
            if old_edge_dist <= dist_up {
                self.first_row -= dist_up - old_edge_dist;
            }
            self.cursor_row -= dist_up;
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
            } else if self.cursor_col == 0{
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
