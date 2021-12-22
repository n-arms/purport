use super::cursor::*;
use super::pane::*;
use crate::frontend::ui::*;
use std::fs;

#[derive(Clone, Debug)]
pub struct Editor {
    pub buffers: Vec<Buffer>,
    pub pane: Pane,
}

#[derive(Clone, Debug)]
pub struct Buffer {
    pub lines: Vec<Vec<char>>,
    pub file_name: Option<String>
}

impl Editor {
    pub fn open(width: usize, height: usize) -> Self {
        Editor {
            buffers: vec![Buffer {
                lines: vec![String::from("welcome!").chars().collect()],
                file_name: None
            }],
            pane: Pane {
                width,
                height,
                buffer: 0,
                offset: Offset::default(),
                cursor: Cursor::default(),
            },
        }
    }

    pub fn load_into(&mut self, buffer: usize, file_name: Option<String>) -> Option<()> {
        let buffer = self.buffers.get_mut(buffer)?;
        buffer.lines = file_name
            .clone()
            .and_then(|fp| fs::read(&fp).ok())
            .map(|file| {
                let mut lines: Vec<_> = String::from_utf8_lossy(file.as_ref())
                    .split('\n')
                    .map(|line| line.chars().collect())
                    .collect();
                lines.truncate(lines.len() - 1);
                lines
            })
            .unwrap_or_else(|| vec![String::from("welcome!").chars().collect()]);
        buffer.file_name = file_name;
        Some(())
    }

    pub fn draw(&self, ui: &mut impl UI) {
        let lines = self
            .pane
            .display(&self.buffers)
            .expect("failed to produce image: buffer was likely closed prematurely");
        let mut first = true;
        for line in lines {
            if !first {
                ui.newln();
            }
            first = false;
            for c in line {
                match c {
                    Char::Normal(c) => ui.draw(&c.to_string()),
                    Char::Foreground(c) => ui.set_foreground(c),
                    Char::Background(c) => ui.set_background(c)
                }
            }
            ui.move_cursor(self.pane.cursor.row + 1, self.pane.cursor.col + 3);
        }
    }
}
