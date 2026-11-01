//! A small Forth interpreter, working towards Forth-2012 (see COMPLIANCE.md).
//!
//! Words are compiled to a list of instructions and run by an explicit
//! frame stack, so deep recursion in Forth never overflows the Rust stack.
//! Memory is byte-addressed with 64-bit little-endian cells. Everything a
//! Forth program may inspect (BASE, STATE, >IN, the input buffers) lives in
//! that memory, and errors are standard THROW codes.

// Lints of clippy::pedantic that the code is clean for; `clippy -D warnings` in CI
// keeps it so. (The rest of pedantic is not enabled: see the open issues.)
#![warn(
    clippy::cast_lossless,
    clippy::doc_markdown,
    clippy::float_cmp,
    clippy::format_push_string,
    clippy::many_single_char_names,
    clippy::match_same_arms,
    clippy::must_use_candidate,
    clippy::redundant_closure_for_method_calls,
    clippy::return_self_not_must_use,
    clippy::semicolon_if_nothing_returned,
    clippy::single_match_else,
    clippy::struct_excessive_bools,
    clippy::unnecessary_literal_bound,
    clippy::uninlined_format_args
)]

mod blocks;
pub mod cli;
mod compiler;
mod debug;
mod dictionary;
mod files;
mod float;
mod functional;
mod heap;
mod host;
mod interpreter;
mod json;
mod memory;
mod parse;
mod report;
mod run;
mod sandbox;
mod stack;
mod terminal;
mod throw;
mod trace;
mod tutorial;
mod typed;
mod vfs;
mod words;
mod xchar;

#[cfg(test)]
mod tests;

pub use debug::{DebugHook, Pause, Reason, Resume};
pub use functional::Functional;
pub use host::Extension;
pub use json::Json;
pub use memory::{DEFAULT_MEMORY, MAX_MEMORY, MIN_MEMORY};
pub use report::ErrorContext;
pub use sandbox::{Limits, Permission};
pub use terminal::Input;
pub use throw::{LIMIT_EXCEEDED, NOT_PERMITTED, QUIT, Throw};
pub use trace::{Debugger, TRACE_CELLS, TraceEntry};
pub use tutorial::{Lesson, Tutorial};
pub use typed::{FromForth, IntoForth};
pub use vfs::{Access, FileHandle, FileSystem, MemoryFs, NativeFs};

// Every module starts with `use crate::*;`, so these make the items of all
// modules, and the std items most of them need, available everywhere.
pub(crate) use blocks::*;
pub(crate) use compiler::*;
pub(crate) use debug::*;
pub(crate) use dictionary::*;
pub(crate) use files::*;
pub(crate) use float::*;
pub(crate) use heap::*;
pub(crate) use host::*;
pub(crate) use interpreter::*;
pub(crate) use memory::*;
pub(crate) use parse::*;
pub(crate) use run::*;
pub(crate) use sandbox::*;
pub(crate) use stack::*;
pub(crate) use std::cell::RefCell;
pub(crate) use std::collections::{HashMap, HashSet};
pub(crate) use std::io::{self, BufRead};
pub(crate) use std::rc::Rc;
pub(crate) use terminal::*;
pub(crate) use throw::*;
pub(crate) use trace::*;
pub(crate) use xchar::*;

