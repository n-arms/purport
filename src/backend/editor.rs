use super::pane::*;
use crate::frontend::ui::*;
use std::io::Write;

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
                first_row: 0,
                first_col: 0,
                cursor_row: 0,
                cursor_col: 0,
            },
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
        ui.move_cursor(self.pane.cursor_row + 2 - self.pane.first_row, self.pane.cursor_col + 3 - self.pane.first_col);
    }
}
