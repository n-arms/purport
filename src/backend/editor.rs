use super::cursor::{Cursor, Offset};
use super::pane::{Char, Pane};
use crate::frontend::ui::UI;
use std::fs;

#[derive(Clone, Debug)]
pub struct Editor {
    pub buffers: Vec<Buffer>,
    pub pane: Pane,
}

#[derive(Clone, Debug)]
pub struct Buffer {
    pub lines: Vec<Vec<char>>,
    pub file_name: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Error {
    BufferClosedPrematurely(usize),
    InvalidHeight(usize),
    OffsetGreaterThanCursor {
        cursor: usize,
        offset: usize,
    },
    CursorOffScreen {
        cursor_on_screen: usize,
        screen_size: usize,
    },
    CursorPastEnd {
        cursor: usize,
        pos: usize,
    }
}

impl Editor {
    pub fn open(width: usize, height: usize) -> Self {
        Editor {
            buffers: vec![Buffer {
                lines: vec![Vec::new()],
                file_name: None,
            }],
            pane: Pane {
                width,
                height,
                buffer_id: 0,
                offset: Offset::default(),
                cursor: Cursor::default(),
            },
        }
    }

    pub fn load_into(&mut self, buffer_id: usize, file_name: Option<String>) -> Option<()> {
        let buffer = self.buffers.get_mut(buffer_id)?;
        buffer.lines = file_name
            .clone()
            .and_then(|fp| fs::read(&fp).ok())
            .map_or_else(|| vec![Vec::new()], |file| {
                let mut lines: Vec<_> = String::from_utf8_lossy(file.as_ref())
                    .split('\n')
                    .map(|line| line.chars().collect())
                    .collect();
                lines.truncate(lines.len().saturating_sub(1));
                lines
            });
        buffer.file_name = file_name;
        Some(())
    }

    pub fn draw(&self, ui: &mut impl UI) -> Result<(), Error> {
        #[allow(clippy::integer_division)]
        let margin = String::from("\n").repeat((self.pane.height.saturating_sub(5)) / 3);
        #[allow(clippy::integer_division)]
        let indent = String::from(" ").repeat((self.pane.width.saturating_sub(18)) / 2);
        let welcome: Vec<Vec<char>> = format!(
            "{}{}welcome to purport\n\n{}Ctrl-s to save\n{}Ctrl-q to quit\n",
            margin, &indent, &indent, &indent
        )
        .split('\n')
        .map(|line| line.chars().collect())
        .collect();
        let lines = self
            .pane
            .display(&self.buffers, &welcome[..])?;
        let mut first = true;
        for line in lines {
            if !first {
                ui.newln();
            }
            first = false;
            for ch in line {
                match ch {
                    Char::Normal(c) => ui.draw(&c.to_string()),
                    Char::Foreground(c) => ui.set_foreground(c),
                    Char::Background(c) => ui.set_background(c),
                }
            }
            #[allow(clippy::integer_arithmetic)]
            ui.move_cursor(
                self.pane.cursor.row + 1 - self.pane.offset.row, 
                self.pane.cursor.col + 3 - self.pane.offset.col
            );
        }
        Ok(())
    }
}
