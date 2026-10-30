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
const STACK_CELLS: usize = 1 << 16; // data stack size, reported by ENVIRONMENT?
const RSTACK_CELLS: usize = 1 << 16; // return stack size, idem

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
const PAD_ADDR: usize = WORD_BUF - 256;
// <# #> builds its string downwards from here, down to DATA_END
const HOLD_END: usize = PAD_ADDR;
const DATA_END: usize = PAD_ADDR - 256;

/// System THROW code: INCLUDEs nested deeper than MAX_SOURCES.
const NESTING_TOO_DEEP: Cell = -256;

/// THROW code of QUIT: back to the terminal, keeping the data stack.
const QUIT: Cell = -56;

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
\ Division is floored (see README).
: */MOD   >R M* R> FM/MOD ;
: */      */MOD NIP ;
: /MOD    >R S>D R> FM/MOD ;
: /       /MOD NIP ;
: MOD     /MOD DROP ;
: 1+  1 + ;   : 1-  1 - ;   : 2*  2 * ;
: 0=  0 = ;   : 0<  0 < ;   : 0>  0 > ;   : <>  = 0= ;
: NEGATE  0 SWAP - ;
: ABS     DUP 0< IF NEGATE THEN ;
: MIN     2DUP > IF SWAP THEN DROP ;
: MAX     2DUP < IF SWAP THEN DROP ;
: +!      DUP @ ROT + SWAP ! ;
: CELLS  8 * ;   : CELL+  8 + ;   : CHARS ;   : CHAR+  1+ ;
: ALIGNED  7 + -8 AND ;
: ALIGN    HERE ALIGNED HERE - ALLOT ;
: VARIABLE  CREATE 0 , ;
: CR      10 EMIT ;
: SPACE   BL EMIT ;
: SPACES  BEGIN DUP 0> WHILE SPACE 1- REPEAT DROP ;
: DECIMAL 10 BASE ! ;
: HEX     16 BASE ! ;
: #S      BEGIN # 2DUP OR 0= UNTIL ;
: SIGN    0< IF [CHAR] - HOLD THEN ;
: HOLDS   BEGIN DUP WHILE 1- 2DUP + C@ HOLD REPEAT 2DROP ;
: U.R     >R 0 <# #S #> R> OVER - SPACES TYPE ;
: .R      >R DUP ABS 0 <# #S ROT SIGN #> R> OVER - SPACES TYPE ;
: U.      0 U.R SPACE ;
: .       0 .R SPACE ;
: 0<>     0= 0= ;
: U>      SWAP U< ;
: WITHIN  OVER - >R - R> U< ;
: ERASE   0 FILL ;
: BUFFER: CREATE ALLOT ;
: 2>R     SWAP >R >R ;
: 2R>     R> R> SWAP ;
: 2R@     R> R@ OVER >R SWAP ;
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
            -1 | -2 => "aborted",
            -3 => "stack overflow",
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
            -17 => "pictured numeric output string overflow",
            -29 => "compiler nesting",
            -31 => ">BODY used on non-CREATEd definition",
            -32 => "invalid name argument",
            -38 => "non-existent file",
            -39 => "unexpected end of file",
            QUIT => "quit",
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
    Of(usize), // ( x1 x2 -- | x1 ): equal drops both, else jumps to the target
    Drop,      // the selector, dropped by ENDCASE
    Do(usize),  // target: just past the matching LOOP
    QDo(usize), // like Do, but skips the loop when limit == index
    Loop(usize),     // target: start of the loop body
    PlusLoop(usize), // target: start of the loop body
    Leave,
    Print(Rc<[u8]>),
    Abort(Rc<[u8]>), // ABORT": throws -2 with this message if the flag is nonzero
    Compile(usize), // POSTPONEd non-immediate word: compiles a call to it
    SetDoes(usize), // gives the latest CREATEd word the code at this offset
    Store(usize),   // compiled TO and IS: stores the top of the stack here
    Fetch(usize),   // compiled ACTION-OF: pushes the cell stored here
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
    Value(usize), // address of the value's cell
    Defer(usize), // address of the cell holding the xt to run
    Marker {
        dict_len: usize, // dictionary length before the marker, itself included
        here: usize,
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
    Case,         // an open CASE
    Of(usize),    // an OF waiting for its ENDOF
    EndOf(usize), // an ENDOF jump waiting for ENDCASE
}

/// An input source: where its text comes from, its line buffer in memory,
/// the number of lines read so far and, while a nested source runs, the
/// >IN to restore afterwards.
struct Source {
    origin: Origin,
    buf: usize,
    len: usize,
    line: usize,
    saved_in: Cell,
}

