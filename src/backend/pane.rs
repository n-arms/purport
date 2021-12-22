use super::cursor::{Cursor, Offset};
use super::editor::{Buffer, Error};
use crate::frontend::ui::Colour;

#[derive(Clone, Debug, Default)]
pub struct Pane {
    pub buffer_id: usize,
    pub width: usize,
    pub height: usize,
    pub offset: Offset,
    pub cursor: Cursor,
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub enum Char {
    Normal(char),
    Foreground(Colour),
    Background(Colour),
}

impl Pane {
    pub fn move_cursor_left_right(
        &mut self,
        buffers: &[Buffer],
        dist: isize,
    ) -> Result<(), Error> {
        let buffer = buffers
            .get(self.buffer_id)
            .ok_or(Error::BufferClosedPrematurely(self.buffer_id))?;
        self.cursor
            .move_left_right(buffer, &mut self.offset, self.width.saturating_sub(2), dist)
    }

    pub fn move_cursor_up_down(
        &mut self,
        buffers: &[Buffer],
        dist: isize,
    ) -> Result<(), Error> {
        let buffer = buffers
            .get(self.buffer_id)
            .ok_or(Error::BufferClosedPrematurely(self.buffer_id))?;
        if self.height == 0 {
            return Err(Error::InvalidHeight(self.height));
        }
        #[allow(clippy::integer_arithmetic)]
        self.cursor
            .move_up_down(buffer, &mut self.offset, self.height - 1, self.width.saturating_sub(2), dist)?;
        Ok(())
    }
    pub fn set_cursor(&mut self, row: usize, col: usize) {
        self.cursor.col = col;
        self.cursor.row = row;
    }

    pub fn insert_char(&mut self, buffers: &mut [Buffer], c: char) -> Result<(), Error> {
        if c == '\r' {
            let buffer = buffers
                .get_mut(self.buffer_id)
                .ok_or(Error::BufferClosedPrematurely(self.buffer_id))?;
            if let Some(line) = buffer.lines.get_mut(self.cursor.row) {
                if line.len() < self.cursor.col {
                    return Err(Error::CursorPastEnd {
                        cursor: self.cursor.col,
                        pos: line.len()
                    });
                }
                #[allow(clippy::indexing_slicing)]
                let rest = line[self.cursor.col..].to_vec();
                line.truncate(self.cursor.col);
                buffer.lines.insert(self.cursor.row.saturating_add(1), rest); // if the file is usize::MAX lines long this will break
                self.set_cursor(self.cursor.row.saturating_add(1), 0);
            } else {
                buffer.lines.push(Vec::new());
                self.move_cursor_up_down(buffers, 1)?;
            }
            Ok(())
        } else {
            let buffer = buffers
                .get_mut(self.buffer_id)
                .ok_or(Error::BufferClosedPrematurely(self.buffer_id))?;
            if let Some(line) = buffer.lines.get_mut(self.cursor.row) {
                if self.cursor.col > line.len() {
                    return Err(Error::CursorPastEnd {
                            cursor: self.cursor.col,
                            pos: line.len(),
                        }
                    );
                }
                #[allow(clippy::indexing_slicing)]
                let rest = &line[self.cursor.col..].to_owned();
                line.truncate(self.cursor.col);
                line.push(c);
                line.extend(rest);
                self.move_cursor_left_right(buffers, 1)?;
            } else {
                buffer.lines.push(vec![c]);
            }

            Ok(())
        }
    }

    pub fn backspace(&mut self, buffers: &mut [Buffer]) -> Result<(), Error> {
        let buffer = buffers
            .get_mut(self.buffer_id)
            .ok_or(Error::BufferClosedPrematurely(self.buffer_id))?;
        if let Some(line) = buffer.lines.get_mut(self.cursor.row) {
            if self.cursor.col == 0 && self.cursor.row == 0 {
                Ok(())
            } else if self.cursor.col == 0 {
                let old = line.clone();
                #[allow(clippy::integer_arithmetic)]
                if let Some(prev) = buffer.lines.get_mut(self.cursor.row - 1) {
                    // row != 0 due to the above if
                    self.set_cursor(self.cursor.row - 1, prev.len());
                    prev.extend(old.iter());
                    buffer.lines.remove(self.cursor.row + 1); // row was decremented by set cursor, so it is less than usize::MAX
                    Ok(())
                } else {
                    Ok(())
                }
            } else {
                #[allow(clippy::integer_arithmetic)]
                line.remove(self.cursor.col - 1); // col != 0 due to the above if
                self.move_cursor_left_right(buffers, -1)
            }
        } else {
            self.move_cursor_up_down(buffers, -1)
        }
    }

