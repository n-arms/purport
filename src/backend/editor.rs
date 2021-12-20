use crate::frontend::ui::*;
use super::pane::*;

#[derive(Clone, Debug)]
pub struct Editor {
    pub buffers: Vec<Buffer>,
    pub pane: Pane
}


#[derive(Clone, Debug)]
pub struct Buffer {
    pub lines: Vec<Vec<char>>
}


impl Editor {
    pub fn open(width: usize, height: usize) -> Self {
        Editor {
            buffers: vec![Buffer {lines: vec![Vec::new()]}],
            pane: Pane {
                width: width - 2,
                height,
                buffer: 0,
                first_row: 0,
                first_col: 0,
                cursor_row: 0,
                cursor_col: 0
            }
        }
    }
    pub fn load_into(&mut self, buffer: usize, text: String) -> Option<()> {
        self.buffers.get_mut(buffer)?.lines = text.split('\n').map(|line| line.chars().collect()).collect();
        Some(())
    }
    fn draw_cursor(ui: &mut impl UI) {
        ui.set_background(Colour::Red);
        ui.draw(" ");
        ui.set_background(Colour::Reset);
    }
    pub fn draw(&self, ui: &mut impl UI) {
        let img = self.pane.display(&self.buffers).expect("failed to produce image: buffer was likely closed prematurely");
        let mut total = 0;
        let mut drawn_cursor = false;
        for (y, line) in img.enumerate() {
            ui.draw("~ ");
            total += 1;
            if y == self.pane.cursor_row {
                for (x, c) in line.enumerate() {
                    if x == self.pane.cursor_col {
                        drawn_cursor = true;
                        Editor::draw_cursor(ui);
                    } else {
                        ui.draw(&c.to_string());
                    }
                }
                if !drawn_cursor {
                    Editor::draw_cursor(ui);
                    drawn_cursor = true;
                }
                ui.newln();
            } else {
                let mut line_str = String::new();
                line_str.extend(line);
                ui.drawln(&line_str);
            }
        }
        if !drawn_cursor {
            ui.draw("~ ");
            Editor::draw_cursor(ui);
        }
        for _ in total..self.pane.height-1 {
            ui.newln();
        }
        ui.refresh().expect("failed to refresh ui");
    }
}
