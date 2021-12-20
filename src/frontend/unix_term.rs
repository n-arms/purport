use super::ui::*;
use std::io::{self, Write, Read};
use std::process::{Command, Stdio};
use std::ops;

#[derive(Clone, Debug)]
pub struct Term {
    width: usize,
    height: usize,
}

impl UI for Term {
    fn draw(&mut self, text: &str) {
        print!("{}", text);
        io::stdout().flush().expect("failed to flush stdout");
    }
    fn height(&self) -> usize {
        self.height
    }
    fn width(&self) -> usize {
        self.width
    }
    fn newln(&mut self) {
        println!("\r");
    }
    fn next_event(&mut self) -> Result<Event, UIError> {
        let c = io::stdin().bytes().nth(0).ok_or(UIError::FailedStdinRead)?.map_err(|_| UIError::FailedStdinRead)?;
        if c == '\x1b' as u8 {
            let mut esc = String::new();
            for byte in io::stdin().bytes() {
                esc.push(byte.map(|b| b as char).map_err(|_| UIError::FailedStdinRead)?);
                match esc.as_str() {
                    "[A" => return Ok(Event::SpecialChar(EscapeSeq::UpArrow)),
                    "[B" => return Ok(Event::SpecialChar(EscapeSeq::DownArrow)),
                    "[C" => return Ok(Event::SpecialChar(EscapeSeq::RightArrow)),
                    "[D" => return Ok(Event::SpecialChar(EscapeSeq::LeftArrow)),
                    _ => ()
                }
            }
            Err(UIError::FailedStdinRead)
        } else {
            Ok(Event::NormalChar(c as char))
        }
    }
    fn clear(&mut self) {
        print!("\x1b[2J\x1b[1;1H");
        io::stdout().flush().expect("failed to flush stdout");
    }
    fn set_foreground(&mut self, colour: Colour) {
        match colour {
            Colour::Black => print!("\x1b[30m"),
            Colour::White => print!("\x1b[37m"),
            Colour::Red => print!("\x1b[31m"),
            Colour::Reset => print!("\x1b[0m")
        }
    }
    fn set_background(&mut self, colour: Colour) {
        match colour {
            Colour::Black => print!("\x1b[40m"),
            Colour::White => print!("\x1b[47m"),
            Colour::Red => print!("\x1b[41m"),
            Colour::Reset => print!("\x1b[0m")
        }
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
        print!("\x1b[?25l");
        io::stdout().flush().expect("failed to flush stdout");

        let raw_cmd = Command::new("stty")
            .arg("-echo")
            .arg("raw")
            .stdin(Stdio::inherit())
            .output()
            .map_err(|e| UIError::IOErr(e))?;
        if !raw_cmd.status.success() {
            Term::cleanup();
            return Err(UIError::ProcFailed(raw_cmd.status));
        }

        let lines_cmd = Command::new("tput")
            .arg("lines")
            .output()
            .map_err(|e| UIError::IOErr(e))?;
        if !lines_cmd.status.success() {
            Term::cleanup();
            return Err(UIError::ProcFailed(lines_cmd.status));
        }
        let cols_cmd = Command::new("tput")
            .arg("cols")
            .output()
            .map_err(|e| UIError::IOErr(e))?;
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
            height
        })
    }
}

impl ops::Drop for Term {
    fn drop(&mut self) {
        Term::cleanup()
    }
}
