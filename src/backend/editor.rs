use crate::frontend::ui::*;

#[derive(Clone, Debug)]
pub struct Editor {
    buffers: Vec<Buffer>,
    pane: Pane
}


#[derive(Clone, Debug)]
pub struct Buffer {
    lines: Vec<Vec<char>>
}

#[derive(Clone, Debug, Default)]
pub struct Pane {
    buffer: usize,
    first_row: usize,
    first_col: usize,
    cursor_row: usize,
    cursor_col: usize,
    width: usize,
    height: usize
}

impl Pane {
    // does not draw cursor
    fn display<'a>(&self, buffers: &'a [Buffer]) -> Option<impl Iterator<Item = impl Iterator<Item = &'a char>>> {
        let buffer = buffers.get(self.buffer)?;
        let first_col = self.first_col;
        let width = self.width;
        Some(buffer.lines.iter()
            .skip(self.first_row)
            .take(self.height)
            .map(move |line| line.iter()
                 .skip(first_col)
                 .take(width)))
    }

    fn move_cursor(&mut self, buffers: &[Buffer], rows: isize, cols: isize) -> Option<()> {
        let buffer = buffers.get(self.buffer)?;
        if let Some(line) = buffer.lines.get((self.cursor_row as isize + rows) as usize) {
            self.cursor_row = (self.cursor_row as isize + rows) as usize;
            self.cursor_col = ((self.cursor_col as isize + cols).max(0) as usize).min(line.len());
        } else if (-rows) > self.cursor_row as isize {
            self.cursor_row = 0;
            self.cursor_col = ((self.cursor_col as isize + cols).max(0) as usize).min(buffer.lines.get(0).map(|line| line.len()).unwrap_or(0));
        } else {
            self.cursor_row = buffer.lines.len();
            self.cursor_col = 0;
        }

        Some(())
    }
}

impl Editor {
    pub fn open(width: usize, height: usize) -> Self {
        Editor {
            buffers: vec![Buffer {lines: vec![Vec::new()]}],
            pane: Pane {
                width,
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
    pub fn move_cursor(&mut self, rows: isize, cols: isize) {
        self.pane.move_cursor(&self.buffers, rows, cols);
    }
    pub fn draw(&self, ui: &mut impl UI) {
        let img = self.pane.display(&self.buffers).expect("failed to produce image: buffer was likely closed prematurely");
        let mut total = 0;
        let mut drawn_cursor = false;
        for (y, line) in img.enumerate() {
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
            Editor::draw_cursor(ui);
        }
        for _ in total..self.pane.height-1 {
            ui.newln();
        }
    }
}
