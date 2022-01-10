use super::buffer::{Buffer, Line};
use super::cursor::{Cursor, Offset};
use super::highlight::Theme;
use super::pane::{Char, Pane};
use super::prompt::Prompt;
use crate::frontend::ui::{self, EscapeSeq, Event, UI};
use std::path::PathBuf;

use super::language::Languages;
use std::cell::RefCell;
use std::fs;
use std::io;

use std::time::Instant;

#[cfg(unix)]
static C_COMPILER: &str = "gcc";
#[cfg(unix)]
static CPP_COMPILER: &str = "g++";
#[cfg(unix)]
static TARGET_DIR: &str = "./target/temp/";

#[cfg(windows)]
static DEFAULT_SYSTEM_DATA: GlobalSystemData = panic!("i am not familiar with windows and so don't know what sensible system defaults would be, if you are looking for windows support, please submit a pull request");

#[derive(Debug)]
pub struct Editor<U: UI> {
    buffers: Vec<Buffer>,
    pane: Pane,
    mode: Mode,
    prompt: Prompt,
    ui: U,
    theme: Theme,
    extensions: Languages,
}

#[derive(Clone, Debug, Copy)]
pub enum Mode {
    Insert = 0,
}

#[derive(Debug)]
pub enum Error {
    BufferClosedPrematurely(usize),
    IO(io::Error),
    UI(ui::Error),
}

impl<U: UI> Editor<U> {
    pub fn open(ui: U) -> Result<Self, Error> {
        let mut buffers = vec![
            Buffer::new(vec![Line::default()], false, None, None),
            Buffer::new(vec![Line::default()], true, None, None),
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
            extensions: Languages::default(),
        })
    }

    pub fn load_into(&mut self, buffer_id: usize, file_name: Option<String>) -> Option<()> {
        let buffer = self.buffers.get_mut(buffer_id)?;
        if let Some(bytes) = file_name.clone().and_then(|fp| fs::read(&fp).ok()) {
            let h = if let Some(fp) = &file_name {
                match self.extensions.get(fp) {
                    Ok(res) => Some(res),
                    Err(e) => {
                        eprintln!("{:?}", e);
                        None
                    }
                }
            } else {
                None
            }?;
            *buffer = Buffer::from_bytes(&bytes, file_name, Some(RefCell::new(h)));
        } else {
            buffer.clear();
            buffer.file_name = None;
            buffer.dirty = false;
            buffer.is_norm = true;
        }
        Some(())
    }

    pub fn save(&mut self, buffer_id: usize) -> Result<(), Error> {
        let mut buffer = self
            .buffers
            .get_mut(buffer_id)
            .ok_or(Error::BufferClosedPrematurely(buffer_id))?;
        if let Some(fp) = &buffer.file_name {
            fs::write(fp, buffer.to_chunk()).map_err(Error::IO)?;
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

    // we are currently highlighting relative to the bottom of the screen instead of line 0: TODO
    pub fn draw(&mut self) -> Result<(), Error> {
        #[cfg(debug_assertions)]
        let now = Instant::now();
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
                        eprintln!("highlighting {:?}", h);
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
            self.ui.set_foreground(ui::Colour::Reset);
        }
        self.ui.move_cursor(
            self.pane.cursor.row + 1 - self.pane.offset.row,
            self.pane.cursor.col + 5 - self.pane.offset.col,
        );
        #[cfg(debug_assertions)]
        {
            eprintln!("it took {:?} ms to refresh the editor", now.elapsed());
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
            let ev = self.ui.next_event().map_err(Error::UI)?;
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
        self.buffers[0].clear();
        Ok(res)
    }

    pub fn mainloop(mut self) -> Result<(), Error> {
        self.refresh()?;
        loop {
            let ev = self.ui.next_event().map_err(Error::UI)?;
            if self.process_event(&ev)? {
                break;
            }
            self.refresh()?;
        }
        self.refresh()
    }

    pub fn refresh(&mut self) -> Result<(), Error> {
        self.draw()?;
        self.ui.refresh().map_err(Error::UI)
    }
}

#[derive(Debug, Clone)]
pub struct GlobalSystemData {
    pub c_compiler: String,
    pub cpp_compiler: String,
    pub target_dir: PathBuf,
}

impl Default for GlobalSystemData {
    fn default() -> Self {
        let mut target_dir = PathBuf::new();
        target_dir.push(TARGET_DIR);
        GlobalSystemData {
            c_compiler: String::from(C_COMPILER),
            cpp_compiler: String::from(CPP_COMPILER),
            target_dir,
        }
    }
}
