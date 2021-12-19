use super::ui::*;

#[derive(Clone, Debug)]
pub struct Term {
    width: usize,
    height: usize,
}

impl UI for Term {
    fn draw(&mut self, text: &str) {
        todo!()
    }
    fn height(&self) -> usize {
        todo!()
    }
    fn width(&self) -> usize {
        todo!()
    }
    fn newln(&mut self) {
        todo!()
    }
    fn next_event(&self) -> Event {
        todo!()
    }
    fn set_colour(&mut self, colour: Colour) {
        todo!()
    }
}
