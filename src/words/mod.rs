//! The primitives, grouped by kind of word.

use crate::*;

mod console;
mod control;
mod define;
mod double;
mod facility;
mod file;
mod memory;
mod stack;
mod string;
mod system;
mod tools;

/// Defines the system variables and all primitives, in this order.
pub(crate) fn install(f: &mut Forth) {
    f.variable("BASE", BASE_ADDR);
    f.variable("STATE", STATE_ADDR);
    f.variable(">IN", IN_ADDR);
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
}
