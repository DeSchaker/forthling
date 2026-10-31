//! A small Forth interpreter, working towards Forth-2012 (see STANDAARD.md).
//!
//! Words are compiled to a list of instructions and run by an explicit
//! frame stack, so deep recursion in Forth never overflows the Rust stack.
//! Memory is byte-addressed with 64-bit little-endian cells. Everything a
//! Forth program may inspect (BASE, STATE, >IN, the input buffers) lives in
//! that memory, and errors are standard THROW codes.

mod compiler;
mod dictionary;
mod files;
mod heap;
mod interpreter;
mod memory;
mod parse;
mod run;
mod stack;
mod terminal;
mod throw;
mod words;

#[cfg(test)]
mod tests;

pub use memory::{DEFAULT_MEMORY, MAX_MEMORY, MIN_MEMORY};
pub use terminal::Input;
pub use throw::{Throw, QUIT};

// Every module starts with `use crate::*;`, so these make the items of all
// modules, and the std items most of them need, available everywhere.
pub(crate) use compiler::*;
pub(crate) use dictionary::*;
pub(crate) use files::*;
pub(crate) use heap::*;
pub(crate) use interpreter::*;
pub(crate) use memory::*;
pub(crate) use parse::*;
pub(crate) use run::*;
pub(crate) use stack::*;
pub(crate) use std::collections::{HashMap, HashSet};
pub(crate) use std::io::{self, BufRead};
pub(crate) use std::rc::Rc;
pub(crate) use terminal::*;
pub(crate) use throw::*;

pub type Cell = i64;
pub(crate) type Res<T = ()> = Result<T, Throw>;
pub(crate) type Prim = fn(&mut Forth) -> Res;

pub(crate) const CELL: usize = 8;

/// A complete Forth system. Its methods are spread over the modules: the
/// stacks in stack.rs, memory in memory.rs, the outer interpreter in
/// interpreter.rs, and so on; the primitives are in words/.
///
/// ```
/// let mut forth = forthling::Forth::new();
/// forth.interpret(": SQUARE DUP * ; 7 SQUARE .")?;
/// assert_eq!(forth.output(), b"49 ");
/// # Ok::<(), forthling::Throw>(())
/// ```
pub struct Forth {
    stack: Vec<Cell>,
    rstack: Vec<Cell>,
    locals: Vec<Cell>, // the locals of all frames; each frame knows where its own start
    loops: Vec<LoopFrame>,
    frames: Vec<Frame>,
    mem: Vec<u8>,
    here: usize,
    heap: Heap,
    dict: Vec<Word>,
    current: Option<usize>,
    code: Vec<Instr>,
    ctrl: Vec<Ctrl>,
    local_names: Vec<String>, // the locals of the definition being compiled
    locals_bound: usize,      // how many of them have their Instr::Locals compiled
    sources: Vec<Source>,
    sq_next: usize,
    hld: usize,                              // start of the pictured numeric output string
    input: Box<dyn Input>, // the terminal: the REPL, ACCEPT and KEY read from here
    interactive: bool,     // flush output before waiting for input
    catches: usize,        // CATCHes in progress
    substitutions: HashMap<String, Vec<u8>>, // REPLACES texts, by upper-case name
    files: Vec<Option<OpenFile>>, // open files, indexed by file id - 1
    included: HashSet<String>, // files INCLUDED so far, for REQUIRED
    out: Vec<u8>,
    bye: bool,
}

pub(crate) fn flag(b: bool) -> Cell {
    if b {
        -1
    } else {
        0
    }
}

impl Forth {
    /// A system with the default memory size (1 MiB).
    pub fn new() -> Self {
        Forth::with_memory(DEFAULT_MEMORY)
    }

    /// A system with `size` bytes of memory, shared by data space and the
    /// heap.
    ///
    /// # Panics
    ///
    /// When `size` is not between `MIN_MEMORY` and `MAX_MEMORY`, or not a
    /// whole number of cells (8 bytes).
    pub fn with_memory(size: usize) -> Self {
        let valid = (MIN_MEMORY..=MAX_MEMORY).contains(&size) && size % CELL == 0;
        assert!(valid, "invalid memory size: {size}");
        let mut f = Forth {
            stack: Vec::new(),
            rstack: Vec::new(),
            locals: Vec::new(),
            loops: Vec::new(),
            frames: Vec::new(),
            mem: vec![0; size],
            here: DICT_START,
            heap: Heap::new(size),
            dict: Vec::new(),
            current: None,
            code: Vec::new(),
            ctrl: Vec::new(),
            local_names: Vec::new(),
            locals_bound: 0,
            sources: vec![Source {
                origin: Origin::Terminal,
                buf: SOURCE_AREA,
                len: 0,
                line: 0,
                saved_in: 0,
            }],
            sq_next: 0,
            hld: HOLD_END,
            input: Box::<Stdin>::default(),
            interactive: false,
            catches: 0,
            substitutions: HashMap::new(),
            files: Vec::new(),
            included: HashSet::new(),
            out: Vec::new(),
            bye: false,
        };
        f.set(BASE_ADDR, 10);
        words::install(&mut f);
        f.interpret(include_str!("prelude.fs")).expect("prelude must compile");
        f
    }
}

impl Default for Forth {
    fn default() -> Self {
        Forth::new()
    }
}

/// The interface for programs that embed the interpreter, such as the REPL
/// in main.rs. `interpret`, `include`, `read_line` and `flush` are public too.
impl Forth {
    /// Flushes the output before waiting for terminal input, as a REPL should.
    pub fn set_interactive(&mut self, interactive: bool) {
        self.interactive = interactive;
    }

    /// Replaces the terminal input (standard input by default).
    pub fn set_input(&mut self, input: Box<dyn Input>) {
        self.input = input;
    }

    /// The output produced since the last `flush`.
    pub fn output(&self) -> &[u8] {
        &self.out
    }

    /// Whether BYE was executed.
    pub fn finished(&self) -> bool {
        self.bye
    }

    /// Whether a colon definition is still open, so the REPL says "compiled".
    pub fn in_definition(&self) -> bool {
        self.current.is_some()
    }
}
