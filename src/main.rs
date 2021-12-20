mod backend;
mod frontend;

use frontend::ui::*;
use frontend::unix_term::*;
use backend::editor::*;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let mut term = Term::sys_default().expect("failed to spawn system default terminal");
    let mut ed = Editor::open(term.width(), term.height());
    ed.draw(&mut term);
    sleep(Duration::from_secs(2));
    term.clear();
    ed.load_into(0, "introducing\npurport\nit claims it is a text editor".to_string());
    ed.draw(&mut term);
    sleep(Duration::from_secs(2));
    term.clear();
}
