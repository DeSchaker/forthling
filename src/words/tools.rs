//! Programming tools.

use crate::*;
use std::fmt::Write as _;

pub(crate) fn install(f: &mut Forth) {
    f.imm("[IF]", |f| match f.pop()? {
        0 => f.skip_conditional(true),
        _ => Ok(()),
    });
    f.imm("[ELSE]", |f| f.skip_conditional(false));
    f.imm("[THEN]", |_| Ok(()));
    // Gforth: [IFDEF] and [IFUNDEF] test a name; [ENDIF] ends them like [THEN].
    f.imm("[IFDEF]", |f| {
        let name = f.name()?;
        if f.find(&name).is_some() { Ok(()) } else { f.skip_conditional(true) }
    });
    f.imm("[IFUNDEF]", |f| {
        let name = f.name()?;
        if f.find(&name).is_none() { Ok(()) } else { f.skip_conditional(true) }
    });
    f.imm("[ENDIF]", |_| Ok(()));
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
        let n = usize::try_from(n).ok().filter(|&n| n <= len).ok_or(Throw::new(STACK_UNDERFLOW))?;
        if f.rstack.len() + n >= RSTACK_CELLS {
            return Err(Throw::new(RSTACK_OVERFLOW));
        }
        f.rstack.extend(f.stack.drain(len - n..));
        f.rstack.push(to_cell(n));
        Ok(())
    });
    f.prim("NR>", |f| {
        let n = f.rstack.pop().ok_or(Throw::new(RSTACK_UNDERFLOW))?;
        let len = f.rstack.len();
        let n =
            usize::try_from(n).ok().filter(|&n| n <= len).ok_or(Throw::new(RSTACK_UNDERFLOW))?;
        f.stack.extend(f.rstack.drain(len - n..));
        f.push(to_cell(n));
        Ok(())
    });
    // A synonym is a copy of the word: the same behaviour and immediacy.
    f.prim("SYNONYM", |f| {
        let name = f.name()?;
        let old = f.name()?;
        let xt = f.find_or_err(&old)?;
        let (kind, immediate) = (f.dict[xt].kind.clone(), f.dict[xt].immediate);
        let compile_only = f.dict[xt].compile_only;
        let new = f.new_word(&name, kind)?;
        f.dict[new].immediate = immediate;
        f.dict[new].compile_only = compile_only;
        Ok(())
    });
    // ALIAS ( xt "name" -- ): a word that does what xt does. Unlike SYNONYM it is
    // not immediate, whatever xt is.
    f.prim("ALIAS", |f| {
        let xt = f.pop()?;
        let name = f.name()?;
        let old = usize::try_from(xt).ok().filter(|&i| i < f.dict.len());
        let old = old.ok_or(Throw::new(INVALID_ARGUMENT))?;
        let (kind, compile_only) = (f.dict[old].kind.clone(), f.dict[old].compile_only);
        let new = f.new_word(&name, kind)?;
        f.dict[new].compile_only = compile_only;
        Ok(())
    });
    f.prim("DUMP", |f| {
        let (i, n) = f.pop_str()?;
        f.charge(4 * n)?;
        let mut text = String::new();
        for (row, bytes) in f.mem[i..i + n].chunks(16).enumerate() {
            let _ = write!(text, "{:08X} ", i + 16 * row);
            for b in bytes {
                let _ = write!(text, " {b:02X}");
            }
            text += &"   ".repeat(16 - bytes.len());
            text += "  ";
            text.extend(
                bytes.iter().map(
                    |&b| {
                        if b == b' ' || b.is_ascii_graphic() { b as char } else { '.' }
                    },
                ),
            );
            text.push('\n');
        }
        f.emit_str(&text);
        Ok(())
    });
}
