//! Memory words.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "c! C, and FILL store the low byte of a cell, as the standard says"
)]

use crate::*;

#[allow(
    clippy::many_single_char_names,
    reason = "names of Forth stack items, as in the stack comments (a, n, u, x)"
)]
pub(crate) fn install(f: &mut Forth) {
    f.prim("@", |f| {
        let a = f.pop()?;
        let i = f.check(a, to_cell(CELL))?;
        f.push(f.get(i));
        Ok(())
    });
    f.prim("!", |f| {
        let (v, a) = f.pop2()?;
        let i = f.check(a, to_cell(CELL))?;
        f.set(i, v);
        Ok(())
    });
    f.prim("C@", |f| {
        let a = f.pop()?;
        let i = f.check(a, 1)?;
        f.push(Cell::from(f.mem[i]));
        Ok(())
    });
    f.prim("C!", |f| {
        let (v, a) = f.pop2()?;
        let i = f.check(a, 1)?;
        f.mem[i] = v as u8;
        Ok(())
    });
    // Gforth: 32-bit and 16-bit cells of memory, little-endian, zero-extended.
    f.prim("L@", fetch_narrow::<4>);
    f.prim("W@", fetch_narrow::<2>);
    f.prim("L!", store_narrow::<4>);
    f.prim("W!", store_narrow::<2>);
    f.prim(",", |f| {
        let v = f.pop()?;
        f.comma(v)
    });
    f.prim("C,", |f| {
        let v = f.pop()?;
        let a = f.reserve(1)?;
        f.mem[a] = v as u8;
        Ok(())
    });
    f.prim("HERE", |f| {
        f.push(to_cell(f.here));
        Ok(())
    });
    f.prim("ALLOT", |f| {
        let n = f.pop()?;
        let new = to_cell(f.here) + n;
        if new < to_cell(DICT_START) || new > to_cell(f.data_end()) {
            return Err(Throw::new(DICTIONARY_OVERFLOW));
        }
        f.here = to_index(new);
        Ok(())
    });
    f.prim("UNUSED", |f| {
        f.push(to_cell(f.data_end() - f.here));
        Ok(())
    });
    f.prim("PAD", |f| {
        f.push(to_cell(PAD_ADDR));
        Ok(())
    });
    f.prim("FILL", |f| {
        let c = f.pop()?;
        let (a, n) = f.pop2()?;
        if n > 0 {
            let i = f.check(a, n)?;
            f.charge(to_index(n))?;
            f.mem[i..i + to_index(n)].fill(c as u8);
        }
        Ok(())
    });
    f.prim("MOVE", |f| {
        let n = f.pop()?;
        let (src, dst) = f.pop2()?;
        if n > 0 {
            let s = f.check(src, n)?;
            let d = f.check(dst, n)?;
            f.charge(to_index(n))?;
            // copy_within handles overlapping ranges in either direction
            f.mem.copy_within(s..s + to_index(n), d);
        }
        Ok(())
    });
    f.prim("COUNT", |f| {
        let a = f.pop()?;
        let i = f.check(a, 1)?;
        f.push(a + 1);
        f.push(Cell::from(f.mem[i]));
        Ok(())
    });
}

fn fetch_narrow<const N: usize>(f: &mut Forth) -> Res {
    let a = f.pop()?;
    let i = f.check(a, to_cell(N))?;
    let mut bytes = [0; 8];
    bytes[..N].copy_from_slice(&f.mem[i..i + N]);
    f.push(Cell::from_le_bytes(bytes));
    Ok(())
}

fn store_narrow<const N: usize>(f: &mut Forth) -> Res {
    let (v, a) = f.pop2()?;
    let i = f.check(a, to_cell(N))?;
    f.mem[i..i + N].copy_from_slice(&v.to_le_bytes()[..N]);
    Ok(())
}
