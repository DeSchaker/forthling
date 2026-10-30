//! Double-cell words, including mixed single/double arithmetic.

use crate::*;

/// The core of M*/: d * n1 / n2, floored, through a 192-bit product so that
/// the multiplication cannot overflow.
pub(crate) fn mul_div(d: i128, n1: Cell, n2: Cell) -> Res<i128> {
    if n2 == 0 {
        return Err(Throw::new(-10));
    }
    let (m, n, v) = (d.unsigned_abs(), n1.unsigned_abs() as u128, n2.unsigned_abs() as u128);
    let lo = (m as u64 as u128) * n;
    let hi = (m >> 64) * n + (lo >> 64);
    // Long division of the product hi:lo, one 64-bit limb at a time.
    let (mut q, mut r) = ([0u64; 3], 0u128);
    for (i, limb) in [(hi >> 64) as u64, hi as u64, lo as u64].into_iter().enumerate() {
        let cur = r << 64 | limb as u128;
        q[i] = (cur / v) as u64;
        r = cur % v;
    }
    let too_big = || Throw::new(-11);
    if q[0] != 0 {
        return Err(too_big());
    }
    let q = (q[1] as u128) << 64 | q[2] as u128;
    if (d < 0) ^ (n1 < 0) ^ (n2 < 0) {
        // A negative quotient with a remainder rounds down, away from zero.
        let q = q.checked_add((r != 0) as u128).filter(|&q| q <= 1 << 127).ok_or_else(too_big)?;
        Ok((q as i128).wrapping_neg())
    } else {
        i128::try_from(q).map_err(|_| too_big())
    }
}

pub(crate) fn install(f: &mut Forth) {
    f.prim("S>D", |f| {
        let n = f.pop()?;
        f.push_d(n as i128);
        Ok(())
    });
    f.prim("M*", |f| {
        let (a, b) = f.pop2()?;
        f.push_d(a as i128 * b as i128);
        Ok(())
    });
    f.prim("UM*", |f| {
        let (a, b) = f.pop2()?;
        f.push_ud(a as u64 as u128 * b as u64 as u128);
        Ok(())
    });
    f.prim("UM/MOD", |f| {
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
    f.prim("FM/MOD", |f| f.divide(true));
    f.prim("SM/REM", |f| f.divide(false));
    f.prim("D+", |f| f.d_binop(|a, b| a.wrapping_add(b)));
    f.prim("D-", |f| f.d_binop(|a, b| a.wrapping_sub(b)));
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
        f.push_d(d.wrapping_add(n as i128));
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
        let i = f.check(a, 2 * CELL as Cell)?;
        f.push(f.get(i + CELL));
        f.push(f.get(i));
        Ok(())
    });
    f.prim("2!", |f| {
        let a = f.pop()?;
        let (x1, x2) = f.pop2()?;
        let i = f.check(a, 2 * CELL as Cell)?;
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
    f.prim("LSHIFT", |f| f.binop(|a, b| ((a as u64) << (b & 63)) as Cell));
    f.prim("RSHIFT", |f| f.binop(|a, b| ((a as u64) >> (b & 63)) as Cell));
    f.prim("=", |f| f.binop(|a, b| flag(a == b)));
    f.prim("<", |f| f.binop(|a, b| flag(a < b)));
    f.prim(">", |f| f.binop(|a, b| flag(a > b)));
    f.prim("U<", |f| f.binop(|a, b| flag((a as u64) < (b as u64))));
}
