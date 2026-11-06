//! Fixed-point numbers: the word list FIXED, installed as the extension `Fixed`.
//!
//! A fixed-point number is one cell: the value times a scale. The scale is a power of
//! ten (`FX-DECIMALS`, up to 18; 9 at the start) or of two (`FX-BITS`, up to 32
//! fraction bits, so Q32.32 at most). With decimals, `0.10fx 0.20fx +` is exactly
//! `0.30fx`. A literal that the scale cannot hold exactly is an error, unless
//! `TRUE FX-ROUND-LITERALS` says it may be rounded (as fractals need in binary).
//! `FX*` and `FX/` calculate in 128 bits and round as `FX-FLOOR`, `FX-TRUNC` or
//! `FX-NEAREST` say; a result that does not fit a cell throws -11, never wraps.
//! The notation `1.5fx` is a literal while FIXED is in the search order.

use crate::*;

/// The extension: `forth.install(&Fixed)`; ENVIRONMENT? knows it as FORTHLING-FIXED.
pub struct Fixed;

const MAX_BITS: u32 = 32;
const MAX_DECIMALS: u32 = 18;
/// Digits of the integer part of a literal that are looked at; more is out of range.
const MAX_INT_DIGITS: usize = 19;
/// Fraction digits that are looked at for a binary scale; the rest only decides a tie.
const MAX_FRAC_DIGITS: usize = 27;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Rounding {
    Floor,
    Trunc,
    Nearest,
}

/// The scale and the rounding of the system.
#[derive(Clone, Copy)]
struct FixedState {
    /// The word list FIXED, or 0 before the extension is installed.
    wid: Cell,
    decimal: bool,
    /// The number of fraction bits, or of decimals.
    digits: u32,
    /// The value of 1.0.
    scale: i128,
    rounding: Rounding,
    /// A literal that does not fit the scale exactly is rounded instead of refused.
    round_literals: bool,
}

/// The decimals at the start: exact for money and percentages, still a wide range.
const START_DECIMALS: u32 = 9;

impl Default for FixedState {
    fn default() -> Self {
        FixedState {
            wid: 0,
            decimal: true,
            digits: START_DECIMALS,
            scale: 10i128.pow(START_DECIMALS),
            rounding: Rounding::Nearest,
            round_literals: false,
        }
    }
}

impl FixedState {
    /// The number of places `FX.` prints: all decimals, or for a binary scale enough
    /// digits to tell two neighbours apart.
    fn places(&self) -> u32 {
        if self.decimal { self.digits } else { (self.digits * 30103).div_ceil(100_000) }
    }
}

/// Why a text is no fixed-point number.
enum Bad {
    /// It is not made of digits with at most one point.
    Syntax,
    /// The scale cannot hold it exactly, and rounding is not allowed.
    Inexact,
    /// It does not fit a cell.
    Range,
}

/// `num / den`, rounded; den is not zero.
fn rdiv(mut num: i128, mut den: i128, mode: Rounding) -> i128 {
    if den < 0 {
        num = -num;
        den = -den;
    }
    let q = num.div_euclid(den);
    let r = num.rem_euclid(den);
    match mode {
        Rounding::Floor => q,
        Rounding::Trunc => q + i128::from(num < 0 && r != 0),
        // Half away from zero.
        Rounding::Nearest => q + i128::from(2 * r > den || (2 * r == den && num >= 0)),
    }
}

fn to_fx(v: i128) -> Res<Cell> {
    Cell::try_from(v).map_err(|_| Throw::new(OUT_OF_RANGE))
}

/// Reads `[+-]digits[.digits]` (or `.digits`) as a fixed-point number.
fn parse(text: &[u8], st: &FixedState) -> Result<Cell, Bad> {
    let (neg, rest) = match text.first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    let (int, frac) = match rest.iter().position(|&c| c == b'.') {
        Some(i) => (&rest[..i], &rest[i + 1..]),
        None => (rest, &rest[..0]),
    };
    if int.len() + frac.len() == 0 || !int.iter().chain(frac).all(u8::is_ascii_digit) {
        return Err(Bad::Syntax);
    }
    let int = &int[int.iter().position(|&c| c != b'0').unwrap_or(int.len())..];
    if int.len() > MAX_INT_DIGITS {
        return Err(Bad::Range);
    }
    // Trailing zeros say nothing.
    let frac = &frac[..frac.iter().rposition(|&c| c != b'0').map_or(0, |i| i + 1)];
    // Digits far beyond the scale can only decide a tie: keep a 1 as their trace.
    // (A decimal scale needs one digit past its decimals; more would overflow.)
    let keep = if st.decimal { st.digits as usize + 1 } else { MAX_FRAC_DIGITS };
    let mut digits: Vec<u8> = frac.iter().take(keep).copied().collect();
    if frac.len() > keep {
        digits.push(b'1');
    }
    let k = u32::try_from(digits.len()).map_err(|_| Bad::Range)?;
    let number = |d: &[u8]| d.iter().fold(0i128, |n, &c| n * 10 + i128::from(c - b'0'));
    let sign = if neg { -1 } else { 1 };
    let whole = number(int) * st.scale;
    let scaled = number(&digits) * st.scale;
    if !st.round_literals && scaled % 10i128.pow(k) != 0 {
        return Err(Bad::Inexact);
    }
    let part = rdiv(sign * scaled, 10i128.pow(k), st.rounding);
    Cell::try_from(sign * whole + part).map_err(|_| Bad::Range)
}