pub type Cell = i64;
/// The result of a Forth operation: a value, or the exception it threw.
pub type Res<T = ()> = Result<T, Throw>;
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
    fstack: Vec<f64>, // the floating-point stack
    precision: usize, // the significant digits of F. FE. FS. (SET-PRECISION)
    rstack: Vec<Cell>,
    locals: Vec<Cell>, // the locals of all frames; each frame knows where its own start
    loops: Vec<LoopFrame>,
    frames: Vec<Frame>,
    mem: Vec<u8>,
    here: usize,
    heap: Heap,
    dict: Vec<Word>,
    wordlists: Vec<Option<String>>, // by wid - 1: the name, if the list has one
    order: Vec<usize>,              // the search order, first searched first
    current_wid: usize,             // the word list new definitions go into
    current: Option<usize>,
    encoding_name: Option<usize>, // where XCHAR-ENCODING keeps its answer, once asked
    fence: usize,                 // the words of the system itself, which FORGET leaves alone
    code: Vec<Instr>,
    ctrl: Vec<Ctrl>,
    quotes: Vec<Quote>, // the definitions around the quotation being compiled
    local_names: Vec<String>, // the locals of the definition being compiled
    locals_bound: usize, // how many of them have their Instr::Locals compiled
    sources: Sources,
    sq_next: usize,
    hld: usize,                              // start of the pictured numeric output string
    input: Box<dyn Input>, // the terminal: the REPL, ACCEPT and KEY read from here
    interactive: bool,     // flush output before waiting for input
    catches: usize,        // CATCHes in progress
    substitutions: HashMap<String, Vec<u8>>, // REPLACES texts, by upper-case name
    files: Vec<Option<OpenFile>>, // open files, indexed by file id - 1
    blocks: Blocks,        // the block file and the block buffers
    vfs: Box<dyn FileSystem>, // where the files are (vfs.rs)
    included: HashSet<String>, // files INCLUDED so far, for REQUIRED
    extensions: HashMap<String, Cell>, // installed extensions: version by upper-case name
    host_state: HashMap<std::any::TypeId, Box<dyn std::any::Any>>, // see Forth::state
    out: Vec<u8>,
    bye: bool,
    limits: Limits,                      // see sandbox.rs
    budget: Budget,                      // what is left of them in this evaluation
    denied: HashSet<Permission>,         // what the program may not do
    clock: Option<Box<dyn Fn() -> u64>>, // seconds since 1970 for TIME&DATE (set_clock)
    watching: bool,                      // whether invoke calls watch() for each word (trace.rs)
    debug: Debugging,                    // the debugger (debug.rs)
    first_line: usize,                   // the terminal line before the text of interpret began
    tracer: Tracer,
    space: usize,        // the dictionary and the REPLACES texts outside Forth memory
    system_space: usize, // the part of the space that is the system's own
}

pub(crate) fn flag(b: bool) -> Cell {
    if b { -1 } else { 0 }
}

impl Forth {
    /// A system with the default memory size (1 MiB).
    ///
    /// Ignoring the result is a mistake, so the compiler warns about it:
    ///
    /// ```compile_fail
    /// #![deny(unused_must_use)]
    /// forthling::Forth::new();
    /// ```
    #[must_use]
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
    #[must_use]
    pub fn with_memory(size: usize) -> Self {
        let valid = (MIN_MEMORY..=MAX_MEMORY).contains(&size) && size % CELL == 0;
        assert!(valid, "invalid memory size: {size}");
        let mut f = Forth {
            stack: Vec::new(),
            fstack: Vec::new(),
            precision: DEFAULT_PRECISION,
            rstack: Vec::new(),
            locals: Vec::new(),
            loops: Vec::new(),
            frames: Vec::new(),
            mem: vec![0; size],
            here: DICT_START,
            heap: Heap::new(size),
            dict: Vec::new(),
            wordlists: vec![Some("FORTH".into())],
            order: vec![FORTH_WID],
            current_wid: FORTH_WID,
            current: None,
            fence: 0,
            encoding_name: None,
            code: Vec::new(),
            ctrl: Vec::new(),
            quotes: Vec::new(),
            local_names: Vec::new(),
            locals_bound: 0,
            sources: Sources::new(Source {
                origin: Origin::Terminal,
                buf: SOURCE_AREA,
                len: 0,
                line: 0,
                saved_in: 0,
            }),
            sq_next: 0,
            hld: HOLD_END,
            input: Box::<Stdin>::default(),
            interactive: false,
            catches: 0,
            substitutions: HashMap::new(),
            files: Vec::new(),
            blocks: Blocks::new("blocks.fb".into()),
            vfs: Box::new(NativeFs),
            included: HashSet::new(),
            extensions: HashMap::new(),
            host_state: HashMap::new(),
            out: Vec::new(),
            bye: false,
            limits: Limits::default(),
            budget: Budget::default(),
            denied: HashSet::new(),
            clock: None,
            watching: false,
            debug: Debugging::default(),
            first_line: 0,
            tracer: Tracer::default(),
            space: 0,
            system_space: 0,
        };
        f.set(BASE_ADDR, 10);
        words::install(&mut f);
        f.interpret(include_str!("prelude.fs")).expect("prelude must compile");
        f.fence = f.dict.len();
        f.system_space = f.space_used();
        f
    }
}

impl Default for Forth {
    fn default() -> Self {
        Forth::new()
    }
}

/// The interface for programs that embed the interpreter, such as the REPL
/// in cli.rs. `interpret`, `include`, `read_line` and `flush` are public too,
/// and host.rs has the API for adding words.
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
    #[must_use]
    pub fn output(&self) -> &[u8] {
        &self.out
    }

    /// Whether BYE was executed.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.bye
    }

    /// Whether a colon definition is still open, so the REPL says "compiled".
    #[must_use]
    pub fn in_definition(&self) -> bool {
        self.current.is_some()
    }
}
