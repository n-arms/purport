pub trait UI {
    fn draw(&mut self, text: &str);
    fn newln(&mut self);
    fn set_colour(&mut self, colour: Colour);
    fn next_event(&self) -> Event;
    fn width(&self) -> usize;
    fn height(&self) -> usize;
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
    Black
}