/// The text of a fixed-point number: all places, with a point.
fn format(v: Cell, st: &FixedState) -> String {
    let places = st.places();
    let ten = 10i128.pow(places);
    let mag = rdiv(i128::from(v).abs() * ten, st.scale, Rounding::Nearest);
    let sign = if v < 0 && mag != 0 { "-" } else { "" };
    if places == 0 {
        return format!("{sign}{mag}");
    }
    format!("{sign}{}.{:0width$}", mag / ten, mag % ten, width = places as usize)
}

impl Forth {
    /// The value of a literal such as `1.5fx`, if `w` is one: None when it is not
    /// shaped like one or FIXED is not in the search order.
    pub(crate) fn fixed_literal(&self, w: &str) -> Option<Res<Cell>> {
        let b = w.as_bytes();
        let (text, suffix) = b.split_at(b.len().checked_sub(2)?);
        if !suffix.eq_ignore_ascii_case(b"fx") {
            return None;
        }
        let st = self.peek_state::<FixedState>()?;
        if !usize::try_from(st.wid).is_ok_and(|w| w != 0 && self.order.contains(&w)) {
            return None;
        }
        match parse(text, st) {
            Ok(v) => Some(Ok(v)),
            Err(Bad::Syntax) => None,
            Err(Bad::Inexact) => {
                let scale = if st.decimal { "FX-DECIMALS" } else { "FX-BITS" };
                Some(Err(Throw::with(
                    INVALID_ARGUMENT,
                    format!(
                        "{w} is not exact at {scale} {}; TRUE FX-ROUND-LITERALS allows rounding",
                        st.digits
                    ),
                )))
            }
            Err(Bad::Range) => Some(Err(Throw::with(OUT_OF_RANGE, w))),
        }
    }
}

fn state(f: &mut Forth) -> FixedState {
    *f.state::<FixedState>()
}

fn set_scale(f: &mut Forth, decimal: bool) -> Res {
    let n = f.pop()?;
    let max = if decimal { MAX_DECIMALS } else { MAX_BITS };
    let digits = u32::try_from(n).ok().filter(|&n| n <= max).ok_or_else(|| {
        let what = if decimal { "FX-DECIMALS" } else { "FX-BITS" };
        Throw::with(INVALID_ARGUMENT, format!("{what} takes 0 to {max}"))
    })?;
    let st = f.state::<FixedState>();
    st.decimal = decimal;
    st.digits = digits;
    st.scale = if decimal { 10i128.pow(digits) } else { 1 << digits };
    Ok(())
}

fn set_rounding(f: &mut Forth, rounding: Rounding) {
    f.state::<FixedState>().rounding = rounding;
}

fn add(f: &mut Forth, sign: i64) -> Res {
    let b = f.pop()?;
    let a = f.pop()?;
    let sum = if sign > 0 { a.checked_add(b) } else { a.checked_sub(b) };
    f.push(sum.ok_or(Throw::new(OUT_OF_RANGE))?);
    Ok(())
}

fn multiply(f: &mut Forth) -> Res {
    let st = state(f);
    let b = f.pop()?;
    let a = f.pop()?;
    f.push(to_fx(rdiv(i128::from(a) * i128::from(b), st.scale, st.rounding))?);
    Ok(())
}

fn divide(f: &mut Forth) -> Res {
    let st = state(f);
    let b = f.pop()?;
    let a = f.pop()?;
    if b == 0 {
        return Err(Throw::new(DIVISION_BY_ZERO));
    }
    f.push(to_fx(rdiv(i128::from(a) * st.scale, i128::from(b), st.rounding))?);
    Ok(())
}

/// The root of the number, rounded as `FX*` does (a root is never negative, so
/// `FX-FLOOR` and `FX-TRUNC` both round down).
fn square_root(f: &mut Forth) -> Res {
    let st = state(f);
    let x = f.pop()?;
    if x < 0 {
        return Err(Throw::with(INVALID_ARGUMENT, "FXSQRT of a negative number"));
    }
    let n = u128::try_from(i128::from(x) * st.scale).map_err(|_| Throw::new(OUT_OF_RANGE))?;
    let mut r = n.isqrt();
    // r is rounded down; the next number is nearer when n is beyond r + 1/2.
    if st.rounding == Rounding::Nearest && n - r * r > r {
        r += 1;
    }
    f.push(Cell::try_from(r).map_err(|_| Throw::new(OUT_OF_RANGE))?);
    Ok(())
}

