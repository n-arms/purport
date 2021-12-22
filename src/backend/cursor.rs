use super::editor::Buffer;

#[derive(Copy, Clone, Debug, Default)]
pub struct Offset {
    pub row: usize,
    pub col: usize,
}

impl Offset {
    fn scroll_left_right(&mut self, dist: isize) {
        self.col = (self.col as isize + dist) as usize
    }

    fn scroll_up_down(&mut self, dist: isize) {
        self.row = (self.row as isize + dist) as usize
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
}

impl Cursor {
    pub fn move_left_right(
        &mut self,
        buffer: &Buffer,
        offset: &mut Offset,
        width: &usize,
        dist: isize,
    ) {
        if let Some(line) = buffer.lines.get(self.row) {
            if dist > 0 {
                // the distance from the screen pos of the cursor to the right edge of the screen
                let old_edge_dist = width - (self.col - offset.col);
                // the distance the cursor needs to be moved to the right
                let dist_right = (dist as usize).min(line.len() - self.col);
                // if the distance to be moved is more than the available space to move
                if old_edge_dist <= dist_right {
                    offset.scroll_left_right((1 + dist_right - old_edge_dist) as isize);
                }
                self.col += dist_right;
            } else {
                // the distance from the screen pos of the cursor to the left edge of the screen
                let old_edge_dist = self.col - offset.col;
                // the distance the cursor needs to be moved to the left
                let dist_left = -(dist.max(-(self.col as isize))) as usize;
                // if the distance to be moved is more than the available space to move
                if old_edge_dist < dist_left {
                    offset.scroll_left_right(-((dist_left - old_edge_dist) as isize));
                }
                self.col -= dist_left;
            }
        }
    }

    pub fn move_up_down(
        &mut self,
        buffer: &Buffer,
        offset: &mut Offset,
        height: &usize,
        dist: isize,
    ) {
        if dist > 0 {
            let old_edge_dist = height - (self.row - offset.row);
            let dist_down = (dist as usize).min(buffer.lines.len() - self.row - 1);
            if old_edge_dist <= dist_down {
                offset.scroll_up_down((1 + dist_down - old_edge_dist) as isize);
            }
            self.row += dist_down;
            let new_line_len = buffer
                .lines
                .get(self.row)
                .map(|line| line.len())
                .unwrap_or(0);
            if new_line_len < self.col {
                self.col = new_line_len;
            }
        } else {
            let old_edge_dist = self.row - offset.row;
            let dist_up = -(dist.max(-(self.row as isize))) as usize;
            if old_edge_dist <= dist_up {
                offset.scroll_up_down(-((dist_up - old_edge_dist) as isize));
            }
            self.row -= dist_up;
            let new_line_len = buffer
                .lines
                .get(self.row)
                .map(|line| line.len())
                .unwrap_or(0);
            if new_line_len < self.col {
                self.col = new_line_len;
            }
        }
    }
}
