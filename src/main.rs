#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::implicit_return,
    clippy::else_if_without_else,
    clippy::missing_docs_in_private_items,
    clippy::unused_unit,
    clippy::pattern_type_mismatch,
    clippy::integer_arithmetic
)]

mod backend;
mod frontend;

use backend::editor::Editor;
use frontend::ui::UI;
use frontend::unix_term::Term;
use std::env::args;

fn main() {
    let mut term = Term::sys_default().expect("failed to spawn system default terminal");
    let mut ed = Editor::open(term.width(), term.height());

    let fp = args().nth(1);

    ed.load_into(0, fp);
    ed.draw(&mut term).expect("failed to get next event");
    term.refresh().expect("failed to refresh ui");

    loop {
        let ev = term.next_event().expect("failed to get next event");
        if ed.process_event(ev).expect("error in editor") {
            break;
        }
        ed.draw(&mut term).expect("failed to draw to term");
        term.refresh().expect("failed to refresh ui");
    }
    term.refresh().expect("failed to refresh ui");
}
