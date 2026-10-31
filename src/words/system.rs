//! System words: INCLUDE, ACCEPT, EVALUATE, ENVIRONMENT? and the like.

use crate::*;

pub(crate) fn install(f: &mut Forth) {
    f.prim("WORDS", |f| {
        f.words();
        Ok(())
    });
    f.prim("SEE", |f| f.see());
    f.prim("INCLUDE", |f| {
        let path = f.name()?;
        f.include(&path)
    });
    f.prim("INCLUDED", |f| {
        let path = f.pop_name()?;
        f.include(&path)
    });
    f.prim("INCLUDE-FILE", |f| {
        let fid = f.pop()?;
        f.include_file(fid)
    });
    f.prim("REQUIRE", |f| {
        let path = f.name()?;
        f.require(&path)
    });
    f.prim("REQUIRED", |f| {
        let path = f.pop_name()?;
        f.require(&path)
    });
    f.prim("ACCEPT", |f| {
        let (a, n) = f.pop2()?;
        let n = n.max(0);
        let i = f.check(a, n)?;
        let line = f.read_line().unwrap_or_default();
        let len = line.len().min(n as usize);
        f.mem[i..i + len].copy_from_slice(&line[..len]);
        f.push(len as Cell);
        Ok(())
    });
    f.prim("KEY", |f| {
        let c = f.read_key().ok_or(Throw::new(-39))?;
        f.push(c as Cell);
        Ok(())
    });
    f.prim(">NUMBER", |f| {
        let (a, n) = f.pop2()?;
        let ud = f.pop_ud()?;
        let n = n.max(0);
        let i = f.check(a, n)?;
        let base = f.base() as u32;
        let (ud, used) = to_number(ud, &f.mem[i..i + n as usize], base);
        f.push_ud(ud);
        f.push(a + used as Cell);
        f.push(n - used as Cell);
        Ok(())
    });
    f.prim("EVALUATE", |f| {
        let (a, n) = f.pop2()?;
        let n = n.max(0);
        let i = f.check(a, n)?;
        f.evaluate(i, n as usize)
    });
    f.prim("ABORT", |_| Err(Throw::new(-1)));
    f.prim("QUIT", |_| Err(Throw::new(QUIT)));
    f.prim("ENVIRONMENT?", |f| {
        let (a, n) = f.pop2()?;
        let i = f.check(a, n.max(0))?;
        let query = f.text(i, n.max(0) as usize).to_ascii_uppercase();
        match query.as_str() {
            "#LOCALS" => f.push(MAX_LOCALS as Cell),
            "/COUNTED-STRING" => f.push(255),
            "/HOLD" => f.push((HOLD_END - HOLD_START) as Cell),
            "/PAD" => f.push((BLOCK_BUFS - PAD_ADDR) as Cell),
            "ADDRESS-UNIT-BITS" => f.push(8),
            "FLOORED" => f.push(-1),
            "MAX-CHAR" => f.push(255),
            "MAX-D" => f.push_d(i128::MAX),
            "MAX-N" => f.push(Cell::MAX),
            "MAX-U" => f.push(-1),
            "MAX-UD" => f.push_ud(u128::MAX),
            "MAX-XCHAR" => f.push(MAX_XCHAR as Cell),
            "RETURN-STACK-CELLS" => f.push(RSTACK_CELLS as Cell),
            "STACK-CELLS" => f.push(STACK_CELLS as Cell),
            "WORDLISTS" => f.push(MAX_ORDER as Cell),
            "XCHAR-ENCODING" => {
                let (a, n) = f.encoding_name()?;
                f.push(a as Cell);
                f.push(n as Cell);
            }
            "XCHAR-MAXMEM" => f.push(XCHAR_MAXMEM as Cell),
            _ => {
                f.push(0); // unknown query
                return Ok(());
            }
        }
        f.push(-1);
        Ok(())
    });
    f.prim("BYE", |f| {
        f.bye = true;
        Ok(())
    });
}
