//! The primitives, grouped by kind of word.

use crate::*;

mod block;
mod console;
mod control;
mod define;
mod double;
mod facility;
mod file;
mod float;
mod heap;
mod inspect;
mod locals;
mod memory;
mod search;
mod stack;
mod string;
mod system;
mod tools;
mod xchar;

/// Defines the system variables and all primitives, in this order.
pub(crate) fn install(f: &mut Forth) {
    f.variable("BASE", BASE_ADDR);
    f.variable("STATE", STATE_ADDR);
    f.variable(">IN", IN_ADDR);
    f.variable("BLK", BLK_ADDR);
    f.variable("SCR", SCR_ADDR);
    stack::install(f);
    double::install(f);
    memory::install(f);
    console::install(f);
    define::install(f);
    control::install(f);
    string::install(f);
    tools::install(f);
    system::install(f);
    file::install(f);
    facility::install(f);
    heap::install(f);
    inspect::install(f);
    locals::install(f);
    search::install(f);
    block::install(f);
    xchar::install(f);
    float::install(f);
}
