//! Double-cell words, including mixed single/double arithmetic.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "double-cell and unsigned arithmetic reinterpret the bits of a cell on purpose"
)]

use crate::*;

/// The core of M*/: d * n1 / n2, floored, through a 192-bit product so that
/// the multiplication cannot overflow.
#[allow(
    clippy::many_single_char_names,
    reason = "names of Forth stack items, as in the stack comments (a, n, u, x)"
)]
pub(crate) fn mul_div(d: i128, n1: Cell, n2: Cell) -> Res<i128> {
    if n2 == 0 {
        return Err(Throw::new(DIVISION_BY_ZERO));
    }
    let (m, n, v) =
        (d.unsigned_abs(), u128::from(n1.unsigned_abs()), u128::from(n2.unsigned_abs()));
    let lo = u128::from(m as u64) * n;
    let hi = (m >> 64) * n + (lo >> 64);
    // Long division of the product hi:lo, one 64-bit limb at a time.
    let (mut q, mut r) = ([0u64; 3], 0u128);
    for (i, limb) in [(hi >> 64) as u64, hi as u64, lo as u64].into_iter().enumerate() {
        let cur = (r << 64) | u128::from(limb);
        q[i] = (cur / v) as u64;
        r = cur % v;
    }
    let too_big = || Throw::new(OUT_OF_RANGE);
    if q[0] != 0 {
        return Err(too_big());
    }
    let q = (u128::from(q[1]) << 64) | u128::from(q[2]);
    if (d < 0) ^ (n1 < 0) ^ (n2 < 0) {
        // A negative quotient with a remainder rounds down, away from zero.
        let q = q.checked_add(u128::from(r != 0)).filter(|&q| q <= 1 << 127).ok_or_else(too_big)?;
        Ok((q as i128).wrapping_neg())
    } else {
        i128::try_from(q).map_err(|_| too_big())
    }
}

#[allow(
    clippy::many_single_char_names,
    reason = "names of Forth stack items, as in the stack comments (a, n, u, x)"
)]
pub(crate) fn install(f: &mut Forth) {
    install_bits(f);
    f.prim("S>D", |f| {
        let n = f.pop()?;
        f.push_d(i128::from(n));
        Ok(())
    });
    f.prim("M*", |f| {
        let (a, b) = f.pop2()?;
        f.push_d(i128::from(a) * i128::from(b));
        Ok(())
    });
    f.prim("UM*", |f| {
        let (a, b) = f.pop2()?;
        f.push_ud(u128::from(a as u64) * u128::from(b as u64));
        Ok(())
    });
    f.prim("UM/MOD", |f| {
        let d = u128::from(f.pop()? as u64);
        let n = f.pop_ud()?;
        if d == 0 {
            return Err(Throw::new(DIVISION_BY_ZERO));
        }
        let q = u64::try_from(n / d).map_err(|_| Throw::new(OUT_OF_RANGE))?;
        f.push((n % d) as u64 as Cell);
        f.push(q as Cell);
        Ok(())
    });
    f.prim("FM/MOD", |f| f.divide(true));
    f.prim("SM/REM", |f| f.divide(false));
    f.prim("D+", |f| f.d_binop(i128::wrapping_add));
    f.prim("D-", |f| f.d_binop(i128::wrapping_sub));
    f.prim("DMAX", |f| f.d_binop(i128::max));
    f.prim("DMIN", |f| f.d_binop(i128::min));
    f.prim("DNEGATE", |f| f.d_unop(i128::wrapping_neg));
    f.prim("DABS", |f| f.d_unop(i128::wrapping_abs));
    f.prim("D2*", |f| f.d_unop(|d| d << 1));
    f.prim("D2/", |f| f.d_unop(|d| d >> 1));
    f.prim("D<", |f| f.d_compare(|a, b| a < b));
    f.prim("D=", |f| f.d_compare(|a, b| a == b));
    f.prim("DU<", |f| f.d_compare(|a, b| (a as u128) < b as u128));
    f.prim("D0<", |f| {
        let d = f.pop_d()?;
        f.push(flag(d < 0));
        Ok(())
    });
    f.prim("D0=", |f| {
        let d = f.pop_d()?;
        f.push(flag(d == 0));
        Ok(())
    });
    f.prim("D>S", |f| {
        let d = f.pop_d()?;
        f.push(d as Cell);
        Ok(())
    });
    f.prim("M+", |f| {
        let n = f.pop()?;
        let d = f.pop_d()?;
        f.push_d(d.wrapping_add(i128::from(n)));
        Ok(())
    });
    f.prim("M*/", |f| {
        let (n1, n2) = f.pop2()?;
        let d = f.pop_d()?;
        f.push_d(mul_div(d, n1, n2)?);
        Ok(())
    });
    f.prim("2@", |f| {
        let a = f.pop()?;
        let i = f.check(a, 2 * to_cell(CELL))?;
        f.push(f.get(i + CELL));
        f.push(f.get(i));
        Ok(())
    });
    f.prim("2!", |f| {
        let a = f.pop()?;
        let (x1, x2) = f.pop2()?;
        let i = f.check(a, 2 * to_cell(CELL))?;
        f.set(i, x2);
        f.set(i + CELL, x1);
        Ok(())
    });
    f.prim("2/", |f| {
        let a = f.pop()?;
        f.push(a >> 1);
        Ok(())
    });
    f.prim("AND", |f| f.binop(|a, b| a & b));
    f.prim("OR", |f| f.binop(|a, b| a | b));
    f.prim("XOR", |f| f.binop(|a, b| a ^ b));
    f.prim("INVERT", |f| {
        let a = f.pop()?;
        f.push(!a);
        Ok(())
    });
    f.prim("=", |f| f.binop(|a, b| flag(a == b)));
    f.prim("<", |f| f.binop(|a, b| flag(a < b)));
    f.prim(">", |f| f.binop(|a, b| flag(a > b)));
    f.prim("U<", |f| f.binop(|a, b| flag((a as u64) < (b as u64))));
}

/// Gforth: rotations, arithmetic shift, trailing zeros, binary logarithm.
fn install_bits(f: &mut Forth) {
    // A shift by a cell or more (or a negative count) shifts every bit out.
    f.prim("LSHIFT", |f| f.binop(|a, b| shift(a, b, u64::checked_shl)));
    f.prim("RSHIFT", |f| f.binop(|a, b| shift(a, b, u64::checked_shr)));
    f.prim("ROL", |f| f.binop(|a, b| (a as u64).rotate_left((b as u64 % 64) as u32) as Cell));
    f.prim("ROR", |f| f.binop(|a, b| (a as u64).rotate_right((b as u64 % 64) as u32) as Cell));
    f.prim("ARSHIFT", |f| f.binop(|a, b| a >> u32::try_from(b).unwrap_or(63).min(63)));
    f.prim("CTZ", |f| {
        let x = f.pop()?;
        f.push(Cell::from(x.trailing_zeros()));
        Ok(())
    });
    f.prim("LOG2", |f| {
        let x = f.pop()?;
        f.push(if x == 0 { -1 } else { Cell::from(63 - x.leading_zeros()) });
        Ok(())
    });
}

fn shift(a: Cell, n: Cell, op: fn(u64, u32) -> Option<u64>) -> Cell {
    u32::try_from(n).ok().and_then(|n| op(a as u64, n)).unwrap_or(0) as Cell
}
