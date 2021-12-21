use super::pane::*;
use crate::frontend::ui::*;
use std::io::Write;
use super::cursor::*;

#[derive(Clone, Debug)]
pub struct Editor {
    pub buffers: Vec<Buffer>,
    pub pane: Pane,
}

#[derive(Clone, Debug)]
pub struct Buffer {
    pub lines: Vec<Vec<char>>,
}

impl Editor {
    pub fn open(width: usize, height: usize) -> Self {
        Editor {
            buffers: vec![Buffer {
                lines: vec![Vec::new()],
            }],
            pane: Pane {
                width: width - 3,
                height: height - 2,
                buffer: 0,
                offset: Offset::default(),
                cursor: Cursor::default()
            }
        }
    }

    pub fn load_into(&mut self, buffer: usize, text: String) -> Option<()> {
        self.buffers.get_mut(buffer)?.lines = text
            .split('\n')
            .map(|line| line.chars().collect())
            .collect();
        Some(())
    }

    pub fn draw(&self, ui: &mut impl UI) {
        let lines = self
            .pane
            .display(&self.buffers)
            .expect("failed to produce image: buffer was likely closed prematurely");

        for _ in 0..self.pane.width + 2 {
            ui.draw("─");
        }
        ui.draw("┐");
        ui.newln();
        for line in lines {
            ui.draw("~ ");
            for c in line {
                match c {
                    Char::Background(c) => ui.set_background(c),
                    Char::Foreground(c) => ui.set_foreground(c),
                    Char::Normal(c) => ui.draw(&c.to_string()),
                }
            }
            ui.draw("│");
            ui.newln();
        }
        for _ in 0..self.pane.width + 2 {
            ui.draw("─");
        }
        ui.draw("┘");
        ui.move_cursor(self.pane.cursor.row + 2 - self.pane.offset.row, self.pane.cursor.col + 3 - self.pane.offset.col);
    }
}
