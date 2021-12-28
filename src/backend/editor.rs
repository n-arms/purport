use super::buffer::{Buffer, Line};
use super::cursor::{Cursor, Offset};
use super::highlight::{RegexHighlighter, Theme};
use super::pane::{Char, Pane};
use super::prompt::Prompt;
use crate::frontend::ui::{self, EscapeSeq, Event, UI};
use regex::Regex;
use std::fs;
use std::io;

#[derive(Debug)]
pub struct Editor<U: UI> {
    pub buffers: Vec<Buffer>,
    pub pane: Pane,
    pub mode: Mode,
    pub prompt: Prompt,
    pub ui: U,
    pub theme: Theme,
}

#[derive(Clone, Debug, Copy)]
pub enum Mode {
    Insert = 0,
}

#[derive(Debug)]
pub enum Error {
    BufferClosedPrematurely(usize),
    IOErr(io::Error),
    UIErr(ui::Error),
}

impl<U: UI> Editor<U> {
    pub fn open(ui: U) -> Result<Self, Error> {
        let mut buffers = vec![
            Buffer {
                lines: vec![Line::default()],
                file_name: None,
                dirty: false,
                is_norm: false,
                highlighter: None,
                file_type: None,
            },
            Buffer {
                lines: vec![Line::default()],
                file_name: None,
                dirty: false,
                is_norm: true,
                highlighter: Some(Box::new(RegexHighlighter {
                    number: Regex::new("\\d+").unwrap(),
                    operator: Regex::new("(\\+)|(\\*)|-|/").unwrap(),
                })),
                file_type: None,
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
            theme: Theme::default(),
        })
    }

    pub fn load_into(&mut self, buffer_id: usize, file_name: Option<String>) -> Option<()> {
        let buffer = self.buffers.get_mut(buffer_id)?;
        buffer.lines = file_name
            .clone()
            .and_then(|fp| fs::read(&fp).ok())
            .map_or_else(
                || vec![Line::default()],
                |file| {
                    let mut lines: Vec<_> = String::from_utf8_lossy(file.as_ref())
                        .split('\n')
                        .map(|line| Line::new(line.chars().collect::<String>()))
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
                    .map(Line::to_string)
                    .fold(String::new(), |mut acc, x| {
                        acc.push_str(&x);
                        acc.push('\n');
                        acc
                    }),
            )
            .map_err(Error::IOErr)?;
            buffer.dirty = false;
        } else {
            let new_name = Some(self.prompt("Enter the file name: ")?);
            let mut buffer = self
                .buffers
                .get_mut(buffer_id)
                .ok_or(Error::BufferClosedPrematurely(buffer_id))?;
            buffer.file_name = new_name;
            self.save(buffer_id)?;
        }
        Ok(())
    }

    pub fn draw(&mut self) -> Result<(), Error> {
        /*
        let margin = "\n".repeat(if self.pane.height <= self.pane.height / 3 + 5 {0} else {self.pane.height / 3});
        let indent = " ".repeat(if self.pane.width <= self.pane.width / 2 - 15 {0} else {self.pane.width / 2 - 15});
        let welcome: Vec<Row> = format!("{}{}Welcome to Purport\n\n{}Ctrl-S to save\n{}Ctrl-Q to quit", margin, indent, indent, indent).split('\n').map(|line| line.chars().collect()).collect();
        */
        let welcome = Vec::new();
        let lines = self.pane.display(&self.buffers, &welcome)?;
        let mut first = true;
        for line in lines.chain(self.prompt.display(&self.buffers)?) {
            if !first {
                self.ui.newln();
            }
            first = false;
            let line_highlighting = line.highlighting.clone();
            for (col, ch) in line.enumerate() {
                if let Some(c) = col.checked_sub(4) {
                    if let Some(h) = line_highlighting.as_ref().and_then(|lh| lh.get(c)) {
                        /*
                        if h != HighlightType::Text {
                            panic!("highlighting column {:?} line {:?} with colour {:?}", col, i, h);
                        }
                        */
                        self.ui.set_foreground(self.theme.get(h));
                    }
                }
                match ch {
                    Char::Normal(c) => self.ui.draw(&c.to_string()),
                    Char::Grapheme(g) => self.ui.draw(g),
                    Char::Foreground(c) => self.ui.set_foreground(c),
                    Char::Background(c) => self.ui.set_background(c),
                }
            }
            self.ui.move_cursor(
                self.pane.cursor.row + 1 - self.pane.offset.row,
                self.pane.cursor.col + 5 - self.pane.offset.col,
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
                    self.pane
                        .insert_grapheme(&mut self.buffers, &c.to_string())?;
                }
                Ok(())
            }
            Event::NormalChar('\x11') => {
                return Ok(
                    if self
                        .buffers
                        .get(self.pane.buffer_id)
                        .ok_or(Error::BufferClosedPrematurely(self.pane.buffer_id))?
                        .dirty
                    {
                        let mut should_quit = String::new();
                        while should_quit != "y" && should_quit != "n" {
                            should_quit = self
                                .prompt(
                                    "This file has unsaved changes do you want to quit (y/n): ",
                                )?
                                .to_ascii_lowercase();
                        }
                        should_quit == "y"
                    } else {
                        true
                    },
                );
            }
            Event::NormalChar('\x13') => self.save(self.pane.buffer_id),
            Event::NormalChar('\x7f') => self.pane.backspace(&mut self.buffers),
            Event::NormalChar(c) => self.pane.insert_grapheme(&mut self.buffers, &c.to_string()),
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
                Event::NormalChar(c) => self
                    .prompt
                    .insert_grapheme(&mut self.buffers, &c.to_string()),
            }?;
            self.refresh()?;
        }
        self.buffers[0].lines = vec![Line::default()];
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
