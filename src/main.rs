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

use backend::editor::{Editor, Error};
use frontend::unix_term::Term;
use std::env::args;

fn main() -> Result<(), Error> {
    let term = Term::sys_default().map_err(Error::UIErr)?;
    let mut ed = Editor::open(term)?;

    let fp = args().nth(1);
    ed.load_into(1, fp);
    ed.mainloop()?;

    Ok(())
}

/*
use std::io::{self, Read, Write};
fn main() {
    let term = Term::sys_default().unwrap();
    for (byte, _) in io::stdin().bytes().zip(0..10) {
        println!("{:?}", byte);
    }
    drop(term);
}
*/
