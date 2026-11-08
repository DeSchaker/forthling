//! The word list CSV: records of fields, read one field at a time and written
//! with quotes where they are needed (RFC 4180, with a separator that the
//! program may change).

use crate::{EXTRAS_ERROR, word};
use forthling::{Forth, Res, Throw, to_cell};

const LIST: &str = "CSV";

struct State {
    separator: u8,
}

impl Default for State {
    fn default() -> Self {
        State { separator: b',' }
    }
}

fn error(what: &str) -> Throw {
    Throw::with(EXTRAS_ERROR, what.to_string())
}

/// One field cut from the front of a text.
struct Field {
    /// Where the field is, without the quotes around it.
    start: usize,
    end: usize,
    /// How many bytes the field and what ends it take.
    used: usize,
    /// Does the field end the record?
    last: bool,
}

/// What ends a field at `i`: the separator, a line end, or the end of the text.
/// Returns the bytes used up to and including it, and whether the record ends.
fn terminator(text: &[u8], i: usize, separator: u8) -> Option<(usize, bool)> {
    match text.get(i) {
        None => Some((i, true)),
        Some(&b) if b == separator => Some((i + 1, false)),
        Some(b'\n') => Some((i + 1, true)),
        Some(b'\r') => Some((if text.get(i + 1) == Some(&b'\n') { i + 2 } else { i + 1 }, true)),
        Some(_) => None,
    }
}

fn field(text: &[u8], separator: u8) -> Res<Field> {
    if text.first() == Some(&b'"') {
        let mut i = 1;
        loop {
            match text.get(i) {
                None => return Err(error("a quoted field has no closing quote")),
                Some(b'"') if text.get(i + 1) == Some(&b'"') => i += 2,
                Some(b'"') => break,
                Some(_) => i += 1,
            }
        }
        let Some((used, last)) = terminator(text, i + 1, separator) else {
            return Err(error("text after the closing quote of a field"));
        };
        return Ok(Field { start: 1, end: i, used, last });
    }
    let mut i = 0;
    loop {
        if let Some((used, last)) = terminator(text, i, separator) {
            return Ok(Field { start: 0, end: i, used, last });
        }
        i += 1;
    }
}

/// Pops a text ( c-addr u ), counts its length as work and gives its place in
/// memory.
fn pop_text(f: &mut Forth) -> Res<(usize, usize)> {
    let (a, n) = f.pop_str()?;
    f.charge(n)?;
    Ok((a, n))
}

pub fn install(f: &mut Forth) {
    // CSV-SEPARATOR ( char -- ): the separator between fields, a comma to
    // begin with; 9 for tabs, 59 for a semicolon.
    word(f, LIST, "CSV-SEPARATOR", |f| {
        let c = f.pop()?;
        match u8::try_from(c) {
            Ok(b) if b != b'"' && b != b'\n' && b != b'\r' => {
                f.state::<State>().separator = b;
                Ok(())
            }
            _ => Err(error("the separator must be a byte other than a quote or a line end")),
        }
    });
    // CSV-FIELD ( c-addr u -- rest-addr rest-u field-addr field-u last? ):
    // the first field of the text, without the quotes around it (a doubled
    // quote inside stays doubled: CSV>STRING makes it one), and what is left
    // after the separator or line end that ended it. `last?` is true when the
    // field ended its record. An empty text gives an empty field that ends
    // the record.
    word(f, LIST, "CSV-FIELD", |f| {
        let separator = f.state::<State>().separator;
        let (a, n) = pop_text(f)?;
        let found = field(&f.memory()[a..a + n], separator)?;
        f.push(to_cell(a + found.used));
        f.push(to_cell(n - found.used));
        f.push(to_cell(a + found.start));
        f.push(to_cell(found.end - found.start));
        f.push(if found.last { -1 } else { 0 });
        Ok(())
    });
    // CSV-SKIP ( c-addr u -- rest-addr rest-u ): skips one record.
    word(f, LIST, "CSV-SKIP", |f| {
        let separator = f.state::<State>().separator;
        let (a, n) = pop_text(f)?;
        let mut used = 0;
        loop {
            let found = field(&f.memory()[a + used..a + n], separator)?;
            used += found.used;
            if found.last {
                break;
            }
        }
        f.push(to_cell(a + used));
        f.push(to_cell(n - used));
        Ok(())
    });
    // CSV>STRING ( c-addr u -- c-addr' u' ): a field with its doubled quotes
    // made single; the new text is ALLOCATEd.
    word(f, LIST, "CSV>STRING", |f| {
        let text = f.pop_bytes()?;
        f.charge(text.len())?;
        let mut out = Vec::with_capacity(text.len());
        let mut i = 0;
        while i < text.len() {
            out.push(text[i]);
            i += if text[i] == b'"' && text.get(i + 1) == Some(&b'"') { 2 } else { 1 };
        }
        f.push_bytes(&out)
    });
    // CSV-QUOTE ( c-addr u -- c-addr' u' ): the field as it is written: in
    // quotes, with quotes doubled, when it holds the separator, a quote or a
    // line end; otherwise as it is. The new text is ALLOCATEd.
    word(f, LIST, "CSV-QUOTE", |f| {
        let separator = f.state::<State>().separator;
        let text = f.pop_bytes()?;
        f.charge(text.len())?;
        let needs = text.iter().any(|&b| b == separator || matches!(b, b'"' | b'\n' | b'\r'));
        if !needs {
            return f.push_bytes(&text);
        }
        let mut out = Vec::with_capacity(text.len() + 2);
        out.push(b'"');
        for &b in &text {
            out.push(b);
            if b == b'"' {
                out.push(b'"');
            }
        }
        out.push(b'"');
        f.push_bytes(&out)
    });
}
