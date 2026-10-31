//! Floating-point numbers: IEEE 754 doubles on a stack of their own, the
//! conversion from text (>FLOAT and literals such as 1.5E3) and the
//! conversion to text (REPRESENT, F. FE. FS.).

use crate::*;

pub(crate) const FSTACK_FLOATS: usize = 1 << 10; // float stack size, reported by ENVIRONMENT?
pub(crate) const FLOAT: usize = 8; // the size of a float (and a DFLOAT) in memory
pub(crate) const SFLOAT: usize = 4; // the size of an SFLOAT in memory
pub(crate) const DEFAULT_PRECISION: usize = 15;
pub(crate) const MAX_PRECISION: usize = 17; // enough to tell every double apart

/// Converts text to a float, following the syntax of >FLOAT: a significand
/// with an optional exponent, whose marker is E, e, D, d or just a sign, as
/// in `-1.5E3`, `.5`, `9d-` or `1+1`. A string of blanks is zero. With
/// `literal`, it follows the stricter syntax of the text interpreter
/// instead: digits before the point, and an E or e that is always there.
pub(crate) fn to_float(text: &[u8], literal: bool) -> Option<f64> {
    if !literal && text.iter().all(|&c| c == b' ') {
        return Some(0.0);
    }
    let digits = |i: usize| text[i..].iter().take_while(|c| c.is_ascii_digit()).count();
    let mut i = 0;
    let negative = text.first() == Some(&b'-');
    if matches!(text.first(), Some(b'+' | b'-')) {
        i += 1;
    }
    let int = &text[i..i + digits(i)];
    i += int.len();
    let mut frac: &[u8] = &[];
    if text.get(i) == Some(&b'.') {
        frac = &text[i + 1..i + 1 + digits(i + 1)];
        i += 1 + frac.len();
    }
    if int.is_empty() && (literal || frac.is_empty()) {
        return None;
    }
    let mut exp_negative = false;
    match text.get(i) {
        Some(b'E' | b'e') => {
            i += 1;
            if let Some(&c @ (b'+' | b'-')) = text.get(i) {
                exp_negative = c == b'-';
                i += 1;
            }
        }
        Some(b'D' | b'd') if !literal => {
            i += 1;
            if let Some(&c @ (b'+' | b'-')) = text.get(i) {
                exp_negative = c == b'-';
                i += 1;
            }
        }
        Some(&c @ (b'+' | b'-')) if !literal => {
            exp_negative = c == b'-';
            i += 1;
        }
        None if !literal => {}
        _ => return None,
    }
    let exp = &text[i..i + digits(i)];
    if i + exp.len() != text.len() {
        return None;
    }
    // Rust's own parser rounds correctly, however many digits there are.
    // only ASCII digits get here
    let part = |d: &[u8]| {
        if d.is_empty() {
            "0".to_string()
        } else {
            String::from_utf8_lossy(d).into_owned()
        }
    };
    let s = format!(
        "{}{}.{}e{}{}",
        if negative { "-" } else { "" },
        part(int),
        part(frac),
        if exp_negative { "-" } else { "" },
        part(exp)
    );
    s.parse().ok()
}

/// The first `u` significant digits of |r|, rounded, and the decimal
/// exponent n with |r| = 0.digits × 10^n, as REPRESENT gives them; None for
/// an infinity or a NaN. Zero gives zeros and an exponent of 1.
pub(crate) fn represent(r: f64, u: usize) -> Option<(Vec<u8>, i64)> {
    if !r.is_finite() {
        return None;
    }
    let s = format!("{:.*e}", u.max(1) - 1, r.abs());
    let (mantissa, exp) = s.split_once('e').unwrap();
    let mut digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).collect();
    digits.truncate(u);
    Some((digits, exp.parse::<i64>().unwrap() + 1))
}

/// The text of an infinity or a NaN, for F. FE. and FS.
fn special(r: f64) -> String {
    match (r.is_nan(), r.is_sign_negative()) {
        (true, _) => "nan ".into(),
        (false, true) => "-inf ".into(),
        (false, false) => "inf ".into(),
    }
}

