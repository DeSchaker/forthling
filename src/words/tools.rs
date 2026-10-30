//! Programming tools.

use crate::*;

pub(crate) fn install(f: &mut Forth) {
    f.imm("[IF]", |f| match f.pop()? {
        0 => f.skip_conditional(true),
        _ => Ok(()),
    });
    f.imm("[ELSE]", |f| f.skip_conditional(false));
    f.imm("[THEN]", |_| Ok(()));
    f.imm("[DEFINED]", |f| {
        let name = f.name()?;
        f.push(flag(f.find(&name).is_some()));
        Ok(())
    });
    f.imm("[UNDEFINED]", |f| {
        let name = f.name()?;
        f.push(flag(f.find(&name).is_none()));
        Ok(())
    });
    // N>R moves n cells and then n itself to the return stack; NR> undoes it.
    f.prim("N>R", |f| {
        let n = f.pop()?;
        let len = f.stack.len();
        let n = usize::try_from(n).ok().filter(|&n| n <= len).ok_or(Throw::new(-4))?;
        if f.rstack.len() + n >= RSTACK_CELLS {
            return Err(Throw::new(-5));
        }
        f.rstack.extend(f.stack.drain(len - n..));
        f.rstack.push(n as Cell);
        Ok(())
    });
    f.prim("NR>", |f| {
        let n = f.rstack.pop().ok_or(Throw::new(-6))?;
        let len = f.rstack.len();
        let n = usize::try_from(n).ok().filter(|&n| n <= len).ok_or(Throw::new(-6))?;
        f.stack.extend(f.rstack.drain(len - n..));
        f.push(n as Cell);
        Ok(())
    });
    // A synonym is a copy of the word: the same behaviour and immediacy.
    f.prim("SYNONYM", |f| {
        let name = f.name()?;
        let old = f.name()?;
        let xt = f.find_or_err(&old)?;
        let (kind, immediate) = (f.dict[xt].kind.clone(), f.dict[xt].immediate);
        let new = f.define(&name, kind);
        f.dict[new].immediate = immediate;
        Ok(())
    });
    f.prim("DUMP", |f| {
        let (a, n) = f.pop2()?;
        let i = f.check(a, n.max(0))?;
        let mut text = String::new();
        for (row, bytes) in f.mem[i..i + n.max(0) as usize].chunks(16).enumerate() {
            text += &format!("{:08X} ", a as usize + 16 * row);
            for b in bytes {
                text += &format!(" {b:02X}");
            }
            text += &"   ".repeat(16 - bytes.len());
            text += "  ";
            text.extend(bytes.iter().map(|&b| {
                if b == b' ' || b.is_ascii_graphic() {
                    b as char
                } else {
                    '.'
                }
            }));
            text.push('\n');
        }
        f.emit_str(&text);
        Ok(())
    });
}
