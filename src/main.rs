//! A small Forth interpreter, working towards Forth-2012 (see STANDAARD.md).
//!
//! Words are compiled to a list of instructions and run by an explicit
//! frame stack, so deep recursion in Forth never overflows the Rust stack.
//! Memory is byte-addressed with 64-bit little-endian cells. Everything a
//! Forth program may inspect (BASE, STATE, >IN, the input buffers) lives in
//! that memory, and errors are standard THROW codes.

use std::collections::HashSet;
use std::env;
use std::fmt;
use std::fs;
use std::io::{self, BufRead, Write};
use std::rc::Rc;

type Cell = i64;
type Res<T = ()> = Result<T, Throw>;
type Prim = fn(&mut Forth) -> Res;

const CELL: usize = 8;
const MAX_FRAMES: usize = 10_000;

// Memory map: system variables at the bottom, then the data space that
// HERE grows through, then transient buffers and the input buffers on top.
const MEM_SIZE: usize = 1 << 20;
const BASE_ADDR: usize = 0;
const STATE_ADDR: usize = 8;
const IN_ADDR: usize = 16;
const DICT_START: usize = 64;
const LINE_MAX: usize = 1024; // longest input line
const MAX_SOURCES: usize = 16; // the terminal plus nested INCLUDEs
const SOURCE_AREA: usize = MEM_SIZE - MAX_SOURCES * LINE_MAX;
const SQ_BUFS: usize = SOURCE_AREA - 2 * LINE_MAX; // two alternating S" buffers
const WORD_BUF: usize = SQ_BUFS - 264; // counted string, trailing space, padding
#[allow(dead_code)] // reserved for PAD (CORE EXT)
const PAD_ADDR: usize = WORD_BUF - 256;
#[allow(dead_code)] // reserved for <# #>, which builds downwards from here
const HOLD_END: usize = PAD_ADDR;
const DATA_END: usize = PAD_ADDR - 256;

/// System THROW code: INCLUDEs nested deeper than MAX_SOURCES.
const NESTING_TOO_DEEP: Cell = -256;

/// Words that are easier to express in Forth itself.
const PRELUDE: &str = r#"
-1 CONSTANT TRUE   0 CONSTANT FALSE   32 CONSTANT BL
: NIP     SWAP DROP ;
: TUCK    SWAP OVER ;
: -ROT    ROT ROT ;
: ?DUP    DUP IF DUP THEN ;
: 2DUP    OVER OVER ;
: 2DROP   DROP DROP ;
: 2SWAP   ROT >R ROT R> ;
: 2OVER   3 PICK 3 PICK ;
: 1+  1 + ;   : 1-  1 - ;   : 2*  2 * ;
: 0=  0 = ;   : 0<  0 < ;   : 0>  0 > ;   : <>  = 0= ;
: NEGATE  0 SWAP - ;
: ABS     DUP 0< IF NEGATE THEN ;
: MIN     2DUP > IF SWAP THEN DROP ;
: MAX     2DUP < IF SWAP THEN DROP ;
: +!      DUP @ ROT + SWAP ! ;
: CELLS  8 * ;   : CELL+  8 + ;   : CHARS ;   : CHAR+  1+ ;
: VARIABLE  CREATE 0 , ;
: CR      10 EMIT ;
: SPACE   BL EMIT ;
: SPACES  BEGIN DUP 0> WHILE SPACE 1- REPEAT DROP ;
: DECIMAL 10 BASE ! ;
: HEX     16 BASE ! ;
"#;

/// A Forth exception: a THROW code from table 9.1 of the standard, or a
/// system code below -255.
#[derive(Debug)]
pub struct Throw {
    pub code: Cell,
    detail: Option<String>,
    location: Option<String>,
}

impl Throw {
    fn new(code: Cell) -> Self {
        Throw {
            code,
            detail: None,
            location: None,
        }
    }

    fn with(code: Cell, detail: impl Into<String>) -> Self {
        Throw {
            detail: Some(detail.into()),
            ..Throw::new(code)
        }
    }

    /// Records where the error happened; the innermost location wins.
    fn at(mut self, path: &str, line: usize) -> Self {
        if self.location.is_none() {
            self.location = Some(format!("{path}:{line}"));
        }
        self
    }

    fn message(&self) -> &'static str {
        match self.code {
            -4 => "stack underflow",
            -5 => "return stack overflow",
            -6 => "return stack underflow",
            -8 => "dictionary overflow",
            -9 => "invalid memory address",
            -10 => "division by zero",
            -11 => "result out of range",
            -13 => "undefined word",
            -14 => "interpreting a compile-only word",
            -16 => "attempt to use zero-length string as a name",
            -18 => "parsed string overflow",
            -21 => "unsupported operation",
            -22 => "control structure mismatch",
            -24 => "invalid numeric argument",
            -26 => "loop parameters unavailable",
            -29 => "compiler nesting",
            -38 => "non-existent file",
            NESTING_TOO_DEEP => "input nesting too deep",
            _ => "exception",
        }
    }
}

