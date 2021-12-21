use super::ui::*;
use std::io::{self, Read, Write};
use std::ops;
use std::process::{Command, Stdio};

#[derive(Clone, Debug)]
pub struct Term {
    width: usize,
    height: usize,
    buffer: Vec<String>,
    row: usize,
    cursor_row: usize,
    cursor_col: usize
}
/* for every line to be printed:
collect all the data
on refresh:
    hide cursor
    for each line:
        jump to the start of the line
        print out the line
    jump cursor to new location
    show cursor
*/
impl UI for Term {
    fn move_cursor(&mut self, row: usize, col: usize) {
        self.cursor_col = col;
        self.cursor_row = row;
    }
    fn draw(&mut self, text: &str) {
        self.buffer[self.row].push_str(text);
    }
    fn height(&self) -> usize {
        self.height
    }
    fn width(&self) -> usize {
        self.width
    }
    fn newln(&mut self) {
        self.row += 1;
        if self.row == self.height {
            panic!("call to increase current row beyond the max");
        }
    }
    fn next_event(&mut self) -> Result<Event, UIError> {
        let c = io::stdin()
            .bytes()
            .nth(0)
            .ok_or(UIError::FailedStdinRead)?
            .map_err(|_| UIError::FailedStdinRead)?;
        if c == b'\x1b' {
            let mut esc = String::new();
            for byte in io::stdin().bytes() {
                esc.push(
                    byte.map(|b| b as char)
                        .map_err(|_| UIError::FailedStdinRead)?,
                );
                match esc.as_str() {
                    "[A" => return Ok(Event::SpecialChar(EscapeSeq::UpArrow)),
                    "[B" => return Ok(Event::SpecialChar(EscapeSeq::DownArrow)),
                    "[C" => return Ok(Event::SpecialChar(EscapeSeq::RightArrow)),
                    "[D" => return Ok(Event::SpecialChar(EscapeSeq::LeftArrow)),
                    _ => (),
                }
            }
            Err(UIError::FailedStdinRead)
        } else {
            Ok(Event::NormalChar(c as char))
        }
    }
    fn set_foreground(&mut self, colour: Colour) {
        self.buffer[self.row].push_str(match colour {
            Colour::Black => "\x1b[30m",
            Colour::White => "\x1b[37m",
            Colour::Red => "\x1b[31m",
            Colour::Reset => "\x1b[0m",
        });
    }
    fn set_background(&mut self, colour: Colour) {
        self.buffer[self.row].push_str(match colour {
            Colour::Black => "\x1b[40m",
            Colour::White => "\x1b[47m",
            Colour::Red => "\x1b[41m",
            Colour::Reset => "\x1b[0m",
        });
    }
    fn refresh(&mut self) -> Result<(), UIError> {
        print!("\x1b[?25l\x1b[H");
        io::stdout().flush().map_err(UIError::IOErr)?;
        self.row = 0;
        let max = self.buffer.len();
        for (row, line) in self.buffer.iter_mut().enumerate() {
            print!("\x1b[{};1H{}{}", row + 1, line, if row + 1 == max {""} else {"\r\n"});
            *line = String::new();
        }
        print!("\x1b[?25h\x1b[{};{}H", self.cursor_row, self.cursor_col);
        io::stdout().flush().map_err(UIError::IOErr)
    }
}

impl Term {
    fn cleanup() {
        print!("\x1b[?25h");
        io::stdout().flush().expect("failed to flush stdout");

        let sane_cmd = Command::new("stty")
            .arg("sane")
            .stdin(Stdio::inherit())
            .output();

        sane_cmd.expect("stty sane failed");
    }

    pub fn sys_default() -> Result<Self, UIError> {
        let raw_cmd = Command::new("stty")
            .arg("-echo")
            .arg("raw")
            .stdin(Stdio::inherit())
            .output()
            .map_err(UIError::IOErr)?;
        if !raw_cmd.status.success() {
            Term::cleanup();
            return Err(UIError::ProcFailed(raw_cmd.status));
        }
        let lines_cmd = Command::new("tput")
            .arg("lines")
            .output()
            .map_err(UIError::IOErr)?;
        if !lines_cmd.status.success() {
            Term::cleanup();
            return Err(UIError::ProcFailed(lines_cmd.status));
        }
        let cols_cmd = Command::new("tput")
            .arg("cols")
            .output()
            .map_err(UIError::IOErr)?;
        if !cols_cmd.status.success() {
            Term::cleanup();
            return Err(UIError::ProcFailed(cols_cmd.status));
        }
        let height = String::from_utf8_lossy(&lines_cmd.stdout[..lines_cmd.stdout.len() - 1])
            .parse()
            .map_err(|_| UIError::MissingSystemReq(String::from("tput lines")))?;
        let width = String::from_utf8_lossy(&cols_cmd.stdout[..cols_cmd.stdout.len() - 1])
            .parse()
            .map_err(|_| UIError::MissingSystemReq(String::from("tput cols")))?;
        Ok(Term {
            width,
            height,
            buffer: vec![String::new(); height],
            cursor_col: 0,
            cursor_row: 0,
            row: 0
        })
    }
}

impl ops::Drop for Term {
    fn drop(&mut self) {
        Term::cleanup()
    }
}
