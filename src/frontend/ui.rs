use std::io;
use std::process::ExitStatus;

pub trait UI {
    fn draw(&mut self, text: &str);
    fn newln(&mut self);
    fn set_foreground(&mut self, colour: Colour);
    fn set_background(&mut self, colour: Colour);
    fn next_event(&mut self) -> Result<Event, UIError>;
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn clear(&mut self);
    fn refresh(&mut self) -> Result<(), UIError>;
    fn drawln(&mut self, text: &str) {
        self.draw(text);
        self.newln();
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    NormalChar(char),
    SpecialChar(EscapeSeq),
    Resize
}

#[derive(Clone, Debug)]
pub enum EscapeSeq {
    LeftArrow,
    RightArrow,
    UpArrow,
    DownArrow
}

#[derive(Clone, Debug)]
pub enum Colour {
    White,
    Black,
    Red,
    Reset
}

#[derive(Debug)]
pub enum UIError {
    FailedStdinRead,
    IOErr(io::Error),
    ProcFailed(ExitStatus),
    MissingSystemReq(String)
}
