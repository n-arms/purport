use super::cursor::{Cursor, Offset};
use super::pane::{Char, Pane};
use super::prompt::Prompt;
use crate::frontend::ui::{self, EscapeSeq, Event, UI};
use std::fs;
use std::io;

#[derive(Clone, Debug)]
pub struct Editor<U: UI> {
    pub buffers: Vec<Buffer>,
    pub pane: Pane,
    pub mode: Mode,
    pub prompt: Prompt,
    pub ui: U,
}

#[derive(Clone, Debug)]
pub struct Buffer {
    pub lines: Vec<Vec<char>>,
    pub file_name: Option<String>,
    pub dirty: bool,
    pub is_norm: bool,
}

#[derive(Clone, Debug, Copy)]
pub enum Mode {
    Insert = 0,
}

#[derive(Debug)]
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
    },
    IOErr(io::Error),
    UIErr(ui::Error),
}

impl<U: UI> Editor<U> {
    pub fn open(ui: U) -> Result<Self, Error> {
        let mut buffers = vec![
            Buffer {
                lines: vec![Vec::new()],
                file_name: None,
                dirty: false,
                is_norm: false,
            },
            Buffer {
                lines: vec![Vec::new()],
                file_name: None,
                dirty: false,
                is_norm: true,
            },
        ];
        let prompt = Prompt::new(ui.width(), 0, &mut buffers, "")?;

        Ok(Editor {
            buffers,
            pane: Pane {
                width: ui.width(),
                height: ui.height() - 1,
                buffer_id: 1,
                offset: Offset::default(),
                cursor: Cursor::default(),
            },
            mode: Mode::Insert,
            prompt,
            ui,
        })
    }

    pub fn load_into(&mut self, buffer_id: usize, file_name: Option<String>) -> Option<()> {
        let buffer = self.buffers.get_mut(buffer_id)?;
        buffer.lines = file_name
            .clone()
            .and_then(|fp| fs::read(&fp).ok())
            .map_or_else(
                || vec![Vec::new()],
                |file| {
                    let mut lines: Vec<_> = String::from_utf8_lossy(file.as_ref())
                        .split('\n')
                        .map(|line| line.chars().collect())
                        .collect();
                    lines.truncate(lines.len().saturating_sub(1));
                    lines
                },
            );
        buffer.file_name = file_name;
        Some(())
    }

    pub fn save(&mut self, buffer_id: usize) -> Result<(), Error> {
        let mut buffer = self
            .buffers
            .get_mut(buffer_id)
            .ok_or(Error::BufferClosedPrematurely(buffer_id))?;
        if let Some(fp) = &buffer.file_name {
            fs::write(
                fp,
                buffer
                    .lines
                    .iter()
                    .map(|line| {
                        line.iter().fold(String::new(), |mut acc, x| {
                            acc.push(*x);
                            acc
                        })
                    })
                    .fold(String::new(), |mut acc, x| {
                        acc.push_str(&x);
                        acc.push('\n');
                        acc
                    }),
            )
            .map_err(Error::IOErr)?;
        } else {
            todo!("implement save as");
        }

        buffer.dirty = false;
        Ok(())
    }

    pub fn draw(&mut self) -> Result<(), Error> {
        let lines = self.pane.display(&self.buffers)?;
        let mut first = true;
        for line in lines.chain(self.prompt.display(&self.buffers)?) {
            if !first {
                self.ui.newln();
            }
            first = false;
            for ch in line {
                match ch {
                    Char::Normal(c) => self.ui.draw(&c.to_string()),
                    Char::Foreground(c) => self.ui.set_foreground(c),
                    Char::Background(c) => self.ui.set_background(c),
                }
            }
            self.ui.move_cursor(
                self.pane.cursor.row + 1 - self.pane.offset.row,
                self.pane.cursor.col + 3 - self.pane.offset.col,
            );
        }
        Ok(())
    }
    // processing an event could result in processing a prompt
    // a prompt requires a stream of events not just an event
    // we also want to be able to update the ui after each event
    // concurrency is an option (and might be nice) but it's not needed right now
    // instead, we need to give the entire impl UI object to the editor
    // so much sharing of the UI would happen due to Editor::prompt that it is probably worth just
    // making it a field
    pub fn process_event(&mut self, event: &Event) -> Result<bool, Error> {
        match event {
            Event::SpecialChar(EscapeSeq::DownArrow) => {
                self.pane.move_cursor_up_down(&self.buffers, 1)
            }
            Event::SpecialChar(EscapeSeq::UpArrow) => {
                self.pane.move_cursor_up_down(&self.buffers, -1)
            }
            Event::SpecialChar(EscapeSeq::LeftArrow) => {
                self.pane.move_cursor_left_right(&self.buffers, -1)
            }
            Event::SpecialChar(EscapeSeq::RightArrow) => {
                self.pane.move_cursor_left_right(&self.buffers, 1)
            }
            Event::NormalChar('\x01') => {
                let text = self.prompt("text: ")?;
                for c in text.chars() {
                    self.pane.insert_char(&mut self.buffers, c)?;
                }
                Ok(())
            }
            Event::NormalChar('\x11') => return Ok(true),
            Event::NormalChar('\x13') => self.save(self.pane.buffer_id),
            Event::NormalChar('\x7f') => self.pane.backspace(&mut self.buffers),
            Event::NormalChar(c) => self.pane.insert_char(&mut self.buffers, *c),
        }?;
        Ok(false)
    }

    pub fn prompt(&mut self, text: &str) -> Result<String, Error> {
        self.prompt = Prompt::new(self.pane.width, 0, &mut self.buffers[..], text)?;
        self.refresh()?;
        let res;
        loop {
            let ev = self.ui.next_event().map_err(Error::UIErr)?;
            match ev {
                Event::SpecialChar(EscapeSeq::DownArrow | EscapeSeq::UpArrow) => continue,
                Event::SpecialChar(EscapeSeq::LeftArrow) => {
                    self.prompt.move_cursor_left_right(&self.buffers, -1)
                }
                Event::SpecialChar(EscapeSeq::RightArrow) => {
                    self.prompt.move_cursor_left_right(&self.buffers, 1)
                }
                Event::NormalChar('\x7f') => self.prompt.backspace(&mut self.buffers),
                Event::NormalChar('\r') => {
                    res = self.prompt.take(&self.buffers)?;
                    break;
                }
                Event::NormalChar(c) => self.prompt.insert_char(&mut self.buffers, c),
            }?;
            self.refresh()?;
        }

        Ok(res)
    }

    pub fn mainloop(mut self) -> Result<(), Error> {
        self.refresh()?;
        loop {
            let ev = self.ui.next_event().map_err(Error::UIErr)?;
            if self.process_event(&ev)? {
                break;
            }
            self.refresh()?;
        }
        self.refresh()
    }

    pub fn refresh(&mut self) -> Result<(), Error> {
        self.draw()?;
        self.ui.refresh().map_err(Error::UIErr)
    }
}