/// The scale as a float; a power of two or of ten up to 10^18 is exact.
#[allow(clippy::cast_precision_loss, reason = "the scales are exact floats")]
fn scale_f64(st: &FixedState) -> f64 {
    st.scale as f64
}

#[allow(clippy::cast_possible_truncation, reason = "the range is checked just before")]
fn from_float(f: &mut Forth) -> Res {
    let st = state(f);
    let r = f.fpop()? * scale_f64(&st);
    let r = match st.rounding {
        Rounding::Floor => r.floor(),
        Rounding::Trunc => r.trunc(),
        Rounding::Nearest => r.round(),
    };
    // 2^63 is the first float above the cells.
    if !(-9_223_372_036_854_775_808.0..9_223_372_036_854_775_808.0).contains(&r) {
        return Err(Throw::new(OUT_OF_RANGE));
    }
    f.push(r as Cell);
    Ok(())
}

#[allow(clippy::cast_precision_loss, reason = "a float holds 53 bits; that is its precision")]
fn to_float(f: &mut Forth) -> Res {
    let st = state(f);
    let v = f.pop()?;
    f.fpush(v as f64 / scale_f64(&st))
}

fn parse_word(f: &mut Forth) -> Res {
    let st = state(f);
    let (a, n) = f.pop_str()?;
    if let Ok(v) = parse(&f.mem[a..a + n], &st) {
        f.push(v);
        f.push(-1);
    } else {
        f.push(0);
        f.push(0);
    }
    Ok(())
}

fn to_string(f: &mut Forth) -> Res {
    let st = state(f);
    let v = f.pop()?;
    f.hld = HOLD_END;
    for &c in format(v, &st).as_bytes().iter().rev() {
        f.hold(c)?;
    }
    f.push(to_cell(f.hld));
    f.push(to_cell(HOLD_END - f.hld));
    Ok(())
}

impl Extension for Fixed {
    fn name(&self) -> &'static str {
        "FORTHLING-FIXED"
    }

    fn install(&self, f: &mut Forth) {
        let wid = f.wordlist("FIXED");
        f.state::<FixedState>().wid = wid;
        let mut add_word = |name: &str, word: fn(&mut Forth) -> Res| {
            f.add_word(wid, name, word).expect("FIXED is a word list");
        };
        add_word("FX-BITS", |f| set_scale(f, false));
        add_word("FX-DECIMALS", |f| set_scale(f, true));
        add_word("FX-ROUND-LITERALS", |f| {
            let flag = f.pop()? != 0;
            f.state::<FixedState>().round_literals = flag;
            Ok(())
        });
        add_word("FX-SCALE", |f| {
            let scale = state(f).scale;
            f.push(to_fx(scale)?);
            Ok(())
        });
        add_word("FX-FLOOR", |f| {
            set_rounding(f, Rounding::Floor);
            Ok(())
        });
        add_word("FX-TRUNC", |f| {
            set_rounding(f, Rounding::Trunc);
            Ok(())
        });
        add_word("FX-NEAREST", |f| {
            set_rounding(f, Rounding::Nearest);
            Ok(())
        });
        add_word("FX+", |f| add(f, 1));
        add_word("FX-", |f| add(f, -1));
        add_word("FX*", multiply);
        add_word("FX/", divide);
        add_word("FXNEGATE", |f| {
            let a = f.pop()?;
            f.push(a.checked_neg().ok_or(Throw::new(OUT_OF_RANGE))?);
            Ok(())
        });
        add_word("FXABS", |f| {
            let a = f.pop()?;
            f.push(a.checked_abs().ok_or(Throw::new(OUT_OF_RANGE))?);
            Ok(())
        });
        add_word("FXSQRT", square_root);
        add_word("S>FX", |f| {
            let scale = state(f).scale;
            let n = f.pop()?;
            f.push(to_fx(i128::from(n) * scale)?);
            Ok(())
        });
        add_word("FX>S", |f| {
            let st = state(f);
            let v = f.pop()?;
            f.push(to_fx(rdiv(i128::from(v), st.scale, st.rounding))?);
            Ok(())
        });
        add_word("F>FX", from_float);
        add_word("FX>F", to_float);
        add_word(">FX", parse_word);
        add_word("FX>STRING", to_string);
        add_word("FX.", |f| {
            let st = state(f);
            let v = f.pop()?;
            f.emit_str(&format!("{} ", format(v, &st)));
            Ok(())
        });
    }
}
