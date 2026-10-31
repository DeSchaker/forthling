//! UTF-8, the encoding of the extended characters (xchars). Malformed data
//! is read one byte at a time: a byte that does not start a complete xchar
//! is an xchar of its own, with the byte as its value.

use crate::*;

/// The largest xchar (MAX-XCHAR) and the most bytes one takes (XCHAR-MAXMEM).
pub(crate) const MAX_XCHAR: u32 = 0x10FFFF;
pub(crate) const XCHAR_MAXMEM: usize = 4;

/// The UTF-8 bytes of an xchar. A value above MAX-XCHAR becomes U+FFFD, the
/// replacement character.
pub(crate) fn encode(xc: Cell) -> Vec<u8> {
    let c = if (0..=MAX_XCHAR as Cell).contains(&xc) { xc as u32 } else { 0xFFFD };
    match c {
        0..=0x7F => vec![c as u8],
        0x80..=0x7FF => vec![0xC0 | (c >> 6) as u8, cont(c)],
        0x800..=0xFFFF => vec![0xE0 | (c >> 12) as u8, cont(c >> 6), cont(c)],
        _ => vec![0xF0 | (c >> 18) as u8, cont(c >> 12), cont(c >> 6), cont(c)],
    }
}

fn cont(c: u32) -> u8 {
    0x80 | (c & 0x3F) as u8
}

pub(crate) fn is_cont(b: u8) -> bool {
    b & 0xC0 == 0x80
}

/// How many bytes an xchar starting with `b` takes; 1 for a byte that
/// cannot start one.
pub(crate) fn lead_size(b: u8) -> usize {
    match b {
        0xC2..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF4 => 4,
        _ => 1,
    }
}

/// The first xchar of `bytes` (which must not be empty) and its size.
pub(crate) fn decode(bytes: &[u8]) -> (Cell, usize) {
    let lead = bytes[0];
    let size = lead_size(lead);
    if size == 1 || bytes.len() < size || !bytes[1..size].iter().all(|&b| is_cont(b)) {
        return (lead as Cell, 1);
    }
    let first = (lead & (0x7F >> size)) as u32;
    let c = bytes[1..size].iter().fold(first, |c, &b| (c << 6) | (b & 0x3F) as u32);
    (c as Cell, size)
}

/// Where the last xchar of `bytes` starts: back over at most three
/// continuation bytes to the byte that starts an xchar of exactly that size,
/// else the last byte on its own.
pub(crate) fn last_start(bytes: &[u8]) -> usize {
    let n = bytes.len();
    let mut j = n.saturating_sub(1);
    while j > 0 && n - j < XCHAR_MAXMEM && is_cont(bytes[j]) {
        j -= 1;
    }
    if decode(&bytes[j..]).1 == n - j { j } else { n.saturating_sub(1) }
}

/// The number of terminal columns an xchar takes: 0 for control characters
/// and combining marks, 2 for wide East Asian characters and emoji, else 1.
/// This follows the common ranges of Unicode, not the full tables.
pub(crate) fn width(xc: Cell) -> Cell {
    const ZERO: &[(Cell, Cell)] = &[
        (0x0000, 0x001F),
        (0x007F, 0x009F),
        (0x0300, 0x036F),
        (0x0483, 0x0489),
        (0x0591, 0x05BD),
        (0x0610, 0x061A),
        (0x064B, 0x065F),
        (0x1AB0, 0x1AFF),
        (0x1DC0, 0x1DFF),
        (0x200B, 0x200F),
        (0x202A, 0x202E),
        (0x2060, 0x2064),
        (0x20D0, 0x20FF),
        (0xFE00, 0xFE0F),
        (0xFE20, 0xFE2F),
        (0xFEFF, 0xFEFF),
        (0xE0100, 0xE01EF),
    ];
    const WIDE: &[(Cell, Cell)] = &[
        (0x1100, 0x115F),
        (0x231A, 0x231B),
        (0x2E80, 0x303E),
        (0x3041, 0x33FF),
        (0x3400, 0x4DBF),
        (0x4E00, 0x9FFF),
        (0xA000, 0xA4CF),
        (0xAC00, 0xD7A3),
        (0xF900, 0xFAFF),
        (0xFE30, 0xFE4F),
        (0xFF00, 0xFF60),
        (0xFFE0, 0xFFE6),
        (0x1F300, 0x1F64F),
        (0x1F900, 0x1F9FF),
        (0x20000, 0x2FFFD),
        (0x30000, 0x3FFFD),
    ];
    let within = |ranges: &[(Cell, Cell)]| ranges.iter().any(|&(lo, hi)| (lo..=hi).contains(&xc));
    if within(ZERO) {
        0
    } else if within(WIDE) {
        2
    } else {
        1
    }
}

impl Forth {
    /// The name of the encoding (XCHAR-ENCODING), copied to the heap the
    /// first time it is asked for.
    pub(crate) fn encoding_name(&mut self) -> Res<(usize, usize)> {
        const NAME: &[u8] = b"UTF-8";
        if self.encoding_name.is_none() {
            let a =
                self.heap.allocate(NAME.len(), self.here).ok_or(Throw::new(DICTIONARY_OVERFLOW))?;
            self.mem[a..a + NAME.len()].copy_from_slice(NAME);
            self.encoding_name = Some(a);
        }
        Ok((self.encoding_name.unwrap(), NAME.len()))
    }

    /// The bytes from `a` on, at most `n` and no further than the end of memory.
    pub(crate) fn xbytes(&self, a: Cell, n: usize) -> Res<&[u8]> {
        let i = self.check(a, 1)?;
        Ok(&self.mem[i..(i + n).min(self.mem.len())])
    }

    /// Reads one xchar from the terminal; None at end of input.
    pub(crate) fn read_xkey(&mut self) -> Option<Cell> {
        let mut bytes = vec![self.read_key()?];
        while bytes.len() < lead_size(bytes[0]) {
            match self.input.fill_buf().ok().and_then(|b| b.first().copied()) {
                Some(b) if is_cont(b) => {
                    self.input.consume(1);
                    bytes.push(b);
                }
                _ => break,
            }
        }
        Some(decode(&bytes).0)
    }

    /// Parses up to the bytes of an xchar delimiter, like PARSE.
    pub(crate) fn parse_seq(&mut self, delim: &[u8]) -> (usize, usize) {
        let (start, end) = self.parse_area();
        let text = &self.mem[start..end];
        match text.windows(delim.len()).position(|w| w == delim) {
            Some(i) => {
                self.set_in(start + i + delim.len());
                (start, i)
            }
            None => {
                self.set_in(end);
                (start, end - start)
            }
        }
    }
}
