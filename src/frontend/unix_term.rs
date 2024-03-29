use super::ui::{Colour, Error, EscapeSeq, Event, UI};
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
    cursor_col: usize,
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
        #[allow(clippy::indexing_slicing)]
        self.buffer[self.row].push_str(text);
    }
    fn height(&self) -> usize {
        self.height
    }
    fn width(&self) -> usize {
        self.width
    }
    fn newln(&mut self) {
        self.row = self.row.saturating_add(1);
        assert!(
            !(self.row >= self.height),
            "call to increase current row beyond the max"
        );
    }
    fn next_event(&mut self) -> Result<Event, Error> {
        let c = io::stdin()
            .bytes()
            .nth(0)
            .ok_or(Error::FailedStdinRead)?
            .map_err(Error::IOErr)?;
        if c == b'\x1b' {
            let mut esc = String::new();
            for byte in io::stdin().bytes() {
                esc.push(byte.map(|b| b as char).map_err(Error::IOErr)?);
                match esc.as_str() {
                    "[A" => return Ok(Event::SpecialChar(EscapeSeq::UpArrow)),
                    "[B" => return Ok(Event::SpecialChar(EscapeSeq::DownArrow)),
                    "[C" => return Ok(Event::SpecialChar(EscapeSeq::RightArrow)),
                    "[D" => return Ok(Event::SpecialChar(EscapeSeq::LeftArrow)),
                    _ => (),
                }
            }
            Err(Error::FailedStdinRead)
        } else {
            Ok(Event::NormalChar(c as char))
        }
    }
    fn set_foreground(&mut self, colour: Colour) {
        #[allow(clippy::indexing_slicing)]
        self.buffer[self.row].push_str(match colour {
            Colour::Reset => "\x1b[0m",
            Colour::Black => "\x1b[30m",
            Colour::Red => "\x1b[31m",
            Colour::Green => "\x1b[32m",
            Colour::Yellow => "\x1b[33m",
            Colour::Blue => "\x1b[34m",
            Colour::Magenta => "\x1b[35m",
            Colour::Cyan => "\x1b[36m",
            Colour::White => "\x1b[37m",
        });
    }
    fn set_background(&mut self, colour: Colour) {
        #[allow(clippy::indexing_slicing)]
        self.buffer[self.row].push_str(match colour {
            Colour::Reset => "\x1b[0m",
            Colour::Black => "\x1b[40m",
            Colour::Red => "\x1b[41m",
            Colour::Green => "\x1b[42m",
            Colour::Yellow => "\x1b[43m",
            Colour::Blue => "\x1b[44m",
            Colour::Magenta => "\x1b[45m",
            Colour::Cyan => "\x1b[46m",
            Colour::White => "\x1b[47m",
        });
    }
    fn refresh(&mut self) -> Result<(), Error> {
        print!("\x1b[?25l\x1b[H");
        io::stdout().flush().map_err(Error::IOErr)?;
        self.row = 0;
        let max = self.buffer.len();
        #[allow(clippy::integer_arithmetic)]
        for (row, line) in self.buffer.iter_mut().enumerate() {
            print!(
                "\x1b[{};1H{}{}",
                row + 1,
                line,
                if row + 1 == max { "" } else { "\r\n" }
            );
            *line = String::new();
        }
        print!("\x1b[?25h\x1b[{};{}H", self.cursor_row, self.cursor_col);
        io::stdout().flush().map_err(Error::IOErr)
    }
}

impl Term {
    fn cleanup() -> io::Result<()> {
        print!("\x1b[?25h\x1b[2J\x1b[;H");
        io::stdout().flush()?;

        Command::new("stty")
            .arg("sane")
            .stdin(Stdio::inherit())
            .output()?;
        Ok(())
    }

    pub fn sys_default() -> Result<Self, Error> {
        let raw_cmd = Command::new("stty")
            .arg("-echo")
            .arg("raw")
            .stdin(Stdio::inherit())
            .output()
            .map_err(Error::IOErr)?;
        if !raw_cmd.status.success() {
            Term::cleanup().map_err(Error::IOErr)?;
            return Err(Error::ProcFailed(raw_cmd.status));
        }
        let lines_cmd = Command::new("tput")
            .arg("lines")
            .output()
            .map_err(Error::IOErr)?;
        if !lines_cmd.status.success() {
            Term::cleanup().map_err(Error::IOErr)?;
            return Err(Error::ProcFailed(lines_cmd.status));
        }
        let cols_cmd = Command::new("tput")
            .arg("cols")
            .output()
            .map_err(Error::IOErr)?;
        if !cols_cmd.status.success() {
            Term::cleanup().map_err(Error::IOErr)?;
            return Err(Error::ProcFailed(cols_cmd.status));
        }
        #[allow(clippy::indexing_slicing)]
        let height = String::from_utf8_lossy(
            &lines_cmd.stdout[..lines_cmd.stdout.len().checked_sub(1).ok_or_else(|| {
                Error::MissingSystemReq(String::from("`tput lines` gave empty output"))
            })?],
        )
        .parse()
        .map_err(|err| {
            Error::MissingSystemReq(format!(
                "failed to parse `tput lines` as a usize: {:?}",
                err
            ))
        })?;
        #[allow(clippy::integer_arithmetic)]
        if height >= (usize::MAX - 1) {
            return Err(Error::UnreasonableDimensions {
                width: None,
                height: Some(height),
            });
        }
        #[allow(clippy::indexing_slicing)]
        let width = String::from_utf8_lossy(
            &cols_cmd.stdout[..cols_cmd.stdout.len().checked_sub(1).ok_or_else(|| {
                Error::MissingSystemReq(String::from("`tput cols` gave empty output"))
            })?],
        )
        .parse()
        .map_err(|err| {
            Error::MissingSystemReq(format!("failed to parse `tput cols` as a usize: {:?}", err))
        })?;
        #[allow(clippy::integer_arithmetic)]
        if width >= (usize::MAX - 1) {
            return Err(Error::UnreasonableDimensions {
                width: Some(width),
                height: Some(height),
            });
        }
        Ok(Term {
            width,
            height,
            buffer: vec![String::new(); height],
            cursor_col: 0,
            cursor_row: 0,
            row: 0,
        })
    }
}

impl ops::Drop for Term {
    fn drop(&mut self) {
        #[allow(clippy::expect_used)]
        Term::cleanup().expect("failed to cleanup term");
    }
}
