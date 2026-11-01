//! The memory map and access to Forth memory.

use crate::*;

// Memory map: system variables at the bottom, then the input buffers and
// the transient buffers, all at fixed addresses. Above them is the data
// space that HERE grows through, and the heap grows down from the top of
// memory to meet it (see heap.rs). Only the top depends on the size.
pub(crate) const BASE_ADDR: usize = 0;
pub(crate) const STATE_ADDR: usize = 8;
pub(crate) const IN_ADDR: usize = 16;
pub(crate) const BLK_ADDR: usize = 24;
pub(crate) const SCR_ADDR: usize = 32;
pub(crate) const LINE_MAX: usize = 1024; // longest input line
pub(crate) const MAX_SOURCES: usize = 16; // the terminal plus nested INCLUDEs
pub(crate) const SOURCE_AREA: usize = 64;
pub(crate) const SQ_BUFS: usize = SOURCE_AREA + MAX_SOURCES * LINE_MAX; // two alternating S" buffers
pub(crate) const WORD_BUF: usize = SQ_BUFS + 2 * LINE_MAX; // counted string, trailing space, padding
// <# #> builds its string downwards from HOLD_END, down to HOLD_START
pub(crate) const HOLD_START: usize = WORD_BUF + 264;
pub(crate) const HOLD_END: usize = HOLD_START + 256;
pub(crate) const PAD_ADDR: usize = HOLD_END;
pub(crate) const BLOCK_BUFS: usize = PAD_ADDR + 256; // BLOCK_BUFFERS buffers of a block each
pub(crate) const DICT_START: usize = BLOCK_BUFS + BLOCK_BUFFERS * BLOCK_SIZE;

/// The default size of Forth memory, and the sizes `Forth::with_memory` accepts.
pub const DEFAULT_MEMORY: usize = 1 << 20;
pub const MIN_MEMORY: usize = 64 << 10;
/// 4 GiB; 1 GiB on a 32-bit target such as WebAssembly, where 4 GiB does not
/// fit in a usize.
#[allow(
    clippy::cast_possible_truncation,
    reason = "the 4 GiB branch is only taken where usize is 64 bits wide"
)]
pub const MAX_MEMORY: usize = if usize::BITS >= 64 { (4u64 << 30) as usize } else { 1 << 30 };

/// A length, index or address as a cell. A value that does not fit saturates at
/// `Cell::MAX`; it cannot happen below `MAX_MEMORY`, so real values never change.
#[inline]
pub(crate) fn to_cell(n: usize) -> Cell {
    Cell::try_from(n).unwrap_or(Cell::MAX)
}

/// A cell as an index, length or address. A value that does not fit saturates at
/// `usize::MAX`, also a negative one: it stays outside every memory, so `check` and
/// slice bounds still refuse it, where `0` would be a valid address.
#[inline]
pub(crate) fn to_index(c: Cell) -> usize {
    usize::try_from(c).unwrap_or(usize::MAX)
}

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
        if a >= 0 && n >= 0 && end >= 0 && to_index(end) <= self.mem.len() {
            Ok(to_index(a))
        } else {
            Err(Throw::with(INVALID_ADDRESS, a.to_string()))
        }
    }

    pub(crate) fn reserve(&mut self, n: usize) -> Res<usize> {
        let a = self.here;
        if a + n > self.data_end() {
            return Err(Throw::new(DICTIONARY_OVERFLOW));
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
            self.push(to_cell(buf));
            self.push(to_cell(n));
        }
        Ok(())
    }

    /// Copies a string into data space and compiles code that pushes it.
    pub(crate) fn compile_string(&mut self, text: &[u8]) -> Res {
        let n = text.len();
        let a = self.reserve(n)?;
        self.mem[a..a + n].copy_from_slice(text);
        self.align_here();
        self.code.push(Instr::Lit(to_cell(a)));
        self.code.push(Instr::Lit(to_cell(n)));
        Ok(())
    }

    /// Pops a string ( c-addr u ) and checks it; returns its index in memory
    /// and its length. A negative length counts as zero.
    pub fn pop_str(&mut self) -> Res<(usize, usize)> {
        let (a, n) = self.pop2()?;
        let n = n.max(0);
        Ok((self.check(a, n)?, to_index(n)))
    }

    /// The end of the buffer that address a lies in: one of the system's
    /// fixed buffers, the data space (up to the heap) or an `ALLOCATE`d block.
    pub(crate) fn buffer_end(&self, a: usize) -> usize {
        let mut ends = vec![SOURCE_AREA];
        ends.extend((1..=MAX_SOURCES + 2).map(|k| SOURCE_AREA + k * LINE_MAX));
        ends.extend([HOLD_START, HOLD_END, BLOCK_BUFS]);
        ends.extend((1..=BLOCK_BUFFERS).map(|k| BLOCK_BUFS + k * BLOCK_SIZE));
        match ends.into_iter().find(|&end| a < end) {
            Some(end) => end,
            None => self.heap.region_end(a),
        }
    }

    /// A copy of the string at a with length n.
    pub(crate) fn bytes(&self, a: Cell, n: Cell) -> Res<Vec<u8>> {
        let n = n.max(0);
        let i = self.check(a, n)?;
        Ok(self.mem[i..i + to_index(n)].to_vec())
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
        self.emit_bytes(s.as_bytes());
    }

    pub(crate) fn compiling(&self) -> bool {
        self.get(STATE_ADDR) != 0
    }

    pub(crate) fn set_state(&mut self, compiling: bool) {
        self.set(STATE_ADDR, flag(compiling));
    }
}
