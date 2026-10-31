//! Comments, string literals and the STRING word set.

use crate::*;

pub(crate) fn install(f: &mut Forth) {
    f.imm("(", |f| {
        // In a file, a comment continues on the next lines until the `)`.
        loop {
            let (a, n) = f.parse(b')');
            let in_file = matches!(f.sources.last().unwrap().origin, Origin::File { .. });
            if a + n < f.parse_area().1 || !in_file || !f.refill()? {
                return Ok(());
            }
        }
    });
    // In a block, \ skips to the end of its line of 64 characters.
    f.imm("\\", |f| {
        if let Origin::Block = f.sources.last().unwrap().origin {
            let line_end = (f.get(IN_ADDR) as usize / BLOCK_LINE + 1) * BLOCK_LINE;
            f.set(IN_ADDR, line_end.min(BLOCK_SIZE) as Cell);
            return Ok(());
        }
        let end = f.parse_area().1;
        f.set_in(end);
        Ok(())
    });
    f.imm(".(", |f| {
        let (a, n) = f.parse(b')');
        f.out.extend_from_slice(&f.mem[a..a + n]);
        Ok(())
    });
    f.imm(".\"", |f| {
        let (a, n) = f.parse(b'"');
        if f.compiling() {
            f.code.push(Instr::Print(Rc::from(&f.mem[a..a + n])));
        } else {
            f.out.extend_from_slice(&f.mem[a..a + n]);
        }
        Ok(())
    });
    f.imm("ABORT\"", |f| {
        let (a, n) = f.parse(b'"');
        let msg: Rc<[u8]> = Rc::from(&f.mem[a..a + n]);
        if f.compiling() {
            f.code.push(Instr::Abort(msg));
            Ok(())
        } else {
            f.abort_if(&msg)
        }
    });
    f.imm("S\"", |f| {
        let (a, n) = f.parse(b'"');
        let text = f.mem[a..a + n].to_vec();
        f.string_literal(&text)
    });
    f.imm("S\\\"", |f| {
        let text = f.parse_escaped();
        f.string_literal(&text)
    });
    f.imm("C\"", |f| {
        f.need_def()?;
        let (a, n) = f.parse(b'"');
        if n > 255 {
            return Err(Throw::with(-18, "counted string longer than 255 characters"));
        }
        let addr = f.reserve(n + 1)?;
        f.mem[addr] = n as u8;
        f.mem.copy_within(a..a + n, addr + 1);
        f.align_here();
        f.code.push(Instr::Lit(addr as Cell));
        Ok(())
    });

    // strings; /STRING BLANK -TRAILING are in the prelude
    f.prim("COMPARE", |f| {
        let (a2, n2) = f.pop2()?;
        let (a1, n1) = f.pop2()?;
        let order = f.bytes(a1, n1)?.cmp(&f.bytes(a2, n2)?);
        f.push(order as Cell);
        Ok(())
    });
    f.prim("SEARCH", |f| {
        let (a2, n2) = f.pop2()?;
        let (a1, n1) = f.pop2()?;
        let (text, key) = (f.bytes(a1, n1)?, f.bytes(a2, n2)?);
        let found = match key.len() {
            0 => Some(0),
            n => text.windows(n).position(|w| w == key),
        };
        let at = found.unwrap_or(0) as Cell;
        f.push(a1 + at);
        f.push(n1.max(0) - at);
        f.push(flag(found.is_some()));
        Ok(())
    });
    // CMOVE and CMOVE> copy one character at a time, so an overlapping
    // copy repeats the characters it already copied.
    f.prim("CMOVE", |f| {
        let n = f.pop()?;
        let (src, dst) = f.pop2()?;
        let (s, d) = (f.check(src, n.max(0))?, f.check(dst, n.max(0))?);
        for k in 0..n.max(0) as usize {
            f.mem[d + k] = f.mem[s + k];
        }
        Ok(())
    });
    f.prim("CMOVE>", |f| {
        let n = f.pop()?;
        let (src, dst) = f.pop2()?;
        let (s, d) = (f.check(src, n.max(0))?, f.check(dst, n.max(0))?);
        for k in (0..n.max(0) as usize).rev() {
            f.mem[d + k] = f.mem[s + k];
        }
        Ok(())
    });
    f.imm("SLITERAL", |f| {
        f.need_def()?;
        let (a, n) = f.pop2()?;
        let text = f.bytes(a, n)?;
        f.compile_string(&text)
    });
    f.prim("REPLACES", |f| {
        let (a2, n2) = f.pop2()?;
        let (a1, n1) = f.pop2()?;
        let (text, name) = (f.bytes(a1, n1)?, f.bytes(a2, n2)?);
        if name.contains(&b'%') {
            return Err(Throw::with(-79, "the name contains %"));
        }
        let name = String::from_utf8_lossy(&name).to_ascii_uppercase();
        f.substitutions.insert(name, text);
        Ok(())
    });
    f.prim("SUBSTITUTE", |f| {
        let (a2, n2) = f.pop2()?;
        let (a1, n1) = f.pop2()?;
        let (out, count) = f.substitute(&f.bytes(a1, n1)?);
        f.push(a2);
        if out.len() as Cell > n2 {
            f.push(0);
            f.push(-78); // the result does not fit
            return Ok(());
        }
        let d = f.check(a2, out.len() as Cell)?;
        f.mem[d..d + out.len()].copy_from_slice(&out);
        f.push(out.len() as Cell);
        f.push(count);
        Ok(())
    });
    // UNESCAPE doubles every %, so SUBSTITUTE gives the text back unchanged.
    f.prim("UNESCAPE", |f| {
        let a2 = f.pop()?;
        let (a1, n1) = f.pop2()?;
        let mut out = Vec::new();
        for c in f.bytes(a1, n1)? {
            out.extend_from_slice(if c == b'%' { b"%%" } else { std::slice::from_ref(&c) });
        }
        let d = f.check(a2, out.len() as Cell)?;
        f.mem[d..d + out.len()].copy_from_slice(&out);
        f.push(a2);
        f.push(out.len() as Cell);
        Ok(())
    });
}
