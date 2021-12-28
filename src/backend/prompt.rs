use super::buffer::Buffer;
use super::cursor::{Cursor, Offset};
use super::editor::Error;

use super::pane::{Iter, Pane};
use std::convert::TryInto;
use unicode_segmentation::UnicodeSegmentation;

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
        for g in text[..].graphemes(true) {
            pane.insert_grapheme(buffers, g)?;
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
                dist.max(
                    -TryInto::<isize>::try_into(self.pane.cursor.col - self.prompt_text_len)
                        .expect("overflow"),
                ),
            )
        } else {
            self.pane.move_cursor_left_right(buffers, dist)
        }
    }

    pub fn insert_grapheme(&mut self, buffers: &mut [Buffer], g: &str) -> Result<(), Error> {
        self.pane.insert_grapheme(buffers, g)
    }

    pub fn backspace(&mut self, buffers: &mut [Buffer]) -> Result<(), Error> {
        self.pane.backspace(buffers)
    }

    pub fn display<'a>(&self, buffers: &'a [Buffer]) -> Result<Iter<'a>, Error> {
        self.pane.display(buffers, &[])
    }

    pub fn take(&self, buffers: &[Buffer]) -> Result<String, Error> {
        let buffer = buffers
            .get(self.pane.buffer_id)
            .ok_or(Error::BufferClosedPrematurely(self.pane.buffer_id))?;
        debug_assert_ne!(buffer.lines.len(), 0, "the buffer is empty");
        Ok(buffer.lines[0].skip(self.prompt_text_len).to_string())
    }
}
