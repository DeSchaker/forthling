//! Output, input and parsing words.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a character, a digit or a delimiter is the low byte of a cell, as the standard says"
)]

use crate::*;

pub(crate) fn install(f: &mut Forth) {
    install_output(f);
    install_input(f);
}

fn install_output(f: &mut Forth) {
    // pictured numeric output; #S SIGN HOLDS . U. are in the prelude
    f.prim("<#", |f| {
        f.hld = HOLD_END;
        Ok(())
    });
    f.prim("HOLD", |f| {
        let c = f.pop()?;
        f.hold(c as u8)
    });
    f.prim("#", |f| {
        let ud = f.pop_ud()?;
        let base = u128::from(f.base());
        let d = std::char::from_digit((ud % base) as u32, base as u32).unwrap();
        f.hold(d.to_ascii_uppercase() as u8)?;
        f.push_ud(ud / base);
        Ok(())
    });
    f.prim("#>", |f| {
        f.pop2()?;
        f.push(to_cell(f.hld));
        f.push(to_cell(HOLD_END - f.hld));
        Ok(())
    });
    f.prim(".S", |f| {
        let mut s = format!("<{}> ", f.stack.len());
        for &n in &f.stack {
            s += &f.fmt_num(n);
            s.push(' ');
        }
        f.emit_str(&s);
        Ok(())
    });
    f.prim("EMIT", |f| {
        let c = f.pop()?;
        f.emit_bytes(&[c as u8]);
        Ok(())
    });
    f.prim("TYPE", |f| {
        let (a, n) = f.pop2()?;
        if n > 0 {
            let i = f.check(a, n)?;
            f.charge(to_index(n))?;
            f.emit_memory(i, to_index(n));
        }
        Ok(())
    });
}

fn install_input(f: &mut Forth) {
    // input and parsing
    f.prim("SOURCE", |f| {
        let s = f.sources.top();
        let (buf, len) = (s.buf, s.len);
        f.push(to_cell(buf));
        f.push(to_cell(len));
        Ok(())
    });
    f.prim("PARSE", |f| {
        let delim = f.pop()?;
        let (a, n) = match delim {
            0..=0x7F => f.parse(delim as u8),
            _ => f.parse_seq(&encode(delim)),
        };
        f.push(to_cell(a));
        f.push(to_cell(n));
        Ok(())
    });
    f.prim("PARSE-NAME", |f| {
        let (a, n) = f.parse_name();
        f.push(to_cell(a));
        f.push(to_cell(n));
        Ok(())
    });
    f.prim("REFILL", |f| {
        let more = f.refill()?;
        f.push(flag(more));
        Ok(())
    });
    f.prim("SOURCE-ID", |f| {
        f.push(f.source_id());
        Ok(())
    });
    f.prim("SAVE-INPUT", |f| {
        f.save_input();
        Ok(())
    });
    f.prim("RESTORE-INPUT", |f| {
        let restored = f.restore_input()?;
        f.push(flag(!restored));
        Ok(())
    });
    f.prim("WORD", |f| {
        let delim = f.pop()? as u8;
        let (a, n) = if delim == b' ' { f.parse_name() } else { f.scan(true, |c| c == delim) };
        if n > 255 {
            return Err(Throw::new(PARSED_OVERFLOW));
        }
        f.mem.copy_within(a..a + n, WORD_BUF + 1);
        f.mem[WORD_BUF] = n as u8;
        f.mem[WORD_BUF + 1 + n] = b' ';
        f.push(to_cell(WORD_BUF));
        Ok(())
    });
    f.prim("FIND", |f| {
        let a = f.pop()?;
        let i = f.check(a, 1)?;
        let n = f.mem[i] as usize;
        f.check(a, 1 + to_cell(n))?;
        let name = f.text(i + 1, n);
        if let Some(xt) = f.find(&name) {
            f.push(to_cell(xt));
            f.push(if f.dict[xt].immediate { 1 } else { -1 });
        } else {
            f.push(a);
            f.push(0);
        }
        Ok(())
    });
}