enum Origin {
    Terminal,
    Evaluate,
    File(Rc<Vec<String>>), // all lines of the file; REFILL loads the next one
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
    hld: usize, // start of the pictured numeric output string
    input: Box<dyn BufRead>, // the terminal: the REPL, ACCEPT and KEY read from here
    interactive: bool,       // flush output before waiting for input
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

/// The core of >NUMBER: accumulates digits in `base` into `ud` and stops at
/// the first character that is not a digit. Returns the new `ud` and the
/// number of characters used.
fn to_number(mut ud: u128, digits: &[u8], base: u32) -> (u128, usize) {
    let mut used = 0;
    for &c in digits {
        let Some(d) = (c as char).to_digit(base) else { break };
        ud = ud.wrapping_mul(base as u128).wrapping_add(d as u128);
        used += 1;
    }
    (ud, used)
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
                origin: Origin::Terminal,
                buf: SOURCE_AREA,
                len: 0,
                line: 0,
                saved_in: 0,
            }],
            sq_next: 0,
            hld: HOLD_END,
            input: Box::new(io::BufReader::new(io::stdin())),
            interactive: false,
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

    fn check_stack(&self) -> Res {
        if self.stack.len() > STACK_CELLS {
            return Err(Throw::new(-3));
        }
        Ok(())
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

    /// Pushes a double cell: the low cell first, the high cell on top.
    fn push_d(&mut self, d: i128) {
        self.push_ud(d as u128);
    }

    fn push_ud(&mut self, d: u128) {
        self.push(d as u64 as Cell);
        self.push((d >> 64) as u64 as Cell);
    }

    fn pop_d(&mut self) -> Res<i128> {
        Ok(self.pop_ud()? as i128)
    }

    fn pop_ud(&mut self) -> Res<u128> {
        let (lo, hi) = self.pop2()?;
        Ok((hi as u64 as u128) << 64 | lo as u64 as u128)
    }

    /// FM/MOD (floored) or SM/REM (symmetric): ( d n -- rem quot ).
    fn divide(&mut self, floored: bool) -> Res {
        let d = self.pop()? as i128;
        let n = self.pop_d()?;
        if d == 0 {
            return Err(Throw::new(-10));
        }
        // The only overflow, the smallest double divided by -1, is out of range anyway.
        let mut q = n.checked_div(d).ok_or(Throw::new(-11))?;
        let mut r = n % d;
        if floored && r != 0 && (r < 0) != (d < 0) {
            q -= 1;
            r += d;
        }
        let q = Cell::try_from(q).map_err(|_| Throw::new(-11))?;
        self.push(r as Cell);
        self.push(q);
        Ok(())
    }

    /// The run-time part of ABORT".
    fn abort_if(&mut self, msg: &[u8]) -> Res {
        if self.pop()? != 0 {
            return Err(Throw::with(-2, String::from_utf8_lossy(msg)));
        }
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

    /// The string of S" and S\": compiled into data space, or, when
    /// interpreting, put in a transient buffer so HERE is left alone.
    fn string_literal(&mut self, text: &[u8]) -> Res {
        let n = text.len();
        if self.compiling() {
            let a = self.reserve(n)?;
            self.mem[a..a + n].copy_from_slice(text);
            self.align_here();
            self.code.push(Instr::Lit(a as Cell));
            self.code.push(Instr::Lit(n as Cell));
        } else {
            let buf = SQ_BUFS + self.sq_next * LINE_MAX;
            self.sq_next = 1 - self.sq_next;
            self.mem[buf..buf + n].copy_from_slice(text);
            self.push(buf as Cell);
            self.push(n as Cell);
        }
        Ok(())
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

    /// Parses the text of S\" up to an unescaped `"` and translates its
    /// escapes, such as \n, \" and \x41.
    fn parse_escaped(&mut self) -> Vec<u8> {
        let (mut i, end) = self.parse_area();
        let mut out = Vec::new();
        while i < end {
            let c = self.mem[i];
            i += 1;
            if c == b'"' {
                break;
            }
            if c != b'\\' || i == end {
                out.push(c);
                continue;
            }
            let e = self.mem[i];
            i += 1;
            match e {
                b'a' => out.push(7),
                b'b' => out.push(8),
                b'e' => out.push(27),
                b'f' => out.push(12),
                b'l' | b'n' => out.push(10),
                b'm' => out.extend_from_slice(b"\r\n"),
                b'q' => out.push(b'"'),
                b'r' => out.push(13),
                b't' => out.push(9),
                b'v' => out.push(11),
                b'z' => out.push(0),
                b'x' => {
                    let mut v = 0;
                    for _ in 0..2 {
                        let Some(d) = self.mem[i..end].first().and_then(|&h| (h as char).to_digit(16))
                        else {
                            break;
                        };
                        v = v * 16 + d;
                        i += 1;
                    }
                    out.push(v as u8);
                }
                other => out.push(other), // \" and \\, and any other character as itself
            }
        }
        self.set_in(i);
        out
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
        let (ud, used) = to_number(0, digits.as_bytes(), base as u32);
        if used < digits.len() {
            return None;
        }
        let v = u64::try_from(ud).ok()? as Cell;
        Some(if neg { v.wrapping_neg() } else { v })
    }

    /// BASE for number output, with decimal as the fallback for an invalid BASE.
    fn base(&self) -> u64 {
        let base = self.get(BASE_ADDR);
        if (2..=36).contains(&base) {
            base as u64
        } else {
            10
        }
    }

    /// Adds a character in front of the pictured numeric output string.
    fn hold(&mut self, c: u8) -> Res {
        if self.hld <= DATA_END {
            return Err(Throw::new(-17));
        }
        self.hld -= 1;
        self.mem[self.hld] = c;
        Ok(())
    }

    fn fmt_num(&self, n: Cell) -> String {
        let s = fmt_unsigned(n.unsigned_abs(), self.base());
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
        if let Err(e) = &result {
            // QUIT leaves the data stack alone; every other error empties it.
            self.reset(e.code != QUIT);
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
        let s = self.sources.last_mut().unwrap();
        s.line += 1;
        if bytes.len() > LINE_MAX {
            return Err(Throw::with(-18, "input line too long"));
        }
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
            self.check_stack()?;
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
        let lines = src.lines().map(String::from).collect();
        let buf = SOURCE_AREA + self.sources.len() * LINE_MAX;
        self.push_source(Origin::File(Rc::new(lines)), buf, 0)?;
        // Read the line number before interpreting: a nested source may be
        // on top of the stack when an error comes back.
        let line = |f: &Forth| f.sources.last().unwrap().line;
        while self.refill().map_err(|e| e.at(path, line(self)))? {
            let n = line(self);
            self.interpret_buffer().map_err(|e| e.at(path, n))?;
            if self.bye {
                break;
            }
        }
        self.pop_source();
        Ok(())
    }

    /// Interprets a string in memory as a nested input source (EVALUATE).
    fn evaluate(&mut self, a: usize, n: usize) -> Res {
        self.push_source(Origin::Evaluate, a, n)?;
        self.interpret_buffer()?;
        self.pop_source();
        Ok(())
    }

    /// Loads the next line of the current source (REFILL); false when there
    /// is none, which is always the case for an EVALUATEd string.
    fn refill(&mut self) -> Res<bool> {
        let s = self.sources.last().unwrap();
        let line = match &s.origin {
            Origin::Evaluate => None,
            Origin::File(lines) => lines.get(s.line).cloned(),
            Origin::Terminal => self.read_line().map(|l| String::from_utf8_lossy(&l).into_owned()),
        };
        match line {
            Some(line) => self.load_line(&line).map(|_| true),
            None => Ok(false),
        }
    }

    /// SOURCE-ID: 0 for the terminal, -1 for EVALUATE. Without the FILE word
    /// set there are no file ids, so a file gets its (positive) nesting depth.
    fn source_id(&self) -> Cell {
        match self.sources.last().unwrap().origin {
            Origin::Terminal => 0,
            Origin::Evaluate => -1,
            Origin::File(_) => self.sources.len() as Cell - 1,
        }
    }

    /// Makes `buf` the current input source, remembering the >IN of the
    /// source it interrupts.
    fn push_source(&mut self, origin: Origin, buf: usize, len: usize) -> Res {
        if self.sources.len() == MAX_SOURCES {
            return Err(Throw::new(NESTING_TOO_DEEP));
        }
        let saved_in = self.get(IN_ADDR);
        self.sources.last_mut().unwrap().saved_in = saved_in;
        self.sources.push(Source {
            origin,
            buf,
            len,
            line: 0,
            saved_in: 0,
        });
        self.set(IN_ADDR, 0);
        Ok(())
    }

    /// SAVE-INPUT: >IN, the line number and the nesting depth.
    fn save_input(&mut self) {
        let s = self.sources.last().unwrap();
        let (line, depth) = (s.line as Cell, self.sources.len() as Cell);
        self.push(self.get(IN_ADDR));
        self.push(line);
        self.push(depth);
        self.push(3);
    }

    /// The core of RESTORE-INPUT; returns whether the input was restored.
    /// A file can go back to any line, the terminal and EVALUATE only within
    /// the current line.
    fn restore_input(&mut self) -> Res<bool> {
        let n = self.pop()?;
        if n != 3 {
            for _ in 0..n.max(0) {
                self.pop()?;
            }
            return Ok(false);
        }
        let depth = self.pop()?;
        let line = self.pop()?;
        let to_in = self.pop()?;
        let s = self.sources.last().unwrap();
        if depth != self.sources.len() as Cell {
            return Ok(false);
        }
        if line != s.line as Cell {
            let Origin::File(lines) = &s.origin else { return Ok(false) };
            let Some(text) = usize::try_from(line - 1).ok().and_then(|i| lines.get(i)) else {
                return Ok(false);
            };
            let text = text.clone();
            self.sources.last_mut().unwrap().line = line as usize - 1;
            self.load_line(&text)?;
        }
        self.set(IN_ADDR, to_in);
        Ok(true)
    }

    fn pop_source(&mut self) {
        self.sources.pop();
        let saved_in = self.sources.last().unwrap().saved_in;
        self.set(IN_ADDR, saved_in);
    }

    /// Recovers from an error: clears the stacks (the data stack only if asked),
    /// returns to the terminal and drops a half-finished definition.
    fn reset(&mut self, clear_stack: bool) {
        if clear_stack {
            self.stack.clear();
        }
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

    // ---- terminal input ---------------------------------------------------

    /// Reads a line from the terminal without its line ending; None at end of input.
    fn read_line(&mut self) -> Option<Vec<u8>> {
        if self.interactive {
            self.flush();
        }
        let mut line = Vec::new();
        if self.input.read_until(b'\n', &mut line).unwrap_or(0) == 0 {
            return None;
        }
        while matches!(line.last(), Some(b'\n' | b'\r')) {
            line.pop();
        }
        Some(line)
    }

    /// Reads one character from the terminal; None at end of input.
    fn read_key(&mut self) -> Option<u8> {
        if self.interactive {
            self.flush();
        }
        let c = *self.input.fill_buf().ok()?.first()?;
        self.input.consume(1);
        Some(c)
    }

    // ---- inner interpreter ------------------------------------------------

    fn execute(&mut self, xt: usize) -> Res {
        let base = self.frames.len();
        self.invoke(xt)?;
        self.run(base)
    }

    /// Runs a primitive directly, or pushes a frame for compiled code.
    fn invoke(&mut self, mut xt: usize) -> Res {
        // A deferred word runs its action; follow chains of them in a loop,
        // so a DEFER that refers to itself cannot overflow the Rust stack.
        let mut hops = 0;
        while let Some(Word { kind: Kind::Defer(addr), name, .. }) = self.dict.get(xt) {
            match self.get(*addr) {
                -1 => return Err(Throw::with(-21, format!("{name} has no action yet"))),
                action => xt = action as usize,
            }
            hops += 1;
            if hops > MAX_FRAMES {
                return Err(Throw::new(-5));
            }
        }
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
            Kind::Value(addr) => {
                self.push(self.get(addr));
                Ok(())
            }
            Kind::Defer(_) => unreachable!("resolved above"),
            Kind::Marker { dict_len, here } => {
                self.dict.truncate(dict_len);
                self.here = here;
                Ok(())
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
            self.check_stack()?;
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
                Instr::Of(t) => {
                    let (x1, x2) = self.pop2()?;
                    if x1 != x2 {
                        self.push(x1);
                        self.jump(t);
                    }
                }
                Instr::Drop => {
                    self.pop()?;
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
                Instr::Abort(s) => self.abort_if(&s)?,
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
                Instr::Store(addr) => {
                    let v = self.pop()?;
                    self.set(addr, v);
                }
                Instr::Fetch(addr) => self.push(self.get(addr)),
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

    /// Starts a colon definition (hidden until `;`) and enters compilation state.
    fn start_def(&mut self, name: &str) -> Res<usize> {
        if self.current.is_some() {
            return Err(Throw::new(-29));
        }
        let xt = self.define(name, Kind::Colon(Rc::new(Vec::new())));
        self.dict[xt].hidden = true;
        self.current = Some(xt);
        self.set_state(true);
        self.code.clear();
        self.ctrl.clear();
        Ok(xt)
    }

    /// The cell behind a VALUE (`defer` false) or a DEFER (`defer` true).
    fn cell_of(&self, xt: Cell, defer: bool) -> Res<usize> {
        let word = usize::try_from(xt).ok().and_then(|i| self.dict.get(i));
        match word.map(|w| &w.kind) {
            Some(Kind::Value(addr)) if !defer => Ok(*addr),
            Some(Kind::Defer(addr)) if defer => Ok(*addr),
            _ => Err(Throw::new(-32)),
        }
    }

    /// Parses a name and returns the cell behind it, as `cell_of`.
    fn named_cell(&mut self, defer: bool) -> Res<usize> {
        let name = self.name()?;
        let xt = self.find_or_err(&name)?;
        self.cell_of(xt as Cell, defer).map_err(|e| Throw { detail: Some(name), ..e })
    }

    /// Stores into a VALUE or DEFER cell now, or compiles the store.
    fn store_to(&mut self, addr: usize) -> Res {
        if self.compiling() {
            self.code.push(Instr::Store(addr));
        } else {
            let v = self.pop()?;
            self.set(addr, v);
        }
        Ok(())
    }

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
            Instr::Branch(t) | Instr::ZBranch(t) | Instr::Do(t) | Instr::QDo(t) | Instr::Of(t) => {
                *t = target
            }
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
            Instr::Of(t) => format!("(OF) {t}"),
            Instr::Drop => "DROP".into(),
            Instr::Do(t) => format!("(DO) {t}"),
            Instr::QDo(t) => format!("(?DO) {t}"),
            Instr::Loop(t) => format!("(LOOP) {t}"),
            Instr::PlusLoop(t) => format!("(+LOOP) {t}"),
            Instr::Leave => "LEAVE".into(),
            Instr::Print(s) => format!(".\" {}\"", String::from_utf8_lossy(s)),
            Instr::Abort(s) => format!("ABORT\" {}\"", String::from_utf8_lossy(s)),
            Instr::Compile(xt) => format!("POSTPONE {}", self.dict[*xt].name),
            Instr::SetDoes(_) => "DOES>".into(),
            Instr::Store(addr) => format!("{addr} !"),
            Instr::Fetch(addr) => format!("{addr} @"),
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
            Kind::Value(addr) => format!("{} VALUE {}", self.get(*addr), w.name),
            Kind::Defer(addr) => match self.dict.get(self.get(*addr) as usize) {
                Some(action) => format!("DEFER {} IS {}", w.name, action.name),
                None => format!("DEFER {}", w.name),
            },
            Kind::Marker { .. } => format!("MARKER {}", w.name),
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
        self.prim("ROLL", |f| {
            let n = f.pop()?;
            let len = f.stack.len() as Cell;
            if n < 0 || n >= len {
                return Err(Throw::new(-4));
            }
            let x = f.stack.remove((len - 1 - n) as usize);
            f.push(x);
            Ok(())
        });
        self.prim("DEPTH", |f| {
            f.push(f.stack.len() as Cell);
            Ok(())
        });
        self.prim(">R", |f| {
            let a = f.pop()?;
            if f.rstack.len() >= RSTACK_CELLS {
                return Err(Throw::new(-5));
            }
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
        // / MOD /MOD */ */MOD are defined in the prelude via FM/MOD.

        // double cells
        self.prim("S>D", |f| {
            let n = f.pop()?;
            f.push_d(n as i128);
            Ok(())
        });
        self.prim("M*", |f| {
            let (a, b) = f.pop2()?;
            f.push_d(a as i128 * b as i128);
            Ok(())
        });
        self.prim("UM*", |f| {
            let (a, b) = f.pop2()?;
            f.push_ud(a as u64 as u128 * b as u64 as u128);
            Ok(())
        });
        self.prim("UM/MOD", |f| {
            let d = f.pop()? as u64 as u128;
            let n = f.pop_ud()?;
            if d == 0 {
                return Err(Throw::new(-10));
            }
            let q = u64::try_from(n / d).map_err(|_| Throw::new(-11))?;
            f.push((n % d) as u64 as Cell);
            f.push(q as Cell);
            Ok(())
        });
        self.prim("FM/MOD", |f| f.divide(true));
        self.prim("SM/REM", |f| f.divide(false));
        self.prim("2@", |f| {
            let a = f.pop()?;
            let i = f.check(a, 2 * CELL as Cell)?;
            f.push(f.get(i + CELL));
            f.push(f.get(i));
            Ok(())
        });
        self.prim("2!", |f| {
            let a = f.pop()?;
            let (x1, x2) = f.pop2()?;
            let i = f.check(a, 2 * CELL as Cell)?;
            f.set(i, x2);
            f.set(i + CELL, x1);
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
        self.prim("UNUSED", |f| {
            f.push((DATA_END - f.here) as Cell);
            Ok(())
        });
        self.prim("PAD", |f| {
            f.push(PAD_ADDR as Cell);
            Ok(())
        });
        self.prim("FILL", |f| {
            let c = f.pop()?;
            let (a, n) = f.pop2()?;
            if n > 0 {
                let i = f.check(a, n)?;
                f.mem[i..i + n as usize].fill(c as u8);
            }
            Ok(())
        });
        self.prim("MOVE", |f| {
            let n = f.pop()?;
            let (src, dst) = f.pop2()?;
            if n > 0 {
                let s = f.check(src, n)?;
                let d = f.check(dst, n)?;
                // copy_within handles overlapping ranges in either direction
                f.mem.copy_within(s..s + n as usize, d);
            }
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
        // pictured numeric output; #S SIGN HOLDS . U. are in the prelude
        self.prim("<#", |f| {
            f.hld = HOLD_END;
            Ok(())
        });
        self.prim("HOLD", |f| {
            let c = f.pop()?;
            f.hold(c as u8)
        });
        self.prim("#", |f| {
            let ud = f.pop_ud()?;
            let base = f.base() as u128;
            let d = std::char::from_digit((ud % base) as u32, base as u32).unwrap();
            f.hold(d.to_ascii_uppercase() as u8)?;
            f.push_ud(ud / base);
            Ok(())
        });
        self.prim("#>", |f| {
            f.pop2()?;
            f.push(f.hld as Cell);
            f.push((HOLD_END - f.hld) as Cell);
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
        self.prim("PARSE", |f| {
            let delim = f.pop()? as u8;
            let (a, n) = f.parse(delim);
            f.push(a as Cell);
            f.push(n as Cell);
            Ok(())
        });
        self.prim("PARSE-NAME", |f| {
            let (a, n) = f.parse_name();
            f.push(a as Cell);
            f.push(n as Cell);
            Ok(())
        });
        self.prim("REFILL", |f| {
            let more = f.refill()?;
            f.push(flag(more));
            Ok(())
        });
        self.prim("SOURCE-ID", |f| {
            f.push(f.source_id());
            Ok(())
        });
        self.prim("SAVE-INPUT", |f| {
            f.save_input();
            Ok(())
        });
        self.prim("RESTORE-INPUT", |f| {
            let restored = f.restore_input()?;
            f.push(flag(!restored));
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
            let name = f.name()?;
            f.start_def(&name)?;
            Ok(())
        });
        self.prim(":NONAME", |f| {
            let xt = f.start_def("")?;
            f.push(xt as Cell);
            Ok(())
        });
        self.imm(";", |f| {
            let xt = f.current.ok_or(Throw::new(-22))?;
            if !f.ctrl.is_empty() {
                return Err(Throw::new(-22));
            }
            f.dict[xt].kind = Kind::Colon(Rc::new(std::mem::take(&mut f.code)));
            // A :NONAME definition is only reachable through its xt.
            f.dict[xt].hidden = f.dict[xt].name.is_empty();
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
        self.prim("VALUE", |f| {
            let name = f.name()?;
            let v = f.pop()?;
            f.align_here();
            let addr = f.reserve(CELL)?;
            f.set(addr, v);
            f.define(&name, Kind::Value(addr));
            Ok(())
        });
        self.prim("DEFER", |f| {
            let name = f.name()?;
            f.align_here();
            let addr = f.reserve(CELL)?;
            f.set(addr, -1); // no action yet
            f.define(&name, Kind::Defer(addr));
            Ok(())
        });
        self.prim("MARKER", |f| {
            let name = f.name()?;
            let (dict_len, here) = (f.dict.len(), f.here);
            f.define(&name, Kind::Marker { dict_len, here });
            Ok(())
        });
        // TO, IS and ACTION-OF parse a name; compiled, they become Store/Fetch.
        self.imm("TO", |f| {
            let addr = f.named_cell(false)?;
            f.store_to(addr)
        });
        self.imm("IS", |f| {
            let addr = f.named_cell(true)?;
            f.store_to(addr)
        });
        self.imm("ACTION-OF", |f| {
            let addr = f.named_cell(true)?;
            if f.compiling() {
                f.code.push(Instr::Fetch(addr));
            } else {
                f.push(f.get(addr));
            }
            Ok(())
        });
        self.prim("DEFER@", |f| {
            let xt = f.pop()?;
            let addr = f.cell_of(xt, true)?;
            f.push(f.get(addr));
            Ok(())
        });
        self.prim("DEFER!", |f| {
            let (action, xt) = f.pop2()?;
            let addr = f.cell_of(xt, true)?;
            f.set(addr, action);
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
        self.prim(">BODY", |f| {
            let xt = f.pop()?;
            match usize::try_from(xt).ok().and_then(|i| f.dict.get(i)) {
                Some(Word { kind: Kind::Created { addr, .. }, .. }) => {
                    let addr = *addr;
                    f.push(addr);
                    Ok(())
                }
                _ => Err(Throw::new(-31)),
            }
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
        self.prim("COMPILE,", |f| {
            f.need_def()?;
            let xt = f.pop()?;
            match usize::try_from(xt).ok().filter(|&i| i < f.dict.len()) {
                Some(xt) => f.code.push(Instr::Call(xt)),
                None => return Err(Throw::with(-24, format!("invalid execution token {xt}"))),
            }
            Ok(())
        });
        self.imm("[COMPILE]", |f| {
            f.need_def()?;
            let name = f.name()?;
            let xt = f.find_or_err(&name)?;
            f.code.push(Instr::Call(xt));
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
        // WHILE ( dest -- orig dest ) leaves its orig under the dest, so
        // several WHILEs can share one BEGIN; REPEAT is AGAIN THEN.
        self.imm("WHILE", |f| {
            f.need_def()?;
            let d = f.pop_dest()?;
            let p = f.emit_fwd(Instr::ZBranch(0));
            f.ctrl.push(Ctrl::Orig(p));
            f.ctrl.push(Ctrl::Dest(d));
            Ok(())
        });
        self.imm("REPEAT", |f| {
            f.need_def()?;
            let d = f.pop_dest()?;
            f.code.push(Instr::Branch(d));
            let p = f.pop_orig()?;
            let end = f.code.len();
            f.patch(p, end);
            Ok(())
        });
        self.imm("CASE", |f| {
            f.need_def()?;
            f.ctrl.push(Ctrl::Case);
            Ok(())
        });
        self.imm("OF", |f| {
            f.need_def()?;
            let p = f.emit_fwd(Instr::Of(0));
            f.ctrl.push(Ctrl::Of(p));
            Ok(())
        });
        self.imm("ENDOF", |f| {
            f.need_def()?;
            let Some(Ctrl::Of(p)) = f.ctrl.pop() else {
                return Err(Throw::new(-22));
            };
            let q = f.emit_fwd(Instr::Branch(0));
            f.patch(p, q + 1);
            f.ctrl.push(Ctrl::EndOf(q));
            Ok(())
        });
        // ENDCASE drops the selector when no OF matched; every ENDOF jumps past that.
        self.imm("ENDCASE", |f| {
            f.need_def()?;
            f.code.push(Instr::Drop);
            let end = f.code.len();
            loop {
                match f.ctrl.pop() {
                    Some(Ctrl::EndOf(p)) => f.patch(p, end),
                    Some(Ctrl::Case) => return Ok(()),
                    _ => return Err(Throw::new(-22)),
                }
            }
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
        self.imm("ABORT\"", |f| {
            let (a, n) = f.parse(b'"');
            let msg: Rc<[u8]> = Rc::from(&f.mem[a..a + n]);
            if f.compiling() {
                f.code.push(Instr::Abort(msg));
                Ok(())
            } else {
                f.abort_if(&msg)
            }
        });
        self.imm("S\"", |f| {
            let (a, n) = f.parse(b'"');
            let text = f.mem[a..a + n].to_vec();
            f.string_literal(&text)
        });
        self.imm("S\\\"", |f| {
            let text = f.parse_escaped();
            f.string_literal(&text)
        });
        self.imm("C\"", |f| {
            f.need_def()?;
            let (a, n) = f.parse(b'"');
            if n > 255 {
                return Err(Throw::with(-18, "counted string longer than 255 characters"));
            }
            let addr = f.reserve(n + 1)?;
            f.mem[addr] = n as u8;
            f.mem.copy_within(a..a + n, addr + 1);
            f.align_here();
            f.code.push(Instr::Lit(addr as Cell));
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
        self.prim("ACCEPT", |f| {
            let (a, n) = f.pop2()?;
            let n = n.max(0);
            let i = f.check(a, n)?;
            let line = f.read_line().unwrap_or_default();
            let len = line.len().min(n as usize);
            f.mem[i..i + len].copy_from_slice(&line[..len]);
            f.push(len as Cell);
            Ok(())
        });
        self.prim("KEY", |f| {
            let c = f.read_key().ok_or(Throw::new(-39))?;
            f.push(c as Cell);
            Ok(())
        });
        self.prim(">NUMBER", |f| {
            let (a, n) = f.pop2()?;
            let ud = f.pop_ud()?;
            let n = n.max(0);
            let i = f.check(a, n)?;
            let base = f.base() as u32;
            let (ud, used) = to_number(ud, &f.mem[i..i + n as usize], base);
            f.push_ud(ud);
            f.push(a + used as Cell);
            f.push(n - used as Cell);
            Ok(())
        });
        self.prim("EVALUATE", |f| {
            let (a, n) = f.pop2()?;
            let n = n.max(0);
            let i = f.check(a, n)?;
            f.evaluate(i, n as usize)
        });
        self.prim("ABORT", |_| Err(Throw::new(-1)));
        self.prim("QUIT", |_| Err(Throw::new(QUIT)));
        self.prim("ENVIRONMENT?", |f| {
            let (a, n) = f.pop2()?;
            let i = f.check(a, n.max(0))?;
            let query = f.text(i, n.max(0) as usize).to_ascii_uppercase();
            match query.as_str() {
                "/COUNTED-STRING" => f.push(255),
                "/HOLD" => f.push((HOLD_END - DATA_END) as Cell),
                "/PAD" => f.push((WORD_BUF - PAD_ADDR) as Cell),
                "ADDRESS-UNIT-BITS" => f.push(8),
                "FLOORED" => f.push(-1),
                "MAX-CHAR" => f.push(255),
                "MAX-D" => f.push_d(i128::MAX),
                "MAX-N" => f.push(Cell::MAX),
                "MAX-U" => f.push(-1),
                "MAX-UD" => f.push_ud(u128::MAX),
                "RETURN-STACK-CELLS" => f.push(RSTACK_CELLS as Cell),
                "STACK-CELLS" => f.push(STACK_CELLS as Cell),
                _ => {
                    f.push(0); // unknown query
                    return Ok(());
                }
            }
            f.push(-1);
            Ok(())
        });
        self.prim("BYE", |f| {
            f.bye = true;
            Ok(())
        });
    }
}

fn main() {
    let mut forth = Forth::new();
    forth.interactive = true;
    let files: Vec<String> = env::args().skip(1).collect();
    if !files.is_empty() {
        for path in &files {
            let result = forth.include(path);
            forth.flush();
            match result {
                // There is no terminal to return to, so QUIT ends the run.
                Err(e) if e.code == QUIT => break,
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
                Ok(()) => {}
            }
            if forth.bye {
                break;
            }
        }
        return;
    }

    println!("forthling — WORDS shows the dictionary, BYE quits.");
    // The REPL reads through the same buffer as ACCEPT and KEY.
    while let Some(line) = forth.read_line() {
        let result = forth.interpret(&String::from_utf8_lossy(&line));
        forth.flush();
        match result {
            Ok(()) if forth.bye => break,
            Ok(()) if forth.current.is_some() => println!(" compiled"),
            Ok(()) => println!(" ok"),
            Err(e) if e.code == QUIT => println!(),
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
        assert_eq!(run("-7 2 / . -7 2 MOD ."), "-4 1 ");
        assert_eq!(run("7 -2 / . 7 -2 MOD ."), "-4 -1 ");
        assert_eq!(run("10 3 /MOD . ."), "3 1 ");
        assert_eq!(run("1000000 3000000 1000 */ ."), "3000000000 ");
        assert_eq!(run("-7 2 1 */MOD . ."), "-14 0 ");
        assert_eq!(run("-8 2/ ."), "-4 ");
    }

    #[test]
    fn double_cells() {
        assert_eq!(run("-5 S>D . ."), "-1 -5 ");
        assert_eq!(run("-1 -1 UM* . U."), "-2 1 ");
        assert_eq!(run("-1 2 M* . ."), "-1 -2 ");
        assert_eq!(run("0 1 4 UM/MOD . ."), "4611686018427387904 0 ");
        assert_eq!(run("-7 S>D 2 FM/MOD . . -7 S>D 2 SM/REM . ."), "-4 1 -3 -1 ");
        assert_eq!(run("7 S>D -2 FM/MOD . . 7 S>D -2 SM/REM . ."), "-4 -1 -3 1 ");
        assert_eq!(run("CREATE P 2 CELLS ALLOT 1 2 P 2! P @ . P 2@ . ."), "2 2 1 ");
        assert_eq!(err("0 1 1 UM/MOD").code, -11);
        assert_eq!(err("1 0 0 FM/MOD").code, -10);
        assert_eq!(err("0 -9223372036854775808 -1 SM/REM").code, -11);
    }

    #[test]
    fn pictured_output() {
        assert_eq!(run("<# 66 HOLD 65 HOLD 0 0 #> TYPE"), "AB");
        assert_eq!(run("<# 1 0 # # #> TYPE"), "01");
        assert_eq!(run("<# -1 SIGN 0 SIGN 255 0 #S #> TYPE"), "255-");
        assert_eq!(run("255 HEX <# 0 #S -1 SIGN #> TYPE"), "-FF");
        assert_eq!(run(r#"<# 0 0 #S S" x=" HOLDS #> TYPE"#), "x=0");
        assert_eq!(run("-1 -1 2 BASE ! <# #S #> NIP DECIMAL ."), "128 ");
        assert_eq!(run("0 . -42 . -1 U. -9223372036854775808 ."), "0 -42 18446744073709551615 -9223372036854775808 ");
        assert_eq!(run("12 1 BASE ! . DECIMAL"), "12 ");
        assert_eq!(err(": T <# 300 0 DO 65 HOLD LOOP ; T").code, -17);
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
        let two_whiles = ": T BEGIN DUP 2 > WHILE DUP 5 < WHILE 1+ REPEAT 123 ELSE 345 THEN ; \
                          1 T . . 3 T . .";
        assert_eq!(run(two_whiles), "345 1 123 5 ");
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
        assert_eq!(run("0 ALIGNED . 1 ALIGNED . 8 ALIGNED . 9 ALIGNED ."), "0 8 8 16 ");
        assert_eq!(run("1 C, ALIGN HERE 8 MOD ."), "0 ");
        assert_eq!(run("CREATE B 8 ALLOT B 8 65 FILL B 3 66 FILL B 0 67 FILL B 8 TYPE"), "BBBAAAAA");
        let moves = r#"CREATE B 8 ALLOT S" abcdef" B SWAP MOVE
                       B B 2 + 3 MOVE B 6 TYPE  B 2 + B 3 MOVE B 6 TYPE  B B 0 MOVE"#;
        assert_eq!(run(moves), "ababcfabcbcf");
        assert_eq!(run("-5 0 0 FILL 0 -1 0 MOVE"), "");
        assert_eq!(err("-5 8 0 FILL").code, -9);
        assert_eq!(err("0 -5 8 MOVE").code, -9);
        assert_eq!(err("-1 @").code, -9);
        assert_eq!(err("1048570 @").code, -9);
    }

    #[test]
    fn create_does() {
        assert_eq!(run(": KONST CREATE , DOES> @ ; 7 KONST SEVEN SEVEN SEVEN + ."), "14 ");
        let counter = ": COUNTER CREATE 0 , DOES> 1 OVER +! @ ; COUNTER TICK TICK . TICK . TICK .";
        assert_eq!(run(counter), "1 2 3 ");
        assert_eq!(run("CREATE X 5 , ' X >BODY X = . ' X >BODY @ ."), "-1 5 ");
        assert_eq!(run(": K CREATE , DOES> @ ; 3 K Y ' Y >BODY @ ."), "3 ");
        assert_eq!(err("' DUP >BODY").code, -31);
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
        assert_eq!(run("18446744073709551615 -9223372036854775808 . ."), "-9223372036854775808 -1 ");
        assert_eq!(err("18446744073709551616").code, -13);
        assert_eq!(err("12A").code, -13);
    }

    #[test]
    fn to_number_stops_at_the_first_non_digit() {
        assert_eq!(run(r#"0 0 S" 123xy" >NUMBER TYPE . ."#), "xy0 123 ");
        assert_eq!(run(r#"1 0 S" 23" >NUMBER . DROP . ."#), "0 0 123 ");
        assert_eq!(run(r#"0 0 S" ff" HEX >NUMBER DECIMAL . DROP . ."#), "0 0 255 ");
        assert_eq!(run(r#"0 0 S" -5" >NUMBER . DROP . ."#), "2 0 0 ");
        let max_ud = r#"0 0 S" 340282366920938463463374607431768211455" >NUMBER . DROP U. U."#;
        assert_eq!(run(max_ud), "0 18446744073709551615 18446744073709551615 ");
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

    fn run_with_input(src: &str, input: &str) -> String {
        let mut f = Forth::new();
        f.input = Box::new(io::Cursor::new(input.as_bytes().to_vec()));
        f.interpret(src).unwrap();
        output(f)
    }

    #[test]
    fn accept_and_key_read_the_terminal() {
        let accept = "CREATE B 10 ALLOT : T B 10 ACCEPT B SWAP TYPE [CHAR] | EMIT ; T T T";
        assert_eq!(run_with_input(accept, "hello\r\nforthling rocks\n"), "hello|forthling ||");
        assert_eq!(run_with_input("KEY . KEY . KEY .", "ab\n"), "97 98 10 ");
        assert_eq!(run_with_input("CREATE B 4 ALLOT B 4 ACCEPT . KEY .", "x\ny"), "1 121 ");
        let mut f = Forth::new();
        f.input = Box::new(io::Cursor::new(Vec::new()));
        assert_eq!(f.interpret("KEY").unwrap_err().code, -39);
    }

    #[test]
    fn abort_and_quit() {
        let mut f = Forth::new();
        let e = f.interpret("1 2 ABORT 3").unwrap_err();
        assert_eq!((e.code, f.stack.len()), (-1, 0));
        f.interpret(r#": CHECK  DUP 0< ABORT" negative!" ;  5 CHECK"#).unwrap();
        assert_eq!(f.stack, [5]);
        let e = f.interpret("-5 CHECK").unwrap_err();
        assert_eq!(e.code, -2);
        assert_eq!(e.to_string(), "error -2: aborted: negative!");
        assert!(f.stack.is_empty());
        assert_eq!(f.interpret(r#"1 ABORT" direct""#).unwrap_err().code, -2);
        f.interpret(r#"0 ABORT" not shown""#).unwrap();
        // QUIT empties the return stack and leaves compilation, but keeps the data stack.
        let e = f.interpret(": Q 7 8 >R QUIT ; 1 2 Q 3").unwrap_err();
        assert_eq!(e.code, QUIT);
        assert_eq!((f.stack.as_slice(), f.rstack.len()), ([1, 2, 7].as_slice(), 0));
        f.interpret(": HALF [ QUIT").unwrap_err();
        assert!(!f.compiling() && f.find("HALF").is_none());
        assert_eq!(output(f), "");
    }

    #[test]
    fn core_ext_basics() {
        assert_eq!(run("1 2 3 4 3 ROLL .S"), "<4> 2 3 4 1 ");
        assert_eq!(run("5 0<> . 0 0<> . -1 1 U> . 1 -1 U> ."), "-1 0 -1 0 ");
        assert_eq!(run("2 1 3 WITHIN . 3 1 3 WITHIN . -1 -5 5 WITHIN ."), "-1 0 -1 ");
        assert_eq!(run(": T 1 2 2>R 2R@ 2R> ; T .S"), "<4> 1 2 1 2 ");
        assert_eq!(run("42 5 .R -42 5 .R 42 1 .R 7 3 U.R"), "   42  -4242  7");
        assert_eq!(run("PAD 3 65 FILL PAD 1 ERASE PAD C@ . PAD 1+ C@ ."), "0 65 ");
        assert_eq!(run("UNUSED HERE 100 ALLOT DROP UNUSED - ."), "100 ");
    }

    #[test]
    fn noname_and_compiling_words() {
        assert_eq!(run(":NONAME 6 7 * ; EXECUTE ."), "42 ");
        assert_eq!(run(":NONAME DUP IF 1- RECURSE THEN ; 5 SWAP EXECUTE ."), "0 ");
        assert_eq!(run(":NONAME 1 ; DROP CREATE E 0 C, E FIND NIP ."), "0 ");
        assert_eq!(run(": Q ['] DUP COMPILE, ; IMMEDIATE : T Q * ; 7 T ."), "49 ");
        assert_eq!(run(": MYIF [COMPILE] IF ; IMMEDIATE : T MYIF 1 THEN ; -1 T ."), "1 ");
        assert_eq!(run(r#": T C" abc" ; T COUNT TYPE T C@ ."#), "abc3 ");
        assert_eq!(err(r#"C" abc""#).code, -14);
        assert_eq!(err("5 COMPILE,").code, -14);
    }

    #[test]
    fn escaped_strings() {
        let codes = |lit: &str| run(&format!(": T {lit} 0 ?DO DUP I + C@ . LOOP DROP ; T"));
        assert_eq!(codes(r#"S\" a\tb\"\\\x41\x4a\m""#), "97 9 98 34 92 65 74 13 10 ");
        assert_eq!(codes(r#"S\" \z\q\xF""#), "0 34 15 ");
        assert_eq!(run(r#"S\" x\"y" TYPE"#), "x\"y");
    }

    #[test]
    fn parse_refill_and_input_state() {
        assert_eq!(run("CHAR | PARSE abc| TYPE PARSE-NAME   word TYPE"), "abcword");
        assert_eq!(run(": T PARSE-NAME NIP . ; T"), "0 ");
        assert_eq!(run(r#"SOURCE-ID . S" SOURCE-ID ." EVALUATE S" REFILL ." EVALUATE"#), "0 -1 0 ");
        // REFILL in a file moves to the next line; RESTORE-INPUT can go back.
        // REFILL continues in the next line; the rest of the current one is skipped.
        let file = temp_file("refill.fs", "SOURCE-ID 0> . REFILL 99 .\n. 2 .\n");
        assert_eq!(run(&format!("INCLUDE {file}")), "-1 -1 2 ");
        let lines = "VARIABLE N  : AGAIN? N @ 3 < IF 2OVER 2OVER RESTORE-INPUT . ELSE 2DROP 2DROP THEN ;\n\
                     SAVE-INPUT\n\
                     1 N +! AGAIN?\n\
                     N @ .\n";
        let file = temp_file("restore.fs", lines);
        assert_eq!(run(&format!("INCLUDE {file}")), "0 0 3 ");
        // Another source cannot be restored: the flag is true.
        assert_eq!(run(r#"SAVE-INPUT S" RESTORE-INPUT ." EVALUATE"#), "-1 ");
        assert_eq!(run_with_input("REFILL", "1 2 + . SOURCE TYPE\n"), "3 1 2 + . SOURCE TYPE");
    }

    #[test]
    fn case_of_endof_endcase() {
        let src = ": T CASE 1 OF 10 ENDOF 2 OF 20 ENDOF DUP 100 * SWAP ENDCASE ; \
                   1 T . 2 T . 3 T . : E CASE ENDCASE ; 5 E DEPTH .";
        assert_eq!(run(src), "10 20 300 0 ");
        let nested = ": N CASE 1 OF CASE 2 OF 12 ENDOF 0 SWAP ENDCASE ENDOF 9 SWAP ENDCASE ; \
                      2 1 N . 3 1 N . 1 7 N . DEPTH .";
        assert_eq!(run(nested), "12 0 9 1 ");
        assert_eq!(err(": BAD CASE 1 OF ENDCASE ;").code, -22);
        assert_eq!(err(": BAD IF ENDCASE ;").code, -22);
        assert_eq!(err(": BAD CASE 1 OF THEN ;").code, -22);
        assert_eq!(run("2 CELLS BUFFER: B 7 B CELL+ ! B CELL+ @ . B 8 MOD ."), "7 0 ");
    }

    #[test]
    fn values_defers_and_markers() {
        assert_eq!(run("5 VALUE V V . 7 TO V V . : T 9 TO V ; T V ."), "5 7 9 ");
        let defer = "DEFER D ' 1+ IS D 5 D . : T ['] 2* IS D ; T 5 D . ACTION-OF D ' 2* = .";
        assert_eq!(run(defer), "6 10 -1 ");
        assert_eq!(run("DEFER D ' NEGATE ' D DEFER! 3 D . ' D DEFER@ ' NEGATE = ."), "-3 -1 ");
        assert_eq!(run("DEFER D : A ACTION-OF D ; ' DUP IS D A ' DUP = ."), "-1 ");
        assert_eq!(err("DEFER D D").code, -21);
        assert_eq!(err("DEFER D ' D IS D D").code, -5);
        assert_eq!(err("5 CONSTANT C 6 TO C").code, -32);
        assert_eq!(err("0 VALUE V ' V DEFER@").code, -32);
        assert_eq!(run("MARKER M : A 1 ; MARKER N : A 2 ; A . N A . M"), "2 1 ");
        assert_eq!(err("MARKER M : A 1 ; M A").code, -13);
        assert_eq!(err("MARKER M M M").code, -13);
        assert_eq!(run("HERE MARKER M 100 ALLOT M HERE = ."), "-1 ");
        assert_eq!(run("1 VALUE X MARKER M 2 TO X M X ."), "2 ");
    }

    #[test]
    fn stack_limits() {
        assert_eq!(err(": T BEGIN 1 AGAIN ; T").code, -3);
        assert_eq!(err(": T BEGIN 1 >R AGAIN ; T").code, -5);
    }

    #[test]
    fn environment_queries() {
        let q = |s: &str| run(&format!(r#"S" {s}" ENVIRONMENT? .S"#));
        assert_eq!(q("ADDRESS-UNIT-BITS"), "<2> 8 -1 ");
        assert_eq!(q("floored"), "<2> -1 -1 ");
        assert_eq!(q("/HOLD"), "<2> 256 -1 ");
        assert_eq!(q("MAX-N"), "<2> 9223372036854775807 -1 ");
        assert_eq!(q("MAX-U"), "<2> -1 -1 ");
        assert_eq!(q("MAX-D"), "<3> -1 9223372036854775807 -1 ");
        assert_eq!(q("MAX-UD"), "<3> -1 -1 -1 ");
        assert_eq!(q("STACK-CELLS"), "<2> 65536 -1 ");
        assert_eq!(q("NO-SUCH-THING"), "<1> 0 ");
    }

    #[test]
    fn evaluate_is_a_nested_source() {
        assert_eq!(run(r#"S" 1 2 + ." EVALUATE 4 ."#), "3 4 ");
        assert_eq!(run(r#": E S" 6 7 *" EVALUATE ; E ."#), "42 ");
        assert_eq!(run(r#"S" : SQ DUP * ;" EVALUATE 5 SQ ."#), "25 ");
        assert_eq!(run(r#"S" CHAR A EMIT SOURCE TYPE" EVALUATE"#), "ACHAR A EMIT SOURCE TYPE");
        let nested = r#": INNER S" 2 3 +" EVALUATE ; S" INNER 10 *" EVALUATE ."#;
        assert_eq!(run(nested), "50 ");
        assert_eq!(run(r#": F S" 9 DUP" ; : G F EVALUATE ; IMMEDIATE : H G + ; H ."#), "18 ");
        assert_eq!(err(r#"S" 1 FOO" EVALUATE"#).code, -13);
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
    fn suite_core() {
        let (Some(tester), Some(core)) = (suite_file("tester.fr"), suite_file("core.fr")) else {
            return;
        };
        let mut f = Forth::new();
        // core.fr's ACCEPT test asks for a line of input
        f.input = Box::new(io::Cursor::new(b"typed line\n".to_vec()));
        f.include(&tester).unwrap();
        let result = f.include(&core);
        let out = String::from_utf8_lossy(&f.out).into_owned();
        result.unwrap_or_else(|e| panic!("{e}\n{out}"));
        assert!(!out.contains("INCORRECT RESULT"), "{out}");
        assert!(!out.contains("WRONG NUMBER OF RESULTS"), "{out}");
        assert!(out.contains("RECEIVED: \"typed line\""), "{out}");
        assert!(out.contains("End of Core word set tests"), "{out}");
        f.interpret("#ERRORS @ .").unwrap();
        assert!(output(f).ends_with("0 "));
    }

    /// The CORE EXT tests, preceded by what runtests.fth loads before them.
    #[test]
    fn suite_core_ext() {
        let files = ["tester.fr", "core.fr", "coreplustest.fth", "utilities.fth", "errorreport.fth", "coreexttest.fth"];
        let Some(paths) = files.iter().map(|name| suite_file(name)).collect::<Option<Vec<_>>>() else {
            return;
        };
        let mut f = Forth::new();
        f.input = Box::new(io::Cursor::new(b"typed line\n".to_vec()));
        for path in &paths {
            let result = f.include(path);
            let out = String::from_utf8_lossy(&f.out).into_owned();
            result.unwrap_or_else(|e| panic!("{e}\n{out}"));
        }
        let out = output(f);
        assert!(!out.contains("INCORRECT RESULT"), "{out}");
        assert!(!out.contains("WRONG NUMBER OF RESULTS"), "{out}");
        assert!(!out.contains("FIND returns a TRUE value"), "{out}");
        assert!(out.contains("End of additional Core tests"), "{out}");
        assert!(out.contains("End of Core Extension word tests"), "{out}");
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
