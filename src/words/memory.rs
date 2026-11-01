//! Memory words.

use crate::*;

#[allow(
    clippy::many_single_char_names,
    reason = "names of Forth stack items, as in the stack comments (a, n, u, x)"
)]
pub(crate) fn install(f: &mut Forth) {
    f.prim("@", |f| {
        let a = f.pop()?;
        let i = f.check(a, CELL as Cell)?;
        f.push(f.get(i));
        Ok(())
    });
    f.prim("!", |f| {
        let (v, a) = f.pop2()?;
        let i = f.check(a, CELL as Cell)?;
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
        f.push(f.here as Cell);
        Ok(())
    });
    f.prim("ALLOT", |f| {
        let n = f.pop()?;
        let new = f.here as Cell + n;
        if new < DICT_START as Cell || new > f.data_end() as Cell {
            return Err(Throw::new(DICTIONARY_OVERFLOW));
        }
        f.here = new as usize;
        Ok(())
    });
    f.prim("UNUSED", |f| {
        f.push((f.data_end() - f.here) as Cell);
        Ok(())
    });
    f.prim("PAD", |f| {
        f.push(PAD_ADDR as Cell);
        Ok(())
    });
    f.prim("FILL", |f| {
        let c = f.pop()?;
        let (a, n) = f.pop2()?;
        if n > 0 {
            let i = f.check(a, n)?;
            f.charge(n as usize)?;
            f.mem[i..i + n as usize].fill(c as u8);
        }
        Ok(())
    });
    f.prim("MOVE", |f| {
        let n = f.pop()?;
        let (src, dst) = f.pop2()?;
        if n > 0 {
            let s = f.check(src, n)?;
            let d = f.check(dst, n)?;
            f.charge(n as usize)?;
            // copy_within handles overlapping ranges in either direction
            f.mem.copy_within(s..s + n as usize, d);
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
