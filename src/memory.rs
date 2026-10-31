//! The memory map and access to Forth memory.

use crate::*;

// Memory map: system variables at the bottom, then the data space that
// HERE grows through, the heap growing down from DATA_END to meet it (see
// heap.rs), then transient buffers and the input buffers on top.
pub(crate) const MEM_SIZE: usize = 1 << 20;
pub(crate) const BASE_ADDR: usize = 0;
pub(crate) const STATE_ADDR: usize = 8;
pub(crate) const IN_ADDR: usize = 16;
pub(crate) const DICT_START: usize = 64;
pub(crate) const LINE_MAX: usize = 1024; // longest input line
pub(crate) const MAX_SOURCES: usize = 16; // the terminal plus nested INCLUDEs
pub(crate) const SOURCE_AREA: usize = MEM_SIZE - MAX_SOURCES * LINE_MAX;
pub(crate) const SQ_BUFS: usize = SOURCE_AREA - 2 * LINE_MAX; // two alternating S" buffers
pub(crate) const WORD_BUF: usize = SQ_BUFS - 264; // counted string, trailing space, padding
pub(crate) const PAD_ADDR: usize = WORD_BUF - 256;
// <# #> builds its string downwards from here, down to DATA_END
pub(crate) const HOLD_END: usize = PAD_ADDR;
pub(crate) const DATA_END: usize = PAD_ADDR - 256;

impl Forth {
    /// Reads a cell at an address the system itself controls.
    pub(crate) fn get(&self, a: usize) -> Cell {
        Cell::from_le_bytes(self.mem[a..a + CELL].try_into().unwrap())
    }

    pub(crate) fn set(&mut self, a: usize, v: Cell) {
        self.mem[a..a + CELL].copy_from_slice(&v.to_le_bytes());
    }

    /// Validates a program-supplied address range of `n` bytes.
    pub(crate) fn check(&self, a: Cell, n: Cell) -> Res<usize> {
        let end = a.checked_add(n).unwrap_or(-1);
        if a >= 0 && n >= 0 && end >= 0 && end as usize <= MEM_SIZE {
            Ok(a as usize)
        } else {
            Err(Throw::with(-9, a.to_string()))
        }
    }

    pub(crate) fn reserve(&mut self, n: usize) -> Res<usize> {
        let a = self.here;
        if a + n > self.data_end() {
            return Err(Throw::new(-8));
        }
        self.here += n;
        Ok(a)
    }

    pub(crate) fn comma(&mut self, v: Cell) -> Res {
        let a = self.reserve(CELL)?;
        self.set(a, v);
        Ok(())
    }

    pub(crate) fn align_here(&mut self) {
        self.here = (self.here + CELL - 1) & !(CELL - 1);
    }

    /// The string of S" and S\": compiled into data space, or, when
    /// interpreting, put in a transient buffer so HERE is left alone.
    pub(crate) fn string_literal(&mut self, text: &[u8]) -> Res {
        let n = text.len();
        if self.compiling() {
            self.compile_string(text)?;
        } else {
            let buf = SQ_BUFS + self.sq_next * LINE_MAX;
            self.sq_next = 1 - self.sq_next;
            self.mem[buf..buf + n].copy_from_slice(text);
            self.push(buf as Cell);
            self.push(n as Cell);
        }
        Ok(())
    }

    /// Copies a string into data space and compiles code that pushes it.
    pub(crate) fn compile_string(&mut self, text: &[u8]) -> Res {
        let n = text.len();
        let a = self.reserve(n)?;
        self.mem[a..a + n].copy_from_slice(text);
        self.align_here();
        self.code.push(Instr::Lit(a as Cell));
        self.code.push(Instr::Lit(n as Cell));
        Ok(())
    }

    /// A copy of the string at a with length n.
    pub(crate) fn bytes(&self, a: Cell, n: Cell) -> Res<Vec<u8>> {
        let n = n.max(0);
        let i = self.check(a, n)?;
        Ok(self.mem[i..i + n as usize].to_vec())
    }

    /// SUBSTITUTE's text replacement: each %name% of a substitution made by
    /// REPLACES becomes its text, %% becomes %, and everything else is kept.
    /// Returns the result and the number of substitutions.
    pub(crate) fn substitute(&self, text: &[u8]) -> (Vec<u8>, Cell) {
        let (mut out, mut count, mut i) = (Vec::new(), 0, 0);
        while i < text.len() {
            if text[i] != b'%' {
                out.push(text[i]);
                i += 1;
            } else if text.get(i + 1) == Some(&b'%') {
                out.push(b'%');
                i += 2;
            } else if let Some(len) = text[i + 1..].iter().position(|&c| c == b'%') {
                let name = &text[i + 1..i + 1 + len];
                let key = String::from_utf8_lossy(name).to_ascii_uppercase();
                match self.substitutions.get(&key) {
                    Some(sub) => {
                        out.extend_from_slice(sub);
                        count += 1;
                    }
                    None => out.extend_from_slice(&text[i..i + len + 2]),
                }
                i += len + 2;
            } else {
                out.extend_from_slice(&text[i..]); // a % without a closing %
                break;
            }
        }
        (out, count)
    }

    pub(crate) fn text(&self, a: usize, n: usize) -> String {
        String::from_utf8_lossy(&self.mem[a..a + n]).into_owned()
    }

    pub(crate) fn emit_str(&mut self, s: &str) {
        self.out.extend_from_slice(s.as_bytes());
    }

    pub(crate) fn compiling(&self) -> bool {
        self.get(STATE_ADDR) != 0
    }

    pub(crate) fn set_state(&mut self, compiling: bool) {
        self.set(STATE_ADDR, flag(compiling));
    }
}
