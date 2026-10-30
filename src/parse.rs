//! Parsing the input buffer, numbers and pictured numeric output.

use crate::*;

/// The core of >NUMBER: accumulates digits in `base` into `ud` and stops at
/// the first character that is not a digit. Returns the new `ud` and the
/// number of characters used.
pub(crate) fn to_number(mut ud: u128, digits: &[u8], base: u32) -> (u128, usize) {
    let mut used = 0;
    for &c in digits {
        let Some(d) = (c as char).to_digit(base) else { break };
        ud = ud.wrapping_mul(base as u128).wrapping_add(d as u128);
        used += 1;
    }
    (ud, used)
}

pub(crate) fn fmt_unsigned(mut v: u64, base: u64) -> String {
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
    /// The unparsed rest of the current input buffer, as absolute addresses.
    pub(crate) fn parse_area(&self) -> (usize, usize) {
        let s = self.sources.last().unwrap();
        let i = self.get(IN_ADDR).clamp(0, s.len as Cell) as usize;
        (s.buf + i, s.buf + s.len)
    }

    pub(crate) fn set_in(&mut self, at: usize) {
        let buf = self.sources.last().unwrap().buf;
        self.set(IN_ADDR, (at - buf) as Cell);
    }

    /// Parses up to a delimiter, optionally skipping leading delimiters, and
    /// moves >IN past the delimiter. Returns the text's address and length.
    pub(crate) fn scan(&mut self, skip: bool, is_delim: impl Fn(u8) -> bool) -> (usize, usize) {
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

    pub(crate) fn parse_name(&mut self) -> (usize, usize) {
        self.scan(true, |c| c <= b' ')
    }

    pub(crate) fn parse(&mut self, delim: u8) -> (usize, usize) {
        self.scan(false, |c| c == delim)
    }

    /// Parses the text of S\" up to an unescaped `"` and translates its
    /// escapes, such as \n, \" and \x41.
    pub(crate) fn parse_escaped(&mut self) -> Vec<u8> {
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
                        let Some(d) =
                            self.mem[i..end].first().and_then(|&h| (h as char).to_digit(16))
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

    pub(crate) fn word(&mut self) -> Option<String> {
        let (a, n) = self.parse_name();
        (n > 0).then(|| self.text(a, n))
    }

    pub(crate) fn name(&mut self) -> Res<String> {
        self.word().ok_or(Throw::new(-16))
    }

    /// Converts a word to a number, if it is one. A trailing `.` makes it a
    /// double cell, which the flag tells.
    pub(crate) fn number(&self, w: &str) -> Option<(u128, bool)> {
        let b = w.as_bytes();
        if b.len() == 3 && b[0] == b'\'' && b[2] == b'\'' {
            return Some((b[1] as u128, false));
        }
        let (w, double) = match w.strip_suffix('.') {
            Some(w) => (w, true),
            None => (w, false),
        };
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
        if !double && ud > u64::MAX as u128 {
            return None;
        }
        Some((if neg { ud.wrapping_neg() } else { ud }, double))
    }

    /// BASE for number output, with decimal as the fallback for an invalid BASE.
    pub(crate) fn base(&self) -> u64 {
        let base = self.get(BASE_ADDR);
        if (2..=36).contains(&base) {
            base as u64
        } else {
            10
        }
    }

    /// Adds a character in front of the pictured numeric output string.
    pub(crate) fn hold(&mut self, c: u8) -> Res {
        if self.hld <= DATA_END {
            return Err(Throw::new(-17));
        }
        self.hld -= 1;
        self.mem[self.hld] = c;
        Ok(())
    }

    pub(crate) fn fmt_num(&self, n: Cell) -> String {
        let s = fmt_unsigned(n.unsigned_abs(), self.base());
        if n < 0 {
            format!("-{s}")
        } else {
            s
        }
    }
}
