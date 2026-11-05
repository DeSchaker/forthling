//! Dynamic strings and string arrays: the word list DYNAMIC-STRINGS, installed as the
//! extension `Strings`. The words are written in Forth (strings.fs); only
//! `$TMP`, which needs the output of the system, is in Rust.

use crate::*;

/// The extension: `forth.install(&Strings)`; ENVIRONMENT? knows it as
/// FORTHLING-DYNAMIC-STRINGS.
pub struct Strings;

/// The block of the last `$TMP`, which the next `$TMP` frees.
#[derive(Default)]
struct Tmp(Option<usize>);

impl Extension for Strings {
    fn name(&self) -> &'static str {
        "FORTHLING-DYNAMIC-STRINGS"
    }

    fn install(&self, f: &mut Forth) {
        let wid = f.wordlist("DYNAMIC-STRINGS");
        f.add_word(wid, "$TMP", tmp).expect("DYNAMIC-STRINGS is a word list");
        // Compile strings.fs into the word list, and leave the search order
        // and the compilation word list as they were.
        let (order, current) = (f.order.clone(), f.current_wid);
        f.interpret("ALSO DYNAMIC-STRINGS DEFINITIONS").expect("DYNAMIC-STRINGS is a word list");
        let result = f.interpret(include_str!("strings.fs"));
        f.order = order;
        f.current_wid = current;
        result.expect("strings.fs must compile");
        install_text(f);
    }
}

/// The word list STRINGS: whole-string helpers on `( c-addr u )`. Only JOIN allocates;
/// UPPER and LOWER change the text in place (ASCII letters only).
fn install_text(f: &mut Forth) {
    let wid = f.wordlist("STRINGS");
    let add = |f: &mut Forth, name: &str, word: fn(&mut Forth) -> Res| {
        f.add_word(wid, name, word).expect("STRINGS is a word list");
    };
    add(f, "TRIM", |f| trim(f, true, true));
    add(f, "-LEADING", |f| trim(f, true, false));
    add(f, "UPPER", |f| case(f, u8::to_ascii_uppercase));
    add(f, "LOWER", |f| case(f, u8::to_ascii_lowercase));
    add(f, "STRING-PREFIX?", |f| {
        let (text, key) = two(f)?;
        f.push(flag(text.starts_with(&key)));
        Ok(())
    });
    add(f, "STRING-SUFFIX?", |f| {
        let (text, key) = two(f)?;
        f.push(flag(text.ends_with(&key)));
        Ok(())
    });
    add(f, "CONTAINS?", |f| {
        let (text, key) = two(f)?;
        let found = key.is_empty() || text.windows(key.len()).any(|w| w == key.as_slice());
        f.push(flag(found));
        Ok(())
    });
    add(f, "JOIN", join);
}

/// Pops `( c-addr1 u1 c-addr2 u2 )` as two copies and charges for them.
fn two(f: &mut Forth) -> Res<(Vec<u8>, Vec<u8>)> {
    let (a2, n2) = f.pop2()?;
    let (a1, n1) = f.pop2()?;
    let (text, key) = (f.bytes(a1, n1)?, f.bytes(a2, n2)?);
    f.charge(text.len() + key.len())?;
    Ok((text, key))
}

/// TRIM and -LEADING: drop the white space (characters up to the blank) from the ends.
fn trim(f: &mut Forth, front: bool, back: bool) -> Res {
    let (a, n) = f.pop2()?;
    let text = f.bytes(a, n)?;
    f.charge(text.len())?;
    let blank = |b: &u8| *b <= b' ';
    let start = if front { text.iter().position(|b| !blank(b)).unwrap_or(text.len()) } else { 0 };
    let end = if back {
        text.iter().rposition(|b| !blank(b)).map_or(start, |i| i + 1)
    } else {
        text.len()
    };
    f.push(a + to_cell(start));
    f.push(to_cell(end.max(start) - start));
    Ok(())
}

/// UPPER and LOWER: ( c-addr u -- c-addr u ), the text changed in place.
fn case(f: &mut Forth, change: fn(&u8) -> u8) -> Res {
    let (a, n) = f.pop2()?;
    let (i, len) = (f.check(a, n.max(0))?, to_index(n.max(0)));
    f.charge(len)?;
    for b in &mut f.mem[i..i + len] {
        *b = change(b);
    }
    f.push(a);
    f.push(to_cell(len));
    Ok(())
}

/// `JOIN ( array sep-addr sep-u -- c-addr u )`: the elements of a `$[]` array with the
/// separator between them, in an `ALLOCATE`d string.
fn join(f: &mut Forth) -> Res {
    let (sa, sn) = f.pop2()?;
    let sep = f.bytes(sa, sn)?;
    let array = f.pop()?;
    let cell = |f: &Forth, a: Cell| -> Res<Cell> {
        let i = f.check(a, 8)?;
        Ok(Cell::from_le_bytes(f.mem[i..i + 8].try_into().expect("8 bytes")))
    };
    // A dynamic string is a cell: 0, or a block with a length cell, then the characters.
    let block = cell(f, array)?;
    let count = if block == 0 { 0 } else { to_index(cell(f, block)?) / 8 };
    let mut out = Vec::new();
    for k in 0..count {
        if k > 0 {
            out.extend_from_slice(&sep);
        }
        let s = cell(f, block + 8 + to_cell(k * 8))?;
        if s != 0 {
            let len = cell(f, s)?;
            out.extend_from_slice(&f.bytes(s + 8, len)?);
        }
        f.charge(out.len())?;
    }
    let a = f
        .heap
        .allocate(out.len().max(1), f.here)
        .ok_or_else(|| Throw::with(ALLOCATE_FAILED, "JOIN"))?;
    f.mem[a..a + out.len()].copy_from_slice(&out);
    f.push(to_cell(a));
    f.push(to_cell(out.len()));
    Ok(())
}

/// `$TMP ( xt -- c-addr u )`: runs xt with its output caught. The text is
/// valid until the next `$TMP`.
fn tmp(f: &mut Forth) -> Res {
    let xt = f.pop()?;
    let xt = usize::try_from(xt).ok().filter(|&i| i < f.dict.len()).ok_or_else(|| {
        Throw::with(INVALID_ARGUMENT, format!("$TMP: invalid execution token {xt}"))
    })?;
    if let Some(old) = f.state::<Tmp>().0.take() {
        f.heap.free(old);
    }
    let start = f.out.len();
    let ran = f.execute(xt);
    let text = f.out.split_off(start.min(f.out.len()));
    ran?;
    let a =
        f.heap.allocate(text.len(), f.here).ok_or_else(|| Throw::with(ALLOCATE_FAILED, "$TMP"))?;
    f.mem[a..a + text.len()].copy_from_slice(&text);
    f.state::<Tmp>().0 = Some(a);
    f.push(to_cell(a));
    f.push(to_cell(text.len()));
    Ok(())
}
