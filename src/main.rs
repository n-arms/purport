mod backend;
mod frontend;

use frontend::ui::*;
use frontend::unix_term::*;

fn main() {
    let mut term = Term::sys_default().expect("failed to spawn system default terminal");
    term.clear();
    term.drawln("123456");
    term.set_background(Colour::White);
    term.draw(" ");
    term.set_background(Colour::Black);
    term.set_foreground(Colour::Red);
    term.drawln("789");
}
