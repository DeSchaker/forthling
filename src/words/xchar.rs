//! The XCHAR word set and its extensions: extended characters in UTF-8.
//! CHAR, [CHAR] and PARSE take xchars in define.rs and console.rs.

use crate::*;

pub(crate) fn install(f: &mut Forth) {
    f.prim("X-SIZE", |f| {
        let (a, n) = f.pop2()?;
        let size = if n <= 0 { 0 } else { decode(f.xbytes(a, to_index(n))?).1 };
        f.push(to_cell(size));
        Ok(())
    });
    f.prim("XC!+", |f| {
        let (xc, a) = f.pop2()?;
        let bytes = encode(xc);
        let i = f.check(a, to_cell(bytes.len()))?;
        f.mem[i..i + bytes.len()].copy_from_slice(&bytes);
        f.push(a + to_cell(bytes.len()));
        Ok(())
    });
    f.prim("XC!+?", |f| {
        let (a, n) = f.pop2()?;
        let xc = f.pop()?;
        let bytes = encode(xc);
        let len = to_cell(bytes.len());
        let fits = len <= n;
        if fits {
            let i = f.check(a, len)?;
            f.mem[i..i + bytes.len()].copy_from_slice(&bytes);
        }
        let used = if fits { len } else { 0 };
        f.push(a + used);
        f.push(n - used);
        f.push(flag(fits));
        Ok(())
    });
    f.prim("XC,", |f| {
        let bytes = encode(f.pop()?);
        let a = f.reserve(bytes.len())?;
        f.mem[a..a + bytes.len()].copy_from_slice(&bytes);
        Ok(())
    });
    f.prim("XC-SIZE", |f| {
        let xc = f.pop()?;
        f.push(to_cell(encode(xc).len()));
        Ok(())
    });
    f.prim("XC@+", |f| {
        let a = f.pop()?;
        let (xc, size) = decode(f.xbytes(a, XCHAR_MAXMEM)?);
        f.push(a + to_cell(size));
        f.push(xc);
        Ok(())
    });
    f.prim("XCHAR+", |f| {
        let a = f.pop()?;
        let size = decode(f.xbytes(a, XCHAR_MAXMEM)?).1;
        f.push(a + to_cell(size));
        Ok(())
    });
    f.prim("XEMIT", |f| {
        let bytes = encode(f.pop()?);
        f.emit_bytes(&bytes);
        Ok(())
    });
    f.prim("XKEY", |f| {
        let xc = f.read_xkey().ok_or(Throw::new(UNEXPECTED_EOF))?;
        f.push(xc);
        Ok(())
    });

    // XCHAR EXT
    f.prim("+X/STRING", |f| {
        let (a, n) = f.pop2()?;
        let size = if n <= 0 { 0 } else { to_cell(decode(f.xbytes(a, to_index(n))?).1) };
        f.push(a + size);
        f.push(n - size);
        Ok(())
    });
    f.prim("X\\STRING-", |f| {
        let (a, n) = f.pop2()?;
        let len = if n <= 0 { n } else { to_cell(last_start(f.xbytes(a, to_index(n))?)) };
        f.push(a);
        f.push(len);
        Ok(())
    });
    f.prim("-TRAILING-GARBAGE", |f| {
        let (a, n) = f.pop2()?;
        let mut len = n;
        if n > 0 {
            let bytes = f.xbytes(a, to_index(n))?;
            // An xchar whose last bytes are missing.
            let j = (0..bytes.len()).rev().take(XCHAR_MAXMEM).find(|&j| !is_cont(bytes[j]));
            if let Some(j) = j.filter(|&j| lead_size(bytes[j]) > bytes.len() - j) {
                len = to_cell(j);
            }
        }
        f.push(a);
        f.push(len);
        Ok(())
    });
    f.prim("X-WIDTH", |f| {
        let (i, n) = f.pop_str()?;
        f.charge(n)?;
        let mut bytes = &f.mem[i..i + n];
        let mut columns = 0;
        while !bytes.is_empty() {
            let (xc, size) = decode(bytes);
            columns += width(xc);
            bytes = &bytes[size..];
        }
        f.push(columns);
        Ok(())
    });
    f.prim("XC-WIDTH", |f| {
        let xc = f.pop()?;
        f.push(width(xc));
        Ok(())
    });
    f.prim("XCHAR-", |f| {
        let a = f.pop()?;
        // Back over the continuation bytes to the start of the xchar.
        let mut b = a - 1;
        while b > 0 && a - b < to_cell(XCHAR_MAXMEM) && is_cont(f.mem[f.check(b, 1)?]) {
            b -= 1;
        }
        let start = f.check(b, 1)?;
        if decode(&f.mem[start..to_index(a)]).1 != to_index(a - b) {
            b = a - 1;
        }
        f.push(b);
        Ok(())
    });
    f.prim("XHOLD", |f| {
        let bytes = encode(f.pop()?);
        for &b in bytes.iter().rev() {
            f.hold(b)?;
        }
        Ok(())
    });
}