fn sign(r: f64) -> &'static str {
    if r.is_sign_negative() {
        "-"
    } else {
        ""
    }
}

/// F.: fixed-point notation with `p` significant digits, without trailing
/// zeros after the point, as in `1000.` and `0.33333`.
pub(crate) fn fixed(r: f64, p: usize) -> String {
    let Some((digits, n)) = represent(r, p) else { return special(r) };
    let (int, frac) = if n as usize >= p && n > 0 {
        // Too large for decimals: the digits, then zeros up to the point.
        (String::from_utf8(digits).unwrap() + &"0".repeat(n as usize - p), String::new())
    } else {
        // Below 1 all p digits go after the point, as in 0.00024.
        let decimals = if n <= 0 { p } else { p - n as usize };
        let s = format!("{:.*}", decimals, r.abs());
        let (int, frac) = s.split_once('.').unwrap_or((&s, ""));
        (int.to_string(), frac.to_string())
    };
    format!("{}{int}.{} ", sign(r), frac.trim_end_matches('0'))
}

/// FS. and FE.: scientific notation with `p` significant digits, as in
/// `3.3333E0`; `engineering` makes the exponent a multiple of three, as in
/// `333.33E-3`.
pub(crate) fn scientific(r: f64, p: usize, engineering: bool) -> String {
    let Some((mut digits, n)) = represent(r, p) else { return special(r) };
    let exp = if r == 0.0 { 0 } else { n - 1 };
    let shift = if engineering { exp.rem_euclid(3) as usize } else { 0 };
    while digits.len() < shift + 1 {
        digits.push(b'0');
    }
    let digits = String::from_utf8(digits).unwrap();
    let (int, frac) = digits.split_at(shift + 1);
    format!("{}{int}.{frac}E{} ", sign(r), exp - shift as i64)
}

/// FROUND: to the nearest integer, and to the even one when halfway.
pub(crate) fn round_even(r: f64) -> f64 {
    let rounded = r.round();
    if (r - r.trunc()).abs() == 0.5 {
        2.0 * (r / 2.0).round()
    } else {
        rounded
    }
}

impl Forth {
    pub(crate) fn fpush(&mut self, r: f64) -> Res {
        if self.fstack.len() >= FSTACK_FLOATS {
            return Err(Throw::new(FSTACK_OVERFLOW));
        }
        self.fstack.push(r);
        Ok(())
    }

    pub(crate) fn fpop(&mut self) -> Res<f64> {
        self.fstack.pop().ok_or(Throw::new(FSTACK_UNDERFLOW))
    }

    pub(crate) fn fpop2(&mut self) -> Res<(f64, f64)> {
        let b = self.fpop()?;
        let a = self.fpop()?;
        Ok((a, b))
    }

    /// Replaces the top float with `op` of it.
    pub(crate) fn funop(&mut self, op: impl Fn(f64) -> f64) -> Res {
        let r = self.fpop()?;
        self.fpush(op(r))
    }

    /// Replaces the top two floats with `op` of them.
    pub(crate) fn fbinop(&mut self, op: impl Fn(f64, f64) -> f64) -> Res {
        let (a, b) = self.fpop2()?;
        self.fpush(op(a, b))
    }

    pub(crate) fn fget(&self, a: usize) -> f64 {
        f64::from_le_bytes(self.mem[a..a + FLOAT].try_into().unwrap())
    }

    pub(crate) fn fset(&mut self, a: usize, r: f64) {
        self.mem[a..a + FLOAT].copy_from_slice(&r.to_le_bytes());
    }

    /// A float literal in the text interpreter, which only knows them while
    /// BASE is decimal.
    pub(crate) fn float_literal(&self, w: &str) -> Option<f64> {
        if self.get(BASE_ADDR) == 10 {
            to_float(w.as_bytes(), true)
        } else {
            None
        }
    }
}
