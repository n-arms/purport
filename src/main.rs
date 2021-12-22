macro_rules! log {
    ($fmt:expr $(, $more:expr )* ) => {
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open("log.txt")
            .unwrap();
        writeln!(file, $fmt $(, $more)*).unwrap();
    }
}

mod backend;
mod frontend;

use backend::editor::*;
use frontend::ui::*;
use frontend::unix_term::*;
use std::env::args;
use std::fs;

fn main() -> Result<(), UIError> {
    let mut term = Term::sys_default().expect("failed to spawn system default terminal");
    let mut ed = Editor::open(term.width(), term.height());

    let fp = args()
        .nth(1);
    
    ed.load_into(0, fp.clone());
    ed.draw(&mut term);
    term.refresh().expect("failed to refresh ui");

    loop {
        let ev = term.next_event()?;
        match ev {
            Event::SpecialChar(EscapeSeq::DownArrow) => ed.pane.move_cursor_up_down(&ed.buffers, 1),
            Event::SpecialChar(EscapeSeq::UpArrow) => ed.pane.move_cursor_up_down(&ed.buffers, -1),
            Event::SpecialChar(EscapeSeq::LeftArrow) => {
                ed.pane.move_cursor_left_right(&ed.buffers, -1)
            }
            Event::SpecialChar(EscapeSeq::RightArrow) => {
                ed.pane.move_cursor_left_right(&ed.buffers, 1)
            }
            Event::NormalChar('\x11') => break, // Ctrl-q
            Event::NormalChar('\x7f') => ed.pane.backspace(&mut ed.buffers),
            Event::NormalChar(c) => ed.pane.insert_char(&mut ed.buffers, c),
            _ => continue,
        };
        ed.draw(&mut term);
        term.refresh().expect("failed to refresh ui");
    }
    term.refresh().expect("failed to refresh ui");

    fp.map(|fp| {
        fs::write(
            fp,
            ed.buffers[0]
                .lines
                .iter()
                .map(|line| {
                    line.iter().fold(String::new(), |mut acc, x| {
                        acc.push(*x);
                        acc
                    })
                })
                .fold(String::new(), |mut acc, x| {
                    acc.push_str(&x);
                    acc.push('\n');
                    acc
                }),
        ).ok()
    });
    Ok(())
}
