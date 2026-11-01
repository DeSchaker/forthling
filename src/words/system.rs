//! System words: INCLUDE, ACCEPT, EVALUATE, ENVIRONMENT? and the like.

use crate::*;

pub(crate) fn install(f: &mut Forth) {
    f.prim("WORDS", |f| {
        f.words();
        Ok(())
    });
    f.prim("SEE", Forth::see);
    f.prim("INCLUDE", |f| {
        let path = f.name()?;
        f.need(Permission::Files, || format!("INCLUDE {path}"))?;
        f.include(&path)
    });
    f.prim("INCLUDED", |f| {
        let path = f.pop_name()?;
        f.need(Permission::Files, || format!("INCLUDED {path}"))?;
        f.include(&path)
    });
    f.prim("INCLUDE-FILE", |f| {
        let fid = f.pop()?;
        f.include_file(fid)
    });
    f.prim("REQUIRE", |f| {
        let path = f.name()?;
        f.need(Permission::Files, || format!("REQUIRE {path}"))?;
        f.require(&path)
    });
    f.prim("REQUIRED", |f| {
        let path = f.pop_name()?;
        f.need(Permission::Files, || format!("REQUIRED {path}"))?;
        f.require(&path)
    });
    f.prim("ACCEPT", |f| {
        let (i, n) = f.pop_str()?;
        let line = f.read_line().unwrap_or_default();
        let len = line.len().min(n);
        f.mem[i..i + len].copy_from_slice(&line[..len]);
        f.push(to_cell(len));
        Ok(())
    });
    f.prim("KEY", |f| {
        let c = f.read_key().ok_or(Throw::new(UNEXPECTED_EOF))?;
        f.push(Cell::from(c));
        Ok(())
    });
    f.prim(">NUMBER", |f| {
        let (a, n) = f.pop2()?;
        let ud = f.pop_ud()?;
        let n = n.max(0);
        let i = f.check(a, n)?;
        #[allow(clippy::cast_possible_truncation, reason = "base() returns 2..=36, or 10")]
        let base = f.base() as u32;
        let (ud, used) = to_number(ud, &f.mem[i..i + to_index(n)], base);
        f.push_ud(ud);
        f.push(a + to_cell(used));
        f.push(n - to_cell(used));
        Ok(())
    });
    f.prim("EVALUATE", |f| {
        let (i, n) = f.pop_str()?;
        f.charge(n)?;
        f.evaluate(i, n)
    });
    f.prim("ABORT", |_| Err(Throw::new(ABORT)));
    f.prim("QUIT", |_| Err(Throw::new(QUIT)));
    f.prim("ENVIRONMENT?", |f| {
        let (i, n) = f.pop_str()?;
        let query = f.text(i, n).to_ascii_uppercase();
        match query.as_str() {
            "#LOCALS" => f.push(to_cell(MAX_LOCALS)),
            "/COUNTED-STRING" | "MAX-CHAR" => f.push(255),
            "/HOLD" => f.push(to_cell(HOLD_END - HOLD_START)),
            "/PAD" => f.push(to_cell(BLOCK_BUFS - PAD_ADDR)),
            "ADDRESS-UNIT-BITS" => f.push(8),
            "FLOATING" | "FLOATING-EXT" | "FLOORED" | "MAX-U" => f.push(-1),
            "FLOATING-STACK" => f.push(to_cell(FSTACK_FLOATS)),
            "MAX-D" => f.push_d(i128::MAX),
            "MAX-FLOAT" => f.fpush(f64::MAX)?,
            "MAX-N" => f.push(Cell::MAX),
            "MAX-UD" => f.push_ud(u128::MAX),
            "MAX-XCHAR" => f.push(Cell::from(MAX_XCHAR)),
            "RETURN-STACK-CELLS" => f.push(to_cell(RSTACK_CELLS)),
            "STACK-CELLS" => f.push(to_cell(STACK_CELLS)),
            "WORDLISTS" => f.push(to_cell(MAX_ORDER)),
            "XCHAR-ENCODING" => {
                let (a, n) = f.encoding_name()?;
                f.push(to_cell(a));
                f.push(to_cell(n));
            }
            "XCHAR-MAXMEM" => f.push(to_cell(XCHAR_MAXMEM)),
            _ => {
                if let Some(&version) = f.extensions.get(&query) {
                    f.push(version);
                } else {
                    f.push(0); // unknown query
                    return Ok(());
                }
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
