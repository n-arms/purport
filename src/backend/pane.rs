#![warn(unsafe_code)]

use super::editor::*;

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

impl Pane {
    // does not draw cursor
    pub fn display<'a>(&self, buffers: &'a [Buffer]) -> Option<impl Iterator<Item = impl Iterator<Item = &'a char>>> {
        let buffer = buffers.get(self.buffer)?;
        let first_col = self.first_col;
        let width = self.width;
        Some(buffer.lines.iter()
            .skip(self.first_row)
            .take(self.height)
            .map(move |line| line.iter()
                 .skip(first_col)
                 .take(width)))
    }

    pub fn move_cursor(&mut self, buffers: &[Buffer], rows: isize, cols: isize) -> Option<()> {
        let buffer = buffers.get(self.buffer)?;
        if let Some(line) = buffer.lines.get((self.cursor_row as isize + rows) as usize) {
            self.cursor_row = (self.cursor_row as isize + rows) as usize;
            self.cursor_col = ((self.cursor_col as isize + cols).max(0) as usize).min(line.len());
        } else if (-rows) > self.cursor_row as isize {
            self.cursor_row = 0;
            self.cursor_col = ((self.cursor_col as isize + cols).max(0) as usize).min(buffer.lines.get(0).map(|line| line.len()).unwrap_or(0));
        } else {
            self.cursor_row = buffer.lines.len();
            self.cursor_col = 0;
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
                self.move_cursor(buffers, 1, 0);
            }
            Some(())
        } else {
            let buffer = buffers.get_mut(self.buffer)?;
            if let Some(line) = buffer.lines.get_mut(self.cursor_row) {
                let rest = &line[self.cursor_col..].to_owned();
                line.truncate(self.cursor_col);
                line.push(c);
                line.extend(rest);
                self.move_cursor(buffers, 0, 1);
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
                self.move_cursor(buffers, 0, -1)
            }
        } else {
            self.move_cursor(buffers, -1, 0)
        }
    }
}
