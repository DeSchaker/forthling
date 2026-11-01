//! Comments, string literals and the STRING word set.

use crate::*;

pub(crate) fn install(f: &mut Forth) {
    f.imm("(", |f| {
        // In a file, a comment continues on the next lines until the `)`.
        loop {
            let (a, n) = f.parse(b')');
            let in_file = matches!(f.sources.top().origin, Origin::File { .. });
            if a + n < f.parse_area().1 || !in_file || !f.refill()? {
                return Ok(());
            }
        }
    });
    // In a block, \ skips to the end of its line of 64 characters.
    f.imm("\\", |f| {
        if let Origin::Block = f.sources.top().origin {
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
        f.emit_memory(a, n);
        Ok(())
    });
    f.imm(".\"", |f| {
        let (a, n) = f.parse(b'"');
        if f.compiling() {
            f.code.push(Instr::Print(Rc::from(&f.mem[a..a + n])));
        } else {
            f.emit_memory(a, n);
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
    f.compile_imm("C\"", |f| {
        let (a, n) = f.parse(b'"');
        if n > 255 {
            return Err(Throw::with(PARSED_OVERFLOW, "counted string longer than 255 characters"));
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
        f.charge((n1.max(0) + n2.max(0)) as usize)?;
        let order = f.bytes(a1, n1)?.cmp(&f.bytes(a2, n2)?);
        f.push(order as Cell);
        Ok(())
    });
    f.prim("SEARCH", |f| {
        let (a2, n2) = f.pop2()?;
        let (a1, n1) = f.pop2()?;
        let (text, key) = (f.bytes(a1, n1)?, f.bytes(a2, n2)?);
        f.charge(text.len() + key.len())?;
        let found = find(&text, &key);
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
        f.charge(n.max(0) as usize)?;
        for k in 0..n.max(0) as usize {
            f.mem[d + k] = f.mem[s + k];
        }
        Ok(())
    });
    f.prim("CMOVE>", |f| {
        let n = f.pop()?;
        let (src, dst) = f.pop2()?;
        let (s, d) = (f.check(src, n.max(0))?, f.check(dst, n.max(0))?);
        f.charge(n.max(0) as usize)?;
        for k in (0..n.max(0) as usize).rev() {
            f.mem[d + k] = f.mem[s + k];
        }
        Ok(())
    });
    f.compile_imm("SLITERAL", |f| {
        let (a, n) = f.pop2()?;
        let text = f.bytes(a, n)?;
        f.compile_string(&text)
    });
    f.prim("REPLACES", |f| {
        let (a2, n2) = f.pop2()?;
        let (a1, n1) = f.pop2()?;
        let (text, name) = (f.bytes(a1, n1)?, f.bytes(a2, n2)?);
        if name.contains(&b'%') {
            return Err(Throw::with(REPLACES_FAILED, "the name contains %"));
        }
        let name = String::from_utf8_lossy(&name).to_ascii_uppercase();
        f.charge(name.len() + text.len())?;
        let size = |name: &str, text: &[u8]| REPLACES_SPACE + name.len() + text.len();
        f.space += size(&name, &text);
        let old = f.substitutions.insert(name.clone(), text);
        if let Some(old) = &old {
            f.space -= size(&name, old);
        }
        if let Err(e) = f.check_space() {
            f.space -= size(&name, &f.substitutions[&name]);
            match old {
                Some(old) => {
                    f.space += size(&name, &old);
                    f.substitutions.insert(name, old);
                }
                None => {
                    f.substitutions.remove(&name);
                }
            }
            return Err(e);
        }
        Ok(())
    });
    f.prim("SUBSTITUTE", |f| {
        let (a2, n2) = f.pop2()?;
        let (a1, n1) = f.pop2()?;
        f.charge(n1.max(0) as usize)?;
        let (out, count) = f.substitute(&f.bytes(a1, n1)?);
        f.charge(out.len())?;
        f.push(a2);
        if out.len() as Cell > n2 {
            f.push(0);
            f.push(SUBSTITUTE_FAILED); // the result does not fit
            return Ok(());
        }
        let d = f.check(a2, out.len() as Cell)?;
        f.mem[d..d + out.len()].copy_from_slice(&out);
        f.push(out.len() as Cell);
        f.push(count);
        Ok(())
    });
    // UNESCAPE doubles every %, so SUBSTITUTE gives the text back unchanged.
    // A result that runs past the end of its buffer throws -9.
    f.prim("UNESCAPE", |f| {
        let a2 = f.pop()?;
        let (a1, n1) = f.pop2()?;
        f.charge(2 * n1.max(0) as usize)?;
        let mut out = Vec::new();
        for c in f.bytes(a1, n1)? {
            out.extend_from_slice(if c == b'%' { b"%%" } else { std::slice::from_ref(&c) });
        }
        let d = f.check(a2, out.len() as Cell)?;
        if d + out.len() > f.buffer_end(d) {
            return Err(Throw::with(INVALID_ADDRESS, "the result of UNESCAPE does not fit"));
        }
        f.mem[d..d + out.len()].copy_from_slice(&out);
        f.push(a2);
        f.push(out.len() as Cell);
        Ok(())
    });
}

/// Where key first occurs in text, in time linear in their lengths
/// (Knuth-Morris-Pratt), so that no SEARCH takes long in one step.
fn find(text: &[u8], key: &[u8]) -> Option<usize> {
    if key.is_empty() {
        return Some(0);
    }
    // fail[i]: the longest proper prefix of key[..=i] that is also its suffix.
    let mut fail = vec![0; key.len()];
    let mut k = 0;
    for i in 1..key.len() {
        while k > 0 && key[i] != key[k] {
            k = fail[k - 1];
        }
        if key[i] == key[k] {
            k += 1;
        }
        fail[i] = k;
    }
    let mut k = 0;
    for (i, &c) in text.iter().enumerate() {
        while k > 0 && c != key[k] {
            k = fail[k - 1];
        }
        if c == key[k] {
            k += 1;
            if k == key.len() {
                return Some(i + 1 - k);
            }
        }
    }
    None
}
