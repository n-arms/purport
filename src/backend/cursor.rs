use super::editor::{Buffer, Error};
use std::convert::TryInto;

#[derive(Copy, Clone, Debug, Default)]
pub struct Offset {
    pub row: usize,
    pub col: usize,
}

impl Offset {
    #[allow(clippy::cast_sign_loss)]
    fn scroll_left_right(&mut self, dist: isize) {
        if dist >= 0 {
            self.col = self.col.saturating_add(dist as usize);
        } else {
            self.col = self.col.saturating_sub((-dist) as usize);
        }
    }

    #[allow(clippy::cast_sign_loss)]
    fn scroll_up_down(&mut self, dist: isize) {
        if dist >= 0 {
            self.row = self.row.saturating_add(dist as usize);
        } else {
            self.row = self.row.saturating_sub((-dist) as usize);
        }
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
        width: usize,
        dist: isize,
    ) -> Result<(), Error> {
        if let Some(line) = buffer.lines.get(self.row) {
            if dist > 0 {
                // the distance from the screen pos of the cursor to the right edge of the screen
                // = screen width - position of cursor on screen
                if self.col < offset.col {
                    return Err(Error::OffsetGreaterThanCursor {
                        cursor: self.col,
                        offset: offset.col
                    });
                }
                if width < self.col - offset.col {
                    return Err(Error::CursorOffScreen {
                        cursor_on_screen: self.col - offset.col,
                        screen_size: width
                    });
                }
                let old_edge_dist = width - (self.col - offset.col);
                // the distance the cursor needs to be moved to the right
                if line.len() < self.col {
                    return Err(Error::CursorPastEnd {
                        cursor: self.col,
                        pos: line.len()
                    });
                }
                #[allow(clippy::cast_sign_loss)]
                let dist_right = (dist as usize).min(line.len() - self.col);
                // if the distance to be moved is more than the available space to move
                if old_edge_dist <= dist_right {
                    #[allow(clippy::expect_used)]
                    offset.scroll_left_right(
                        (dist_right - old_edge_dist)
                            .saturating_add(1) // the terminal isnt going to be usize::MAX wide
                            .try_into()
                            .expect("overflow on scroll"),
                    );
                }
                self.col = self.col.checked_add(dist_right).expect("overflow on scroll");
            } else {
                // the distance from the screen pos of the cursor to the left edge of the screen
                // = cursor column - offset column
                if self.col < offset.col {
                    return Err(Error::OffsetGreaterThanCursor {
                        cursor: self.col,
                        offset: offset.col
                    });
                }
                let old_edge_dist = self.col - offset.col;
                // the distance the cursor needs to be moved to the left
                #[allow(clippy::cast_sign_loss)]
                let dist_left = (-dist as usize).min(self.col); // !dist > 0 -> dist < 0 -> -dist > 0
                // if the distance to be moved is more than the available space to move
                if old_edge_dist < dist_left {
                    #[allow(clippy::expect_used)]
                    offset.scroll_left_right(-TryInto::<isize>::try_into(dist_left - old_edge_dist).expect("overflow on scroll"));
                }
                self.col -= dist_left; // dist_left is at most col, col - col = 0
            }
        }
        Ok(())
    }

    pub fn move_up_down(
        &mut self,
        buffer: &Buffer,
        offset: &mut Offset,
        height: usize,
        _width: usize,
        dist: isize,
    ) -> Result<(), Error> {
        if dist > 0 {
            if self.row < offset.row {
                return Err(Error::OffsetGreaterThanCursor {
                    cursor: self.row,
                    offset: offset.row
                });
            }
            if height < (self.row - offset.row) {
                return Err(Error::CursorOffScreen {
                    cursor_on_screen: self.row - offset.row,
                    screen_size: height
                });
            }
            let old_edge_dist = height - (self.row - offset.row);
            if buffer.lines.len() <= self.row {
                return Err(Error::CursorPastEnd{
                    pos: buffer.lines.len(),
                    cursor: self.row
                });
            }
            #[allow(clippy::cast_sign_loss)]
            let dist_down = (dist as usize).min(buffer.lines.len() - self.row - 1);
            if old_edge_dist <= dist_down {
                #[allow(clippy::expect_used)]
                offset.scroll_up_down(
                    (dist_down - old_edge_dist)
                        .saturating_add(1) // the terminal won't be usize::MAX tall
                        .try_into()
                        .expect("overflow on scroll"),
                );
            }
            self.row = self.row.checked_add(dist_down).expect("overflow on scroll");
        } else {
            if self.row < offset.row {
                return Err(Error::OffsetGreaterThanCursor {
                    offset: offset.row,
                    cursor: self.row
                });
            }
            let old_edge_dist = self.row - offset.row;

            #[allow(clippy::cast_sign_loss)]
            let dist_up = (-dist as usize).min(self.row); // dist < 0 -> -dist > 0

            if old_edge_dist <= dist_up {
                #[allow(clippy::expect_used)]
                offset.scroll_up_down(-TryInto::<isize>::try_into(dist_up - old_edge_dist).expect("overflow")); // dist_up - old_edge_dist > 0, so negative is sound
            }
            self.row -= dist_up; // dist up is at most self.row, self.row - self.row = 0
        }
        let new_line_len = buffer
            .lines
            .get(self.row)
            .map_or(0, Vec::len);
        if new_line_len < self.col {
            self.col = new_line_len;
            if self.col < offset.col {
                offset.scroll_left_right(-TryInto::<isize>::try_into(offset.col - self.col).expect("overflow"));
            }
        }
        Ok(())
    }
}
