mod backend;
mod frontend;

use frontend::ui::*;
use frontend::unix_term::*;
use backend::editor::*;

fn main() -> Result<(), UIError> {
    let mut term = Term::sys_default().expect("failed to spawn system default terminal");
    let mut ed = Editor::open(term.width(), term.height());
    ed.load_into(0, "introducing\npurport\nit claims it is a text editor".to_string());
    ed.draw(&mut term);

    loop {
        let ev = term.next_event()?;
        match ev {
            Event::SpecialChar(EscapeSeq::DownArrow) => ed.move_cursor(1, 0),
            Event::SpecialChar(EscapeSeq::UpArrow) => ed.move_cursor(-1, 0),
            Event::SpecialChar(EscapeSeq::LeftArrow) => ed.move_cursor(0, -1),
            Event::SpecialChar(EscapeSeq::RightArrow) => ed.move_cursor(0, 1),
            Event::NormalChar('q') => break,
            _ => continue
        }
        term.clear();
        ed.draw(&mut term);
    }
    term.clear();
    Ok(())
}