impl fmt::Display for Throw {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(loc) = &self.location {
            write!(f, "{loc}: ")?;
        }
        write!(f, "error {}: {}", self.code, self.message())?;
        if let Some(detail) = &self.detail {
            write!(f, ": {detail}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
enum Instr {
    Call(usize),
    Lit(Cell),
    Branch(usize),
    ZBranch(usize),
    Do(usize),  // target: just past the matching LOOP
    QDo(usize), // like Do, but skips the loop when limit == index
    Loop(usize),     // target: start of the loop body
    PlusLoop(usize), // target: start of the loop body
    Leave,
    Print(Rc<[u8]>),
    Compile(usize), // POSTPONEd non-immediate word: compiles a call to it
    SetDoes(usize), // gives the latest CREATEd word the code at this offset
    Exit,
}

#[derive(Clone)]
enum Kind {
    Prim(Prim),
    Colon(Rc<Vec<Instr>>),
    Constant(Cell),
    Created {
        addr: Cell,
        does: Option<(Rc<Vec<Instr>>, usize)>,
    },
}

struct Word {
    name: String,
    kind: Kind,
    immediate: bool,
    hidden: bool,
}

struct Frame {
    code: Rc<Vec<Instr>>,
    pc: usize,
    loops: usize, // loop stack depth on entry, restored on exit
}

struct LoopFrame {
    index: Cell,
    limit: Cell,
    exit: usize,
}

/// Compile-time control-flow markers: an unresolved forward jump, a
/// backward jump target, or an open DO.
enum Ctrl {
    Orig(usize),
    Dest(usize),
    Do(usize),
}

/// An input source: its line buffer in memory and, while a nested source
/// runs, the >IN to restore afterwards.
struct Source {
    buf: usize,
    len: usize,
    saved_in: Cell,
}

pub struct Forth {
    stack: Vec<Cell>,
    rstack: Vec<Cell>,
    loops: Vec<LoopFrame>,
    frames: Vec<Frame>,
    mem: Vec<u8>,
    here: usize,
    dict: Vec<Word>,
    current: Option<usize>,
    code: Vec<Instr>,
    ctrl: Vec<Ctrl>,
    sources: Vec<Source>,
    sq_next: usize,
    out: Vec<u8>,
    bye: bool,
}

fn flag(b: bool) -> Cell {
    if b {
        -1
    } else {
        0
    }
}

fn fmt_unsigned(mut v: u64, base: u64) -> String {
    let mut digits = Vec::new();
    loop {
        let d = std::char::from_digit((v % base) as u32, base as u32).unwrap();
        digits.push(d.to_ascii_uppercase());
        v /= base;
        if v == 0 {
            break;
        }
    }
    digits.iter().rev().collect()
}

impl Forth {
    pub fn new() -> Self {
        let mut f = Forth {
            stack: Vec::new(),
            rstack: Vec::new(),
            loops: Vec::new(),
            frames: Vec::new(),
            mem: vec![0; MEM_SIZE],
            here: DICT_START,
            dict: Vec::new(),
            current: None,
            code: Vec::new(),
            ctrl: Vec::new(),
            sources: vec![Source {
                buf: SOURCE_AREA,
                len: 0,
                saved_in: 0,
            }],
            sq_next: 0,
            out: Vec::new(),
            bye: false,
        };
        f.set(BASE_ADDR, 10);
        f.install_primitives();
        f.interpret(PRELUDE).expect("prelude must compile");
        f
    }

    // ---- stacks -----------------------------------------------------------

    fn push(&mut self, v: Cell) {
        self.stack.push(v);
    }

    fn pop(&mut self) -> Res<Cell> {
        self.stack.pop().ok_or(Throw::new(-4))
    }

    fn pop2(&mut self) -> Res<(Cell, Cell)> {
        let b = self.pop()?;
        let a = self.pop()?;
        Ok((a, b))
    }

    fn binop(&mut self, op: impl Fn(Cell, Cell) -> Cell) -> Res {
        let (a, b) = self.pop2()?;
        self.push(op(a, b));
        Ok(())
    }

    fn divop(&mut self, op: impl Fn(Cell, Cell) -> Cell) -> Res {
        let (a, b) = self.pop2()?;
        if b == 0 {
            return Err(Throw::new(-10));
        }
        self.push(op(a, b));
        Ok(())
    }

    fn loop_index(&self, depth: usize) -> Res<Cell> {
        let n = self.loops.len();
        if depth < n {
            Ok(self.loops[n - 1 - depth].index)
        } else {
            Err(Throw::new(-26))
        }
    }

    // ---- memory -----------------------------------------------------------

    /// Reads a cell at an address the system itself controls.
    fn get(&self, a: usize) -> Cell {
        Cell::from_le_bytes(self.mem[a..a + CELL].try_into().unwrap())
    }

    fn set(&mut self, a: usize, v: Cell) {
        self.mem[a..a + CELL].copy_from_slice(&v.to_le_bytes());
    }

    /// Validates a program-supplied address range of `n` bytes.
    fn check(&self, a: Cell, n: Cell) -> Res<usize> {
        let end = a.checked_add(n).unwrap_or(-1);
        if a >= 0 && n >= 0 && end >= 0 && end as usize <= MEM_SIZE {
            Ok(a as usize)
        } else {
            Err(Throw::with(-9, a.to_string()))
        }
    }

    fn reserve(&mut self, n: usize) -> Res<usize> {
        let a = self.here;
        if a + n > DATA_END {
            return Err(Throw::new(-8));
        }
        self.here += n;
        Ok(a)
    }

    fn comma(&mut self, v: Cell) -> Res {
        let a = self.reserve(CELL)?;
        self.set(a, v);
        Ok(())
    }

    fn align_here(&mut self) {
        self.here = (self.here + CELL - 1) & !(CELL - 1);
    }

    /// Copies `n` bytes into data space and returns their new address.
    fn compile_string(&mut self, src: usize, n: usize) -> Res<usize> {
        let a = self.reserve(n)?;
        self.mem.copy_within(src..src + n, a);
        self.align_here();
        Ok(a)
    }

    fn text(&self, a: usize, n: usize) -> String {
        String::from_utf8_lossy(&self.mem[a..a + n]).into_owned()
    }

    fn emit_str(&mut self, s: &str) {
        self.out.extend_from_slice(s.as_bytes());
    }

    fn compiling(&self) -> bool {
        self.get(STATE_ADDR) != 0
    }

    fn set_state(&mut self, compiling: bool) {
        self.set(STATE_ADDR, flag(compiling));
    }

    // ---- dictionary -------------------------------------------------------

    fn define(&mut self, name: &str, kind: Kind) -> usize {
        self.dict.push(Word {
            name: name.to_ascii_uppercase(),
            kind,
            immediate: false,
            hidden: false,
        });
        self.dict.len() - 1
    }

    fn prim(&mut self, name: &str, f: Prim) {
        self.define(name, Kind::Prim(f));
    }

    fn imm(&mut self, name: &str, f: Prim) {
        let xt = self.define(name, Kind::Prim(f));
        self.dict[xt].immediate = true;
    }

    fn variable(&mut self, name: &str, addr: usize) {
        self.define(
            name,
            Kind::Created {
                addr: addr as Cell,
                does: None,
            },
        );
    }

    fn find(&self, name: &str) -> Option<usize> {
        let upper = name.to_ascii_uppercase();
        self.dict.iter().rposition(|w| !w.hidden && w.name == upper)
    }

    fn find_or_err(&self, name: &str) -> Res<usize> {
        self.find(name).ok_or_else(|| Throw::with(-13, name))
    }

    // ---- parsing ----------------------------------------------------------

    /// The unparsed rest of the current input buffer, as absolute addresses.
    fn parse_area(&self) -> (usize, usize) {
        let s = self.sources.last().unwrap();
        let i = self.get(IN_ADDR).clamp(0, s.len as Cell) as usize;
        (s.buf + i, s.buf + s.len)
    }

    fn set_in(&mut self, at: usize) {
        let buf = self.sources.last().unwrap().buf;
        self.set(IN_ADDR, (at - buf) as Cell);
    }

    /// Parses up to a delimiter, optionally skipping leading delimiters, and
    /// moves >IN past the delimiter. Returns the text's address and length.
    fn scan(&mut self, skip: bool, is_delim: impl Fn(u8) -> bool) -> (usize, usize) {
        let (mut i, end) = self.parse_area();
        if skip {
            while i < end && is_delim(self.mem[i]) {
                i += 1;
            }
        }
        let start = i;
        while i < end && !is_delim(self.mem[i]) {
            i += 1;
        }
        self.set_in(if i < end { i + 1 } else { i });
        (start, i - start)
    }

    fn parse_name(&mut self) -> (usize, usize) {
        self.scan(true, |c| c <= b' ')
    }

    fn parse(&mut self, delim: u8) -> (usize, usize) {
        self.scan(false, |c| c == delim)
    }

    fn word(&mut self) -> Option<String> {
        let (a, n) = self.parse_name();
        (n > 0).then(|| self.text(a, n))
    }

    fn name(&mut self) -> Res<String> {
        self.word().ok_or(Throw::new(-16))
    }

    fn number(&self, w: &str) -> Option<Cell> {
        let b = w.as_bytes();
        if b.len() == 3 && b[0] == b'\'' && b[2] == b'\'' {
            return Some(b[1] as Cell);
        }
        let (base, s) = match b.first()? {
            b'#' => (10, &w[1..]),
            b'$' => (16, &w[1..]),
            b'%' => (2, &w[1..]),
            _ => (self.get(BASE_ADDR), w),
        };
        let (neg, digits) = match s.strip_prefix('-') {
            Some(d) => (true, d),
            None => (false, s),
        };
        if digits.is_empty() || !(2..=36).contains(&base) {
            return None;
        }
        if !digits.chars().all(|c| c.is_digit(base as u32)) {
            return None;
        }
        let v = u64::from_str_radix(digits, base as u32).ok()? as Cell;
        Some(if neg { v.wrapping_neg() } else { v })
    }

    fn fmt_num(&self, n: Cell) -> String {
        let base = self.get(BASE_ADDR);
        let base = if (2..=36).contains(&base) { base as u64 } else { 10 };
        let s = fmt_unsigned(n.unsigned_abs(), base);
        if n < 0 {
            format!("-{s}")
        } else {
            s
        }
    }

    // ---- outer interpreter ------------------------------------------------

    /// Interprets text line by line, as if typed at the terminal.
    pub fn interpret(&mut self, src: &str) -> Res {
        let result = self.interpret_lines(src);
        if result.is_err() {
            self.reset();
        }
        result
    }

    fn interpret_lines(&mut self, src: &str) -> Res {
        for line in src.lines() {
            self.load_line(line)?;
            self.interpret_buffer()?;
            if self.bye {
                break;
            }
        }
        Ok(())
    }

    /// Puts a line in the current source's buffer and resets >IN.
    fn load_line(&mut self, line: &str) -> Res {
        // A byte-order mark (from Windows editors and pipes) is not input.
        let bytes = line.trim_start_matches('\u{feff}').as_bytes();
        if bytes.len() > LINE_MAX {
            return Err(Throw::with(-18, "input line too long"));
        }
        let s = self.sources.last_mut().unwrap();
        s.len = bytes.len();
        let buf = s.buf;
        self.mem[buf..buf + bytes.len()].copy_from_slice(bytes);
        self.set(IN_ADDR, 0);
        Ok(())
    }

    fn interpret_buffer(&mut self) -> Res {
        while !self.bye {
            let Some(w) = self.word() else { break };
            self.handle(&w)?;
        }
        Ok(())
    }

    fn handle(&mut self, w: &str) -> Res {
        if let Some(xt) = self.find(w) {
            if self.compiling() && !self.dict[xt].immediate {
                self.code.push(Instr::Call(xt));
                Ok(())
            } else {
                self.execute(xt)
            }
        } else if let Some(n) = self.number(w) {
            if self.compiling() {
                self.code.push(Instr::Lit(n));
            } else {
                self.push(n);
            }
            Ok(())
        } else {
            Err(Throw::with(-13, w))
        }
    }

    /// Interprets a file as a nested input source with its own line buffer.
    fn include(&mut self, path: &str) -> Res {
        let src = fs::read_to_string(path).map_err(|e| Throw::with(-38, format!("{path}: {e}")))?;
        if self.sources.len() == MAX_SOURCES {
            return Err(Throw::new(NESTING_TOO_DEEP));
        }
        let saved_in = self.get(IN_ADDR);
        self.sources.last_mut().unwrap().saved_in = saved_in;
        let buf = SOURCE_AREA + self.sources.len() * LINE_MAX;
        self.sources.push(Source {
            buf,
            len: 0,
            saved_in: 0,
        });
        for (n, line) in src.lines().enumerate() {
            self.load_line(line)
                .and_then(|_| self.interpret_buffer())
                .map_err(|e| e.at(path, n + 1))?;
            if self.bye {
                break;
            }
        }
        self.sources.pop();
        let saved_in = self.sources.last().unwrap().saved_in;
        self.set(IN_ADDR, saved_in);
        Ok(())
    }

    /// Recovers from an error: clears the stacks, returns to the terminal
    /// and drops a half-finished definition.
    fn reset(&mut self) {
        self.stack.clear();
        self.rstack.clear();
        self.loops.clear();
        self.frames.clear();
        self.ctrl.clear();
        self.sources.truncate(1);
        if let Some(xt) = self.current.take() {
            self.dict.truncate(xt);
        }
        self.set_state(false);
        self.code.clear();
    }

    fn flush(&mut self) {
        io::stdout().write_all(&self.out).ok();
        self.out.clear();
        io::stdout().flush().ok();
    }

    // ---- inner interpreter ------------------------------------------------

    fn execute(&mut self, xt: usize) -> Res {
        let base = self.frames.len();
        self.invoke(xt)?;
        self.run(base)
    }

    /// Runs a primitive directly, or pushes a frame for compiled code.
    fn invoke(&mut self, xt: usize) -> Res {
        let word = self
            .dict
            .get(xt)
            .ok_or_else(|| Throw::with(-24, format!("invalid execution token {xt}")))?;
        match word.kind.clone() {
            Kind::Prim(f) => f(self),
            Kind::Colon(code) => self.enter(code, 0),
            Kind::Constant(v) => {
                self.push(v);
                Ok(())
            }
            Kind::Created { addr, does } => {
                self.push(addr);
                match does {
                    Some((code, pc)) => self.enter(code, pc),
                    None => Ok(()),
                }
            }
        }
    }

    fn enter(&mut self, code: Rc<Vec<Instr>>, pc: usize) -> Res {
        if self.frames.len() >= MAX_FRAMES {
            return Err(Throw::new(-5));
        }
        self.frames.push(Frame {
            code,
            pc,
            loops: self.loops.len(),
        });
        Ok(())
    }

    fn leave_frame(&mut self) {
        if let Some(f) = self.frames.pop() {
            self.loops.truncate(f.loops);
        }
    }

    fn jump(&mut self, pc: usize) {
        if let Some(f) = self.frames.last_mut() {
            f.pc = pc;
        }
    }

    fn run(&mut self, base: usize) -> Res {
        while self.frames.len() > base {
            let frame = self.frames.last_mut().unwrap();
            let Some(instr) = frame.code.get(frame.pc).cloned() else {
                self.leave_frame();
                continue;
            };
            frame.pc += 1;
            match instr {
                Instr::Call(xt) => self.invoke(xt)?,
                Instr::Lit(n) => self.push(n),
                Instr::Branch(t) => self.jump(t),
                Instr::ZBranch(t) => {
                    if self.pop()? == 0 {
                        self.jump(t);
                    }
                }
                Instr::Do(exit) => {
                    let (limit, index) = self.pop2()?;
                    self.loops.push(LoopFrame { index, limit, exit });
                }
                Instr::QDo(exit) => {
                    let (limit, index) = self.pop2()?;
                    if index == limit {
                        self.jump(exit);
                    } else {
                        self.loops.push(LoopFrame { index, limit, exit });
                    }
                }
                Instr::Loop(start) => self.loop_step(start, 1)?,
                Instr::PlusLoop(start) => {
                    let step = self.pop()?;
                    self.loop_step(start, step)?;
                }
                Instr::Leave => {
                    let lf = self.loops.pop().ok_or(Throw::new(-26))?;
                    self.jump(lf.exit);
                }
                Instr::Print(s) => self.out.extend_from_slice(&s),
                Instr::Compile(xt) => {
                    self.need_def()?;
                    self.code.push(Instr::Call(xt));
                }
                Instr::SetDoes(pc) => {
                    let code = self.frames.last().unwrap().code.clone();
                    match self.dict.last_mut() {
                        Some(Word {
                            kind: Kind::Created { does, .. },
                            ..
                        }) => *does = Some((code, pc)),
                        _ => return Err(Throw::with(-21, "DOES> without CREATE")),
                    }
                }
                Instr::Exit => self.leave_frame(),
            }
        }
        Ok(())
    }

    /// Advances the innermost loop; it ends when the index crosses the
    /// boundary between limit-1 and limit, in either direction.
    fn loop_step(&mut self, start: usize, step: Cell) -> Res {
        let lf = self.loops.last_mut().ok_or(Throw::new(-26))?;
        let before = lf.index.wrapping_sub(lf.limit);
        let after = before.wrapping_add(step);
        lf.index = lf.index.wrapping_add(step);
        if (before ^ after) & (before ^ step) < 0 {
            self.loops.pop();
        } else {
            self.jump(start);
        }
        Ok(())
    }

    // ---- compiler helpers -------------------------------------------------

    fn need_def(&self) -> Res {
        if self.current.is_some() {
            Ok(())
        } else {
            Err(Throw::new(-14))
        }
    }

    fn emit_fwd(&mut self, instr: Instr) -> usize {
        self.code.push(instr);
        self.code.len() - 1
    }

    fn patch(&mut self, at: usize, target: usize) {
        match &mut self.code[at] {
            Instr::Branch(t) | Instr::ZBranch(t) | Instr::Do(t) | Instr::QDo(t) => *t = target,
            _ => unreachable!("patching a non-jump instruction"),
        }
    }

    fn pop_orig(&mut self) -> Res<usize> {
        match self.ctrl.pop() {
            Some(Ctrl::Orig(p)) => Ok(p),
            _ => Err(Throw::new(-22)),
        }
    }

    fn pop_dest(&mut self) -> Res<usize> {
        match self.ctrl.pop() {
            Some(Ctrl::Dest(p)) => Ok(p),
            _ => Err(Throw::new(-22)),
        }
    }

    fn pop_do(&mut self) -> Res<usize> {
        match self.ctrl.pop() {
            Some(Ctrl::Do(p)) => Ok(p),
            _ => Err(Throw::new(-22)),
        }
    }

    fn close_loop(&mut self, make: fn(usize) -> Instr) -> Res {
        self.need_def()?;
        let p = self.pop_do()?;
        self.code.push(make(p + 1));
        let end = self.code.len();
        self.patch(p, end);
        Ok(())
    }

    // ---- introspection ----------------------------------------------------

    fn show(&self, instr: &Instr) -> String {
        match instr {
            Instr::Call(xt) => self.dict[*xt].name.clone(),
            Instr::Lit(n) => n.to_string(),
            Instr::Branch(t) => format!("BRANCH {t}"),
            Instr::ZBranch(t) => format!("0BRANCH {t}"),
            Instr::Do(t) => format!("(DO) {t}"),
            Instr::QDo(t) => format!("(?DO) {t}"),
            Instr::Loop(t) => format!("(LOOP) {t}"),
            Instr::PlusLoop(t) => format!("(+LOOP) {t}"),
            Instr::Leave => "LEAVE".into(),
            Instr::Print(s) => format!(".\" {}\"", String::from_utf8_lossy(s)),
            Instr::Compile(xt) => format!("POSTPONE {}", self.dict[*xt].name),
            Instr::SetDoes(_) => "DOES>".into(),
            Instr::Exit => "EXIT".into(),
        }
    }

    fn see(&mut self) -> Res {
        let name = self.name()?;
        let w = &self.dict[self.find_or_err(&name)?];
        let mut text = match &w.kind {
            Kind::Prim(_) => format!("{} is a primitive", w.name),
            Kind::Constant(v) => format!("{v} CONSTANT {}", w.name),
            Kind::Created { addr, does } => format!(
                "CREATE {} ( address {addr} ){}",
                w.name,
                if does.is_some() { " with DOES>" } else { "" }
            ),
            Kind::Colon(code) => {
                let mut s = format!(": {}\n", w.name);
                for (i, instr) in code.iter().enumerate() {
                    s += &format!("{i:>4}  {}\n", self.show(instr));
                }
                s + ";"
            }
        };
        if w.immediate {
            text += " IMMEDIATE";
        }
        text.push('\n');
        self.emit_str(&text);
        Ok(())
    }

    fn words(&mut self) {
        let mut seen = HashSet::new();
        let mut text = String::new();
        let mut line = String::new();
        for w in self.dict.iter().rev() {
            if w.hidden || !seen.insert(w.name.as_str()) {
                continue;
            }
            if line.len() + w.name.len() > 72 {
                text += line.trim_end();
                text.push('\n');
                line.clear();
            }
            line += &w.name;
            line.push(' ');
        }
        text += line.trim_end();
        text.push('\n');
        self.emit_str(&text);
    }

    // ---- primitives -------------------------------------------------------

    fn install_primitives(&mut self) {
        self.variable("BASE", BASE_ADDR);
        self.variable("STATE", STATE_ADDR);
        self.variable(">IN", IN_ADDR);

        // stack
        self.prim("DUP", |f| {
            let a = f.pop()?;
            f.push(a);
            f.push(a);
            Ok(())
        });
        self.prim("DROP", |f| f.pop().map(|_| ()));
        self.prim("SWAP", |f| {
            let (a, b) = f.pop2()?;
            f.push(b);
            f.push(a);
            Ok(())
        });
        self.prim("OVER", |f| {
            let (a, b) = f.pop2()?;
            f.push(a);
            f.push(b);
            f.push(a);
            Ok(())
        });
        self.prim("ROT", |f| {
            let c = f.pop()?;
            let (a, b) = f.pop2()?;
            f.push(b);
            f.push(c);
            f.push(a);
            Ok(())
        });
        self.prim("PICK", |f| {
            let n = f.pop()?;
            let len = f.stack.len() as Cell;
            if n < 0 || n >= len {
                return Err(Throw::new(-4));
            }
            f.push(f.stack[(len - 1 - n) as usize]);
            Ok(())
        });
        self.prim("DEPTH", |f| {
            f.push(f.stack.len() as Cell);
            Ok(())
        });
        self.prim(">R", |f| {
            let a = f.pop()?;
            f.rstack.push(a);
            Ok(())
        });
        self.prim("R>", |f| {
            let a = f.rstack.pop().ok_or(Throw::new(-6))?;
            f.push(a);
            Ok(())
        });
        self.prim("R@", |f| {
            let a = *f.rstack.last().ok_or(Throw::new(-6))?;
            f.push(a);
            Ok(())
        });

        // arithmetic and logic
        self.prim("+", |f| f.binop(|a, b| a.wrapping_add(b)));
        self.prim("-", |f| f.binop(|a, b| a.wrapping_sub(b)));
        self.prim("*", |f| f.binop(|a, b| a.wrapping_mul(b)));
        self.prim("/", |f| f.divop(|a, b| a.wrapping_div(b)));
        self.prim("MOD", |f| f.divop(|a, b| a.wrapping_rem(b)));
        self.prim("/MOD", |f| {
            let (a, b) = f.pop2()?;
            if b == 0 {
                return Err(Throw::new(-10));
            }
            f.push(a.wrapping_rem(b));
            f.push(a.wrapping_div(b));
            Ok(())
        });
        self.prim("*/", |f| {
            let c = f.pop()?;
            let (a, b) = f.pop2()?;
            if c == 0 {
                return Err(Throw::new(-10));
            }
            let r = a as i128 * b as i128 / c as i128;
            f.push(Cell::try_from(r).map_err(|_| Throw::new(-11))?);
            Ok(())
        });
        self.prim("2/", |f| {
            let a = f.pop()?;
            f.push(a >> 1);
            Ok(())
        });
        self.prim("AND", |f| f.binop(|a, b| a & b));
        self.prim("OR", |f| f.binop(|a, b| a | b));
        self.prim("XOR", |f| f.binop(|a, b| a ^ b));
        self.prim("INVERT", |f| {
            let a = f.pop()?;
            f.push(!a);
            Ok(())
        });
        self.prim("LSHIFT", |f| f.binop(|a, b| ((a as u64) << (b & 63)) as Cell));
        self.prim("RSHIFT", |f| f.binop(|a, b| ((a as u64) >> (b & 63)) as Cell));
        self.prim("=", |f| f.binop(|a, b| flag(a == b)));
        self.prim("<", |f| f.binop(|a, b| flag(a < b)));
        self.prim(">", |f| f.binop(|a, b| flag(a > b)));
        self.prim("U<", |f| f.binop(|a, b| flag((a as u64) < (b as u64))));

        // memory
        self.prim("@", |f| {
            let a = f.pop()?;
            let i = f.check(a, CELL as Cell)?;
            f.push(f.get(i));
            Ok(())
        });
        self.prim("!", |f| {
            let (v, a) = f.pop2()?;
            let i = f.check(a, CELL as Cell)?;
            f.set(i, v);
            Ok(())
        });
        self.prim("C@", |f| {
            let a = f.pop()?;
            let i = f.check(a, 1)?;
            f.push(f.mem[i] as Cell);
            Ok(())
        });
        self.prim("C!", |f| {
            let (v, a) = f.pop2()?;
            let i = f.check(a, 1)?;
            f.mem[i] = v as u8;
            Ok(())
        });
        self.prim(",", |f| {
            let v = f.pop()?;
            f.comma(v)
        });
        self.prim("C,", |f| {
            let v = f.pop()?;
            let a = f.reserve(1)?;
            f.mem[a] = v as u8;
            Ok(())
        });
        self.prim("HERE", |f| {
            f.push(f.here as Cell);
            Ok(())
        });
        self.prim("ALLOT", |f| {
            let n = f.pop()?;
            let new = f.here as Cell + n;
            if new < DICT_START as Cell || new > DATA_END as Cell {
                return Err(Throw::new(-8));
            }
            f.here = new as usize;
            Ok(())
        });
        self.prim("COUNT", |f| {
            let a = f.pop()?;
            let i = f.check(a, 1)?;
            f.push(a + 1);
            f.push(f.mem[i] as Cell);
            Ok(())
        });

        // output
        self.prim(".", |f| {
            let n = f.pop()?;
            let s = f.fmt_num(n);
            f.emit_str(&s);
            f.out.push(b' ');
            Ok(())
        });
        self.prim("U.", |f| {
            let n = f.pop()?;
            let base = f.get(BASE_ADDR).clamp(2, 36) as u64;
            f.emit_str(&fmt_unsigned(n as u64, base));
            f.out.push(b' ');
            Ok(())
        });
        self.prim(".S", |f| {
            let mut s = format!("<{}> ", f.stack.len());
            for &n in &f.stack {
                s += &f.fmt_num(n);
                s.push(' ');
            }
            f.emit_str(&s);
            Ok(())
        });
        self.prim("EMIT", |f| {
            let c = f.pop()?;
            f.out.push(c as u8);
            Ok(())
        });
        self.prim("TYPE", |f| {
            let (a, n) = f.pop2()?;
            if n > 0 {
                let i = f.check(a, n)?;
                f.out.extend_from_slice(&f.mem[i..i + n as usize]);
            }
            Ok(())
        });

        // input and parsing
        self.prim("SOURCE", |f| {
            let s = f.sources.last().unwrap();
            let (buf, len) = (s.buf, s.len);
            f.push(buf as Cell);
            f.push(len as Cell);
            Ok(())
        });
        self.prim("WORD", |f| {
            let delim = f.pop()? as u8;
            let (a, n) = if delim == b' ' {
                f.parse_name()
            } else {
                f.scan(true, |c| c == delim)
            };
            if n > 255 {
                return Err(Throw::new(-18));
            }
            f.mem.copy_within(a..a + n, WORD_BUF + 1);
            f.mem[WORD_BUF] = n as u8;
            f.mem[WORD_BUF + 1 + n] = b' ';
            f.push(WORD_BUF as Cell);
            Ok(())
        });
        self.prim("FIND", |f| {
            let a = f.pop()?;
            let i = f.check(a, 1)?;
            let n = f.mem[i] as usize;
            f.check(a, 1 + n as Cell)?;
            let name = f.text(i + 1, n);
            match f.find(&name) {
                Some(xt) => {
                    f.push(xt as Cell);
                    f.push(if f.dict[xt].immediate { 1 } else { -1 });
                }
                None => {
                    f.push(a);
                    f.push(0);
                }
            }
            Ok(())
        });

        // definitions
        self.prim(":", |f| {
            if f.current.is_some() {
                return Err(Throw::new(-29));
            }
            let name = f.name()?;
            let xt = f.define(&name, Kind::Colon(Rc::new(Vec::new())));
            f.dict[xt].hidden = true;
            f.current = Some(xt);
            f.set_state(true);
            f.code.clear();
            f.ctrl.clear();
            Ok(())
        });
        self.imm(";", |f| {
            let xt = f.current.ok_or(Throw::new(-22))?;
            if !f.ctrl.is_empty() {
                return Err(Throw::new(-22));
            }
            f.dict[xt].kind = Kind::Colon(Rc::new(std::mem::take(&mut f.code)));
            f.dict[xt].hidden = false;
            f.current = None;
            f.set_state(false);
            Ok(())
        });
        self.prim("IMMEDIATE", |f| {
            f.dict.last_mut().unwrap().immediate = true;
            Ok(())
        });
        self.prim("CREATE", |f| {
            let name = f.name()?;
            f.align_here();
            let addr = f.here as Cell;
            f.define(&name, Kind::Created { addr, does: None });
            Ok(())
        });
        self.imm("DOES>", |f| {
            f.need_def()?;
            let at = f.code.len();
            f.code.push(Instr::SetDoes(at + 2));
            f.code.push(Instr::Exit);
            Ok(())
        });
        self.prim("CONSTANT", |f| {
            let name = f.name()?;
            let v = f.pop()?;
            f.define(&name, Kind::Constant(v));
            Ok(())
        });
        self.prim("'", |f| {
            let name = f.name()?;
            let xt = f.find_or_err(&name)?;
            f.push(xt as Cell);
            Ok(())
        });
        self.imm("[']", |f| {
            f.need_def()?;
            let name = f.name()?;
            let xt = f.find_or_err(&name)?;
            f.code.push(Instr::Lit(xt as Cell));
            Ok(())
        });
        self.prim("EXECUTE", |f| {
            let xt = f.pop()?;
            if xt < 0 {
                return Err(Throw::with(-24, format!("invalid execution token {xt}")));
            }
            f.invoke(xt as usize)
        });
        self.prim("CHAR", |f| {
            let (a, n) = f.parse_name();
            if n == 0 {
                return Err(Throw::new(-16));
            }
            f.push(f.mem[a] as Cell);
            Ok(())
        });
        self.imm("[CHAR]", |f| {
            f.need_def()?;
            let (a, n) = f.parse_name();
            if n == 0 {
                return Err(Throw::new(-16));
            }
            f.code.push(Instr::Lit(f.mem[a] as Cell));
            Ok(())
        });
        self.imm("LITERAL", |f| {
            f.need_def()?;
            let v = f.pop()?;
            f.code.push(Instr::Lit(v));
            Ok(())
        });
        self.imm("[", |f| {
            f.set_state(false);
            Ok(())
        });
        self.prim("]", |f| {
            f.need_def()?;
            f.set_state(true);
            Ok(())
        });
        self.imm("POSTPONE", |f| {
            f.need_def()?;
            let name = f.name()?;
            let xt = f.find_or_err(&name)?;
            f.code.push(if f.dict[xt].immediate {
                Instr::Call(xt)
            } else {
                Instr::Compile(xt)
            });
            Ok(())
        });
        self.imm("RECURSE", |f| {
            f.need_def()?;
            f.code.push(Instr::Call(f.current.unwrap()));
            Ok(())
        });
        self.imm("EXIT", |f| {
            f.need_def()?;
            f.code.push(Instr::Exit);
            Ok(())
        });

        // control flow
        self.imm("IF", |f| {
            f.need_def()?;
            let p = f.emit_fwd(Instr::ZBranch(0));
            f.ctrl.push(Ctrl::Orig(p));
            Ok(())
        });
        self.imm("ELSE", |f| {
            f.need_def()?;
            let p = f.pop_orig()?;
            let q = f.emit_fwd(Instr::Branch(0));
            f.patch(p, q + 1);
            f.ctrl.push(Ctrl::Orig(q));
            Ok(())
        });
        self.imm("THEN", |f| {
            f.need_def()?;
            let p = f.pop_orig()?;
            let end = f.code.len();
            f.patch(p, end);
            Ok(())
        });
        self.imm("BEGIN", |f| {
            f.need_def()?;
            f.ctrl.push(Ctrl::Dest(f.code.len()));
            Ok(())
        });
        self.imm("UNTIL", |f| {
            f.need_def()?;
            let d = f.pop_dest()?;
            f.code.push(Instr::ZBranch(d));
            Ok(())
        });
        self.imm("AGAIN", |f| {
            f.need_def()?;
            let d = f.pop_dest()?;
            f.code.push(Instr::Branch(d));
            Ok(())
        });
        self.imm("WHILE", |f| {
            f.need_def()?;
            let p = f.emit_fwd(Instr::ZBranch(0));
            f.ctrl.push(Ctrl::Orig(p));
            Ok(())
        });
        self.imm("REPEAT", |f| {
            f.need_def()?;
            let p = f.pop_orig()?;
            let d = f.pop_dest()?;
            f.code.push(Instr::Branch(d));
            let end = f.code.len();
            f.patch(p, end);
            Ok(())
        });
        self.imm("DO", |f| {
            f.need_def()?;
            let p = f.emit_fwd(Instr::Do(0));
            f.ctrl.push(Ctrl::Do(p));
            Ok(())
        });
        self.imm("?DO", |f| {
            f.need_def()?;
            let p = f.emit_fwd(Instr::QDo(0));
            f.ctrl.push(Ctrl::Do(p));
            Ok(())
        });
        self.imm("LOOP", |f| f.close_loop(Instr::Loop));
        self.imm("+LOOP", |f| f.close_loop(Instr::PlusLoop));
        self.imm("LEAVE", |f| {
            f.need_def()?;
            f.code.push(Instr::Leave);
            Ok(())
        });
        self.prim("UNLOOP", |f| {
            f.loops.pop().ok_or(Throw::new(-26))?;
            Ok(())
        });
        self.prim("I", |f| {
            let i = f.loop_index(0)?;
            f.push(i);
            Ok(())
        });
        self.prim("J", |f| {
            let j = f.loop_index(1)?;
            f.push(j);
            Ok(())
        });

        // comments and strings
        self.imm("(", |f| {
            f.parse(b')');
            Ok(())
        });
        self.imm("\\", |f| {
            let end = f.parse_area().1;
            f.set_in(end);
            Ok(())
        });
        self.imm(".(", |f| {
            let (a, n) = f.parse(b')');
            f.out.extend_from_slice(&f.mem[a..a + n]);
            Ok(())
        });
        self.imm(".\"", |f| {
            let (a, n) = f.parse(b'"');
            if f.compiling() {
                f.code.push(Instr::Print(Rc::from(&f.mem[a..a + n])));
            } else {
                f.out.extend_from_slice(&f.mem[a..a + n]);
            }
            Ok(())
        });
        self.imm("S\"", |f| {
            let (a, n) = f.parse(b'"');
            if f.compiling() {
                let addr = f.compile_string(a, n)?;
                f.code.push(Instr::Lit(addr as Cell));
                f.code.push(Instr::Lit(n as Cell));
            } else {
                // Interpreted strings go to a transient buffer, leaving HERE alone.
                let buf = SQ_BUFS + f.sq_next * LINE_MAX;
                f.sq_next = 1 - f.sq_next;
                f.mem.copy_within(a..a + n, buf);
                f.push(buf as Cell);
                f.push(n as Cell);
            }
            Ok(())
        });

        // system
        self.prim("WORDS", |f| {
            f.words();
            Ok(())
        });
        self.prim("SEE", |f| f.see());
        self.prim("INCLUDE", |f| {
            let path = f.name()?;
            f.include(&path)
        });
        self.prim("BYE", |f| {
            f.bye = true;
            Ok(())
        });
    }
}

fn main() {
    let mut forth = Forth::new();
    let files: Vec<String> = env::args().skip(1).collect();
    if !files.is_empty() {
        for path in &files {
            let result = forth.include(path);
            forth.flush();
            if let Err(e) = result {
                eprintln!("{e}");
                std::process::exit(1);
            }
            if forth.bye {
                break;
            }
        }
        return;
    }

    println!("forthling — WORDS shows the dictionary, BYE quits.");
    let stdin = io::stdin();
    loop {
        io::stdout().flush().ok();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let result = forth.interpret(&line);
        forth.flush();
        match result {
            Ok(()) if forth.bye => break,
            Ok(()) if forth.current.is_some() => println!(" compiled"),
            Ok(()) => println!(" ok"),
            Err(e) => println!(" {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn output(f: Forth) -> String {
        String::from_utf8(f.out).unwrap()
    }

    fn run(src: &str) -> String {
        let mut f = Forth::new();
        f.interpret(src).unwrap();
        output(f)
    }

    fn err(src: &str) -> Throw {
        Forth::new().interpret(src).unwrap_err()
    }

    fn temp_file(name: &str, content: &str) -> String {
        let path = env::temp_dir().join(format!("forth-test-{}-{name}", std::process::id()));
        fs::write(&path, content).unwrap();
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn arithmetic() {
        assert_eq!(run("2 3 + 4 * ."), "20 ");
        assert_eq!(run("-7 2 / . -7 2 MOD ."), "-3 -1 ");
        assert_eq!(run("10 3 /MOD . ."), "3 1 ");
        assert_eq!(run("1000000 3000000 1000 */ ."), "3000000000 ");
        assert_eq!(run("-8 2/ ."), "-4 ");
    }

    #[test]
    fn stack_ops() {
        assert_eq!(run("1 2 3 ROT .S"), "<3> 2 3 1 ");
        assert_eq!(run("1 2 3 4 2SWAP .S"), "<4> 3 4 1 2 ");
        assert_eq!(run("1 2 3 4 2OVER .S"), "<6> 1 2 3 4 1 2 ");
        assert_eq!(run("5 0 ?DUP .S"), "<2> 5 0 ");
    }

    #[test]
    fn conditionals() {
        let src = ": SIGN DUP 0< IF DROP -1 ELSE 0> IF 1 ELSE 0 THEN THEN ; \
                   -5 SIGN . 0 SIGN . 9 SIGN .";
        assert_eq!(run(src), "-1 0 1 ");
    }

    #[test]
    fn recursion() {
        assert_eq!(run(": FAC DUP 1 > IF DUP 1- RECURSE * THEN ; 10 FAC ."), "3628800 ");
        assert_eq!(err(": FOREVER RECURSE ; FOREVER").code, -5);
    }

    #[test]
    fn loops() {
        assert_eq!(run(": T 5 0 DO I . LOOP ; T"), "0 1 2 3 4 ");
        assert_eq!(run(": T 0 10 DO I . -3 +LOOP ; T"), "10 7 4 1 ");
        assert_eq!(run(": T 3 0 DO 2 0 DO J 10 * I + . LOOP LOOP ; T"), "0 1 10 11 20 21 ");
        assert_eq!(run(": T 10 0 DO I 3 = IF LEAVE THEN I . LOOP ; T"), "0 1 2 ");
        assert_eq!(run(": T 0 0 ?DO 1 . LOOP ; T"), "");
        assert_eq!(run(": T 3 BEGIN DUP . 1- DUP 0= UNTIL DROP ; T"), "3 2 1 ");
        assert_eq!(run(": T 0 BEGIN DUP 3 < WHILE DUP . 1+ REPEAT DROP ; T"), "0 1 2 ");
        assert_eq!(run(": T 10 0 DO I 2 = IF UNLOOP EXIT THEN I . LOOP ; T 7 ."), "0 1 7 ");
    }

    #[test]
    fn variables_and_constants() {
        assert_eq!(run("VARIABLE X 5 X ! 3 X +! X @ ."), "8 ");
        assert_eq!(run("42 CONSTANT ANSWER ANSWER ."), "42 ");
        assert_eq!(run("CREATE BUF 3 CELLS ALLOT 9 BUF 2 CELLS + ! BUF 2 CELLS + @ ."), "9 ");
    }

    #[test]
    fn byte_addressed_memory() {
        assert_eq!(run("1 CELLS . 1 CHARS . 0 CELL+ . 0 CHAR+ ."), "8 1 8 1 ");
        // cells are little-endian bytes
        assert_eq!(run("VARIABLE V 258 V ! V C@ . V 1+ C@ ."), "2 1 ");
        assert_eq!(run("VARIABLE V -1 V ! 0 V C! V @ ."), "-256 ");
        // CREATE aligns its data field after an odd C,
        assert_eq!(run("1 C, CREATE X X 8 MOD ."), "0 ");
        assert_eq!(err("-1 @").code, -9);
        assert_eq!(err("1048570 @").code, -9);
    }

    #[test]
    fn create_does() {
        assert_eq!(run(": KONST CREATE , DOES> @ ; 7 KONST SEVEN SEVEN SEVEN + ."), "14 ");
        let counter = ": COUNTER CREATE 0 , DOES> 1 OVER +! @ ; COUNTER TICK TICK . TICK . TICK .";
        assert_eq!(run(counter), "1 2 3 ");
    }

    #[test]
    fn strings_and_chars() {
        assert_eq!(run(r#": HI ." Hello, Forth!" CR ; HI"#), "Hello, Forth!\n");
        assert_eq!(run(r#"S" abc" TYPE"#), "abc");
        assert_eq!(run(r#": T S" xyz" TYPE ; T"#), "xyz");
        assert_eq!(run("CHAR A EMIT 'B' EMIT"), "AB");
        assert_eq!(run(": T [CHAR] Z EMIT ; T"), "Z");
        assert_eq!(run("1 ( a comment ) 2 \\ rest of line\n + ."), "3 ");
    }

    #[test]
    fn transient_strings_leave_here_alone() {
        assert_eq!(run(r#"HERE S" abc" 2DROP HERE = ."#), "-1 ");
        // two S" strings are usable at the same time
        assert_eq!(run(r#"S" one" S" two" 2SWAP TYPE TYPE"#), "onetwo");
    }

    #[test]
    fn number_bases() {
        assert_eq!(run("HEX FF DECIMAL ."), "255 ");
        assert_eq!(run("255 HEX . DECIMAL"), "FF ");
        assert_eq!(run("$10 #10 %10 + + ."), "28 ");
        assert_eq!(run("-1 U."), "18446744073709551615 ");
    }

    #[test]
    fn input_buffer_and_parsing() {
        // SOURCE is the current line, >IN the offset just past the parsed word
        assert_eq!(run("SOURCE TYPE"), "SOURCE TYPE");
        assert_eq!(run(">IN @ ."), "6 ");
        // moving >IN skips or re-reads input
        assert_eq!(run("1 >IN +! x2 ."), "2 ");
        assert_eq!(run(": SKIP-REST SOURCE >IN ! DROP ; 1 . SKIP-REST 2 ."), "1 ");
        let reread = "VARIABLE N 0 N !\nN @ 1+ DUP N ! 3 < 0= 8 AND >IN +! 0 >IN ! N @ .";
        assert_eq!(run(reread), "3 ");
        // WORD and COUNT
        assert_eq!(run(": MSG 41 WORD COUNT ; MSG hi there) TYPE"), "hi there");
        assert_eq!(run("BL WORD   spaced COUNT TYPE"), "spaced");
        assert_eq!(run("BL WORD xyz C@ ."), "3 ");
    }

    #[test]
    fn find_and_state() {
        assert_eq!(run("BL WORD DUP FIND NIP ."), "-1 ");
        assert_eq!(run("BL WORD IF FIND NIP ."), "1 ");
        assert_eq!(run("BL WORD NOSUCHWORD FIND . COUNT TYPE"), "0 NOSUCHWORD");
        assert_eq!(run("BL WORD dup FIND SWAP ' DUP = . ."), "-1 -1 ");
        assert_eq!(run("STATE @ . : T [ STATE @ ] LITERAL ; T ."), "0 0 ");
        assert_eq!(run(": NOW STATE @ ; IMMEDIATE : T NOW LITERAL ; T ."), "-1 ");
    }

    #[test]
    fn include_nests_and_restores_input() {
        let inner = temp_file("inner.fs", "10 20\n+ \\ a comment\n");
        let outer = temp_file("outer.fs", &format!("1 INCLUDE {inner} 2 .S\n"));
        assert_eq!(run(&format!("INCLUDE {outer} 3 .S")), "<3> 1 30 2 <4> 1 30 2 3 ");
    }

    #[test]
    fn include_reports_error_location() {
        let path = temp_file("bad.fs", "1 2\n: OK ;\n3 FOO\n");
        let e = err(&format!("INCLUDE {path}"));
        assert_eq!(e.code, -13);
        assert!(e.to_string().ends_with(":3: error -13: undefined word: FOO"), "{e}");
        assert_eq!(err("INCLUDE /no/such/file.fs").code, -38);
    }

    #[test]
    fn byte_order_mark_is_ignored() {
        assert_eq!(run("\u{feff}1 2 + ."), "3 ");
    }

    #[test]
    fn metaprogramming() {
        assert_eq!(run(": SQ DUP * ; 4 ' SQ EXECUTE ."), "16 ");
        assert_eq!(run(": ENDIF POSTPONE THEN ; IMMEDIATE : T 1 IF 2 . ENDIF ; T"), "2 ");
        assert_eq!(run(": T [ 6 7 * ] LITERAL ; T ."), "42 ");
        assert_eq!(run(": A 1 ; : A A 1 + ; A ."), "2 ");
    }

    #[test]
    fn errors_are_throw_codes_and_reset_state() {
        let mut f = Forth::new();
        let e = f.interpret("1 2 FOO").unwrap_err();
        assert_eq!(e.code, -13);
        assert_eq!(e.to_string(), "error -13: undefined word: FOO");
        assert!(f.stack.is_empty());
        assert_eq!(f.interpret("DROP").unwrap_err().code, -4);
        assert_eq!(f.interpret(": BAD IF ;").unwrap_err().code, -22);
        assert!(f.find("BAD").is_none());
        assert!(!f.compiling());
        assert_eq!(f.interpret("1 0 /").unwrap_err().code, -10);
        assert_eq!(f.interpret("IF").unwrap_err().code, -14);
        assert_eq!(f.interpret("R>").unwrap_err().code, -6);
        assert_eq!(f.interpret("I").unwrap_err().code, -26);
        assert_eq!(f.interpret(":").unwrap_err().code, -16);
        f.interpret("1 2 + .").unwrap();
        assert_eq!(output(f), "3 ");
    }

    // ---- Forth-2012 test suite (tests/forth2012-test-suite, see STANDAARD.md)

    fn suite_file(name: &str) -> Option<String> {
        let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/forth2012-test-suite/src")
            .join(name);
        if path.exists() {
            Some(path.to_string_lossy().into_owned())
        } else {
            eprintln!("skipped: {} not found (see STANDAARD.md)", path.display());
            None
        }
    }

    #[test]
    fn suite_prelimtest() {
        let Some(path) = suite_file("prelimtest.fth") else { return };
        let mut f = Forth::new();
        f.include(&path).unwrap_or_else(|e| panic!("{e}\n{}", String::from_utf8_lossy(&f.out)));
        let out = output(f);
        assert!(!out.contains("Error #"), "{out}");
        assert!(out.contains("Pass #23: testing S\""), "{out}");
        assert!(out.contains("\n0 tests failed out of"), "{out}");
    }

    #[test]
    fn suite_tester() {
        let Some(path) = suite_file("tester.fr") else { return };
        let mut f = Forth::new();
        f.include(&path).unwrap();
        f.interpret("T{ 1 2 + -> 3 }T  T{ 1 2 SWAP -> 2 1 }T  T{ -> }T").unwrap();
        f.interpret("#ERRORS @ .").unwrap();
        assert_eq!(String::from_utf8_lossy(&f.out), "0 ");
        f.interpret("T{ 1 2 + -> 4 }T").unwrap();
        f.interpret("T{ 1 2 -> 3 }T").unwrap();
        f.interpret("#ERRORS @ .").unwrap();
        let out = output(f);
        assert!(out.contains("INCORRECT RESULT: T{ 1 2 + -> 4 }T"), "{out}");
        assert!(out.contains("WRONG NUMBER OF RESULTS: T{ 1 2 -> 3 }T"), "{out}");
        assert!(out.ends_with("2 "), "{out}");
    }
}
