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
    pub fn move_cursor_left_right(&mut self, buffers: &[Buffer], dist: isize) -> Result<(), Error> {
        let buffer = buffers
            .get(self.buffer_id)
            .ok_or(Error::BufferClosedPrematurely(self.buffer_id))?;
        self.cursor
            .move_left_right(buffer, &mut self.offset, self.width.saturating_sub(2), dist)
    }

    pub fn move_cursor_up_down(&mut self, buffers: &[Buffer], dist: isize) -> Result<(), Error> {
        let buffer = buffers
            .get(self.buffer_id)
            .ok_or(Error::BufferClosedPrematurely(self.buffer_id))?;
        debug_assert_ne!(self.height, 0, "the height of the pane cannot be 0");
        #[allow(clippy::integer_arithmetic)]
        self.cursor.move_up_down(
            buffer,
            &mut self.offset,
            self.height - 1,
            self.width.saturating_sub(2),
            dist,
        )?;
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
            buffer.dirty = true;
            if let Some(line) = buffer.lines.get_mut(self.cursor.row) {
                debug_assert!(line.len() >= self.cursor.col, "cursor has moved past the end of the line");
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
            buffer.dirty = true;
            if let Some(line) = buffer.lines.get_mut(self.cursor.row) {
                debug_assert!(line.len() >= self.cursor.col, "cursor has moved past the end of the line");
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
        buffer.dirty = true;
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

    pub fn display<'a>(&self, buffers: &'a [Buffer], default: &'a [Vec<char>]) -> Result<Iter<'a>, Error> {
        let buffer = buffers
            .get(self.buffer_id)
            .ok_or(Error::BufferClosedPrematurely(self.buffer_id))?;

        let status_bar = if buffer.is_norm {
            let mut status_bar = buffer
                .file_name
                .as_ref()
                .cloned()
                .unwrap_or_else(|| String::from("[No Name]"));
            status_bar.push_str(if buffer.dirty { " | + | " } else { " " });
            status_bar.push_str(&(self.cursor.row + 1).to_string());
            status_bar.push(':');
            status_bar.push_str(&buffer.lines.len().to_string());
            Some(status_bar)
        } else {
            None
        };

        Ok(Iter {
            text: if buffer.lines == vec![Vec::new()] {
                Some(default)
            } else if buffer.lines.len() < self.offset.row {
                None
            } else {
                Some(&buffer.lines[self.offset.row..])
            },
            col_offset: self.offset.col,
            height: self.height,
            row: 0,
            status_bar,
            width: self.width,
            draw_tildes: buffer.is_norm,
        })
    }
}

pub struct Iter<'a> {
    text: Option<&'a [Vec<char>]>,
    col_offset: usize,
    width: usize,
    height: usize,
    status_bar: Option<String>,
    row: usize,
    draw_tildes: bool,
}

pub enum Row<'a> {
    Normal(&'a [char]),
    Empty{part_of_file: bool},
    StatusBar(String),
}

pub struct RowIter<'a> {
    row: Row<'a>,
    col: usize,
    line: usize,
    width: usize,
    draw_tildes: bool,
}

impl<'a> Iterator for Iter<'a> {
    type Item = RowIter<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.row < self.height {
            if self.row == self.height - 1 && matches!(self.status_bar, Some(_)) {
                self.row += 1;
                Some(RowIter {
                    row: Row::StatusBar(self.status_bar.as_ref().unwrap().clone()),
                    col: 0,
                    width: self.width,
                    draw_tildes: self.draw_tildes,
                    line: self.row
                })
            } else {
                self.row += 1;
                let row = self.text.and_then(|text| text.get(self.row - 1));
                Some(RowIter {
                    row: if row.map(Vec::len).map(|row| self.col_offset >= row).unwrap_or(false) {
                        Row::Empty{part_of_file: true}
                    } else if row.is_none() {
                        Row::Empty{part_of_file: false}
                    } else {
                        Row::Normal(&row.unwrap()[self.col_offset..])
                    },
                    col: 0,
                    width: self.width,
                    draw_tildes: self.draw_tildes,
                    line: self.row
                })
            }
        } else {
            None
        }
    }
}

impl<'a> Iterator for RowIter<'a> {
    type Item = Char;
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.row {
            Row::Normal(_) | Row::Empty{part_of_file: true} => {
                if self.col < self.width {
                    self.col += 1;
                    if self.draw_tildes {
                        if self.col == 1 {
                            if self.line > 99 {
                                Some(Char::Normal((((self.line / 100) % 10) as u8 + 48) as char))
                            } else {
                                Some(Char::Normal(' '))
                            }
                        } else if self.col == 2 {
                            if self.line > 9 {
                                Some(Char::Normal((((self.line / 10) % 10) as u8 + 48) as char))
                            } else {
                                Some(Char::Normal(' '))
                            }
                        } else if self.col == 3 {
                            Some(Char::Normal(((self.line % 10) as u8 + 48) as char))
                        } else if self.col == 4 {
                            Some(Char::Normal(' '))
                        } else if let Row::Normal(r) = &mut self.row {
                            Some(Char::Normal(r.get(self.col - 5).copied().unwrap_or(' ')))
                        } else {
                            Some(Char::Normal(' '))
                        }
                    } else if let Row::Normal(r) = self.row {
                        Some(Char::Normal(r.get(self.col - 1).copied().unwrap_or(' ')))
                    } else {
                        Some(Char::Normal(' '))
                    }
                } else {
                    None
                }
            }
            Row::Empty{part_of_file} => {
                if self.col >= self.width {
                    None
                } else if *part_of_file && self.col == 0 {
                    self.col += 1;
                    Some(Char::Normal('~'))
                } else {
                    self.col += 1;
                    Some(Char::Normal(' '))
                }
            }
            Row::StatusBar(sb) => {
                if self.col == 0 {
                    self.col += 1;
                    Some(Char::Background(Colour::Red))
                } else if self.col < self.width + 1 {
                    self.col += 1;
                    Some(Char::Normal(sb.chars().nth(self.col - 2).unwrap_or(' ')))
                } else if self.col == self.width + 1 {
                    self.col += 1;
                    Some(Char::Background(Colour::Reset))
                } else {
                    None
                }
            }
        }
    }
}
