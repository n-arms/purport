use super::cursor::{Cursor, Offset};
use super::editor::{Buffer, Error};
use super::pane::{Pane, Iter};
use std::convert::TryInto;

#[derive(Debug, Clone)]
pub struct Prompt {
    pane: Pane,
    prompt_text_len: usize,
}

impl Prompt {
    pub fn new(
        width: usize,
        buffer_id: usize,
        buffers: &mut [Buffer],
        text: &str,
    ) -> Result<Self, Error> {
        let mut pane = Pane {
            buffer_id,
            cursor: Cursor::default(),
            offset: Offset::default(),
            height: 1,
            width,
        };
        for c in text.chars() {
            pane.insert_char(buffers, c)?;
        }
        Ok(Prompt {
            pane,
            prompt_text_len: text.len(),
        })
    }

    pub fn move_cursor_left_right(&mut self, buffers: &[Buffer], dist: isize) -> Result<(), Error> {
        if dist < 0 {
            self.pane.move_cursor_left_right(
                buffers,
                dist.max(-TryInto::<isize>::try_into(self.pane.cursor.col - self.prompt_text_len).expect("overflow")),
            )
        } else {
            self.pane.move_cursor_left_right(buffers, dist)
        }
    }

    pub fn insert_char(&mut self, buffers: &mut [Buffer], c: char) -> Result<(), Error> {
        self.pane.insert_char(buffers, c)
    }

    pub fn backspace(&mut self, buffers: &mut [Buffer]) -> Result<(), Error> {
        self.pane.backspace(buffers)
    }

    pub fn display<'a>(&self, buffers: &'a [Buffer]) -> Result<Iter<'a>, Error> {
        self.pane.display(buffers)
    }

    pub fn take(&self, buffers: &[Buffer]) -> Result<String, Error> {
        let buffer = buffers
            .get(self.pane.buffer_id)
            .ok_or(Error::BufferClosedPrematurely(self.pane.buffer_id))?;
        Ok(buffer
            .lines
            .get(0)
            .ok_or(Error::InvalidHeight(0))?
            .iter()
            .skip(self.prompt_text_len)
            .collect())
    }
}