    pub fn display<'a>(
        &self,
        buffers: &'a [Buffer],
        welcome: &'a [Vec<char>],
    ) -> Result<impl Iterator<Item = impl Iterator<Item = Char> + 'a> + 'a, Error> {
        let buffer = buffers
            .get(self.buffer_id)
            .ok_or(Error::BufferClosedPrematurely(self.buffer_id))?;
        let first_col = self.offset.col;
        let width = self.width;
        let text = if buffer.lines == vec![Vec::new()] && buffer.file_name == None {
            welcome.iter()
        } else {
            buffer.lines.iter()
        }
        .skip(self.offset.row)
        .take(
            self.height
                .checked_sub(1)
                .ok_or(Error::InvalidHeight(
                    self.height,
                ))?,
        )
        .map(move |line| line.iter().skip(first_col).take(width.saturating_sub(2))); // line iter can handle 0 width lines
        Ok(TextIter {
            text,
            row: 0,
            max_row: self.height,
            max_col: self.width,
            status_bar: StatusBar {
                file_name: buffer
                    .file_name
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| String::from("[No Name]")),
                line: self.cursor.row,
                lines: buffer.lines.len(),
            },
        })
    }
}

#[derive(Debug, Clone)]
pub struct StatusBar {
    file_name: String,
    line: usize,
    lines: usize,
}

pub struct TextIter<'a, L: Iterator<Item = &'a char>, Text: Iterator<Item = L>> {
    text: Text,
    row: usize,
    max_row: usize,
    max_col: usize,
    status_bar: StatusBar,
}

impl<'a, L, Text> Iterator for TextIter<'a, L, Text>
where
    L: Iterator<Item = &'a char>,
    Text: Iterator<Item = L>,
{
    type Item = LineIter<'a, L>;
    fn next(&mut self) -> Option<Self::Item> {
        #[allow(clippy::integer_arithmetic)]
        if self.max_row.checked_sub(1).map(|row| row == self.row)? {
            self.row += 1; // max row (a valid usize) was greater than row, so row + 1 is a valid usize
            Some(LineIter {
                max_col: self.max_col,
                col: 0,
                line: Line::StatusBar(self.status_bar.clone()),
            })
        } else if self.max_row.checked_sub(1).map(|row| row > self.row)? {
            self.row += 1; // same reasoning as above
            Some(LineIter {
                max_col: self.max_col,
                col: 0,
                line: self.text.next().map_or(Line::Empty, |l| Line::Normal(0, l)),
            })
        } else {
            None
        }
    }
}

pub enum Line<'a, L: Iterator<Item = &'a char>> {
    StatusBar(StatusBar),
    Normal(u8, L),
    Empty,
}

// to be used with syntax highlighting later: we want to reuse the memory for each line, (hence
// holding an iterator instead of just collecting, but copying over the start/end points for
// colours doesnt seem too crazy (although it is still O(n))
pub struct LineIter<'a, L: Iterator<Item = &'a char>> {
    max_col: usize,
    col: usize,
    line: Line<'a, L>,
}

impl<'a, L: Iterator<Item = &'a char>> Iterator for LineIter<'a, L> {
    type Item = Char;
    #[allow(clippy::expect_used, clippy::integer_arithmetic, clippy::unwrap_in_result)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.max_col > self.col {
            self.col += 1; // self.col is less then a valid usize, it wont overflow
            let c = match &mut self.line {
                Line::Empty => Char::Normal(' '),
                Line::Normal(i @ 0, _) => {
                    *i = 1;
                    Char::Normal('~')
                }
                Line::Normal(i @ 1, _) => {
                    *i = 2;
                    Char::Normal(' ')
                }
                Line::Normal(_, l) => Char::Normal(l.next().copied().unwrap_or(' ')),
                Line::StatusBar(StatusBar {
                    file_name,
                    line,
                    lines,
                }) => {
                    let line_str = line.to_string();
                    let max_line_str = lines.to_string();
                    // col cannot be 0, as we already added 1 to it
                    if self.col == 1 {
                        Char::Background(Colour::Red)
                    } else if self.col - 1 <= file_name.len() {
                        // col is >= 2
                        Char::Normal(
                            file_name
                                .chars()
                                .nth(self.col - 2)
                                .expect("unreachable due to bounds checks in if statement"),
                        )
                    } else if self.col - 2 == file_name.len() {
                        // col is >= 2
                        Char::Normal(' ')
                    } else if self.col - file_name.len() - 3 < line_str.len() {
                        // col >= 3 + file_name.len
                        Char::Normal(
                            line_str
                                .chars()
                                .nth(self.col - file_name.len() - 3)
                                .expect("unreachable due to bounds checks in if statement"),
                        ) // the panic is unreachable
                    } else if self.col - file_name.len() - line_str.len() == 3 {
                        Char::Normal(':')
                    } else if self.col - file_name.len() - line_str.len() - 4 < max_line_str.len() {
                        // col >= 4 + file_name.len + line_str.len
                        Char::Normal(
                            max_line_str
                                .chars()
                                .nth(self.col - file_name.len() - line_str.len() - 4)
                                .expect("unreachable due to bounds checks in if statement"),
                        ) // the panic is unreachable
                    } else {
                        Char::Normal(' ')
                    }
                }
            };
            Some(c)
        } else if self.max_col == self.col && matches!(self.line, Line::StatusBar(_)) {
            self.col = self.col
                .checked_add(1)
                .expect("overflow: the width of the pane is == usize::MAX");
            Some(Char::Background(Colour::Reset))
        } else {
            None
        }
    }
}
