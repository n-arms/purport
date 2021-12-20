mod backend;
mod frontend;

use frontend::ui::*;
use frontend::unix_term::*;
use backend::editor::*;
use std::fs;
use std::env::args;

fn main() -> Result<(), UIError> {
    let mut term = Term::sys_default().expect("failed to spawn system default terminal");
    let mut ed = Editor::open(term.width(), term.height());

    let fp = args().nth(1).ok_or(UIError::MissingSystemReq(String::from("missing command line arg")))?;
    let file = fs::read_to_string(fp.clone()).map_err(|e| UIError::IOErr(e))?;
    ed.load_into(0, file);
    ed.draw(&mut term);

    loop {
        let ev = term.next_event()?;
        match ev {
            Event::SpecialChar(EscapeSeq::DownArrow) => ed.pane.move_cursor(&ed.buffers, 1, 0),
            Event::SpecialChar(EscapeSeq::UpArrow) => ed.pane.move_cursor(&ed.buffers, -1, 0),
            Event::SpecialChar(EscapeSeq::LeftArrow) => ed.pane.move_cursor(&ed.buffers, 0, -1),
            Event::SpecialChar(EscapeSeq::RightArrow) => ed.pane.move_cursor(&ed.buffers, 0, 1),
            Event::NormalChar('\x11') => break,
            Event::NormalChar('\x7f') => ed.pane.backspace(&mut ed.buffers),
            Event::NormalChar(c) => ed.pane.insert_char(&mut ed.buffers, c),
            _ => continue
        };
        term.clear();
        term.refresh().expect("failed to refresh ui");
        ed.draw(&mut term);
    }
    term.clear();
    term.refresh().expect("failed to refresh ui");

    fs::write(fp, ed.buffers[0]
              .lines
              .iter()
              .map(|line| line
                   .iter()
                   .fold(String::new(), |mut acc, x| {
                       acc.push(*x);
                       acc
                   }))
              .fold(String::new(), |mut acc, x| {
                  acc.push_str(&x);
                  acc.push('\n');
                  acc
              })).map_err(|e| UIError::IOErr(e))?;
    Ok(())
}
