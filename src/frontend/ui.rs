use std::io;
use std::process::ExitStatus;
use std::str;

pub trait UI {
    fn draw(&mut self, text: &str);
    fn newln(&mut self);
    fn set_foreground(&mut self, colour: Colour);
    fn set_background(&mut self, colour: Colour);
    fn next_event(&mut self) -> Result<Event, Error>;
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn refresh(&mut self) -> Result<(), Error>;
    fn move_cursor(&mut self, row: usize, col: usize);
    fn drawln(&mut self, text: &str) {
        self.draw(text);
        self.newln();
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    NormalChar(char),
    SpecialChar(EscapeSeq),
}

#[allow(clippy::enum_variant_names)]
#[derive(Clone, Debug, Copy)]
pub enum EscapeSeq {
    LeftArrow,
    RightArrow,
    UpArrow,
    DownArrow,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Colour {
    White,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Reset,
}

#[derive(Debug)]
pub struct IsNotColour(String);

impl str::FromStr for Colour {
    type Err = IsNotColour;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "white" => Ok(Colour::White),
            "black" => Ok(Colour::Black),
            "red" => Ok(Colour::Red),
            "yellow" => Ok(Colour::Yellow),
            "green" => Ok(Colour::Green),
            "blue" => Ok(Colour::Blue),
            "cyan" => Ok(Colour::Cyan),
            "magenta" => Ok(Colour::Magenta),
            "reset" => Ok(Colour::Reset),
            _ => Err(IsNotColour(s.to_string())),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    FailedStdinRead,
    IOErr(io::Error),
    ProcFailed(ExitStatus),
    MissingSystemReq(String),
    UnreasonableDimensions {
        width: Option<usize>,
        height: Option<usize>,
    },
}
