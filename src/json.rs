//! JSON: the word list JSON, installed as the extension `Json`. A JSON value
//! is its text, a string ( c-addr u ) in Forth memory. The reading words
//! give back pieces of that text, so they copy and allocate nothing, except
//! for the strings that JSON>STRING and JSON-NAME unescape, which are
//! `ALLOCATE`d. The writing words write JSON text to the output or, between
//! <JSON and JSON>, to a string, with the commas in their places.

use crate::*;
use std::fmt::Write as _;

/// Arrays and objects nested deeper than this are not read or written, so
/// that deep nesting cannot overflow the Rust stack; <JSON nests as deep.
const MAX_DEPTH: usize = 512;

/// The extension: `forth.install(&Json)`; ENVIRONMENT? knows it as
/// FORTHLING-JSON.
pub struct Json;

/// A range of the text: start and end.
type Span = (usize, usize);

/// An element of an array (no key) or of an object.
struct Entry {
    key: Option<Span>, // the key as JSON text, with its quotes
    value: Span,
}

fn skip_space(s: &[u8], mut i: usize) -> usize {
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | b'\r') {
        i += 1;
    }
    i
}

/// The end of the value that starts at i, if it is valid JSON.
fn value_end(s: &[u8], i: usize, depth: usize) -> Option<usize> {
    match *s.get(i)? {
        b'"' => string_end(s, i),
        b'{' | b'[' => entries(s, i, depth, false).map(|(_, end)| end),
        b't' => literal_end(s, i, b"true"),
        b'f' => literal_end(s, i, b"false"),
        b'n' => literal_end(s, i, b"null"),
        b'-' | b'0'..=b'9' => number_end(s, i),
        _ => None,
    }
}

fn literal_end(s: &[u8], i: usize, word: &[u8]) -> Option<usize> {
    s[i..].starts_with(word).then_some(i + word.len())
}

fn string_end(s: &[u8], mut i: usize) -> Option<usize> {
    i += 1;
    loop {
        match *s.get(i)? {
            b'"' => return Some(i + 1),
            b'\\' => match *s.get(i + 1)? {
                b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => i += 2,
                b'u' if s.get(i + 2..i + 6)?.iter().all(u8::is_ascii_hexdigit) => i += 6,
                _ => return None,
            },
            0..=0x1F => return None,
            _ => i += 1,
        }
    }
}

fn digits_end(s: &[u8], mut i: usize) -> usize {
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
    }
    i
}

fn number_end(s: &[u8], mut i: usize) -> Option<usize> {
    if s[i] == b'-' {
        i += 1;
    }
    match *s.get(i)? {
        b'0' => i += 1,
        b'1'..=b'9' => i = digits_end(s, i),
        _ => return None,
    }
    if s.get(i) == Some(&b'.') {
        let end = digits_end(s, i + 1);
        if end == i + 1 {
            return None;
        }
        i = end;
    }
    if matches!(s.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(s.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let end = digits_end(s, i);
        if end == i {
            return None;
        }
        i = end;
    }
    Some(i)
}

/// The elements of the array or object at i (only when `collect`, so that
/// checking a value allocates nothing), and its end.
fn entries(s: &[u8], i: usize, depth: usize, collect: bool) -> Option<(Vec<Entry>, usize)> {
    if depth >= MAX_DEPTH {
        return None;
    }
    let object = s[i] == b'{';
    let close = if object { b'}' } else { b']' };
    let mut found = Vec::new();
    let mut j = skip_space(s, i + 1);
    if s.get(j) == Some(&close) {
        return Some((found, j + 1));
    }
    loop {
        let key = if object {
            if s.get(j) != Some(&b'"') {
                return None;
            }
            let end = string_end(s, j)?;
            let key = (j, end);
            j = skip_space(s, end);
            if s.get(j) != Some(&b':') {
                return None;
            }
            j = skip_space(s, j + 1);
            Some(key)
        } else {
            None
        };
        let end = value_end(s, j, depth + 1)?;
        if collect {
            found.push(Entry { key, value: (j, end) });
        }
        j = skip_space(s, end);
        match *s.get(j)? {
            b',' => j = skip_space(s, j + 1),
            c if c == close => return Some((found, j + 1)),
            _ => return None,
        }
    }
}

/// The value that the whole text is, without the white space around it.
fn whole(s: &[u8]) -> Option<Span> {
    let start = skip_space(s, 0);
    let end = value_end(s, start, 0)?;
    (skip_space(s, end) == s.len()).then_some((start, end))
}

fn elements(s: &[u8], v: Span) -> Option<Vec<Entry>> {
    matches!(s[v.0], b'{' | b'[').then(|| entries(s, v.0, 0, true).map(|(e, _)| e))?
}

/// The value of a key in an object; the last one when it occurs twice.
fn get(s: &[u8], v: Span, key: &[u8]) -> Option<Span> {
    if s[v.0] != b'{' {
        return None;
    }
    let found = elements(s, v)?;
    let entry = found.iter().rev().find(|e| e.key.is_some_and(|k| unescape(&s[k.0..k.1]) == key));
    entry.map(|e| e.value)
}

fn nth(s: &[u8], v: Span, n: Cell) -> Option<Entry> {
    let n = usize::try_from(n).ok()?;
    elements(s, v)?.into_iter().nth(n)
}

/// Follows a path such as `users[0].name` or `["a key"].x`.
fn follow(s: &[u8], mut v: Span, path: &[u8]) -> Option<Span> {
    let mut i = 0;
    while i < path.len() {
        match path[i] {
            b'.' => i += 1,
            b'[' if path.get(i + 1) == Some(&b'"') => {
                let len = path[i + 2..].windows(2).position(|w| w == b"\"]")?;
                v = get(s, v, &path[i + 2..i + 2 + len])?;
                i += len + 4;
            }
            b'[' => {
                let len = path[i + 1..].iter().position(|&c| c == b']')?;
                let n = std::str::from_utf8(&path[i + 1..i + 1 + len]).ok()?.parse().ok()?;
                v = nth(s, v, n)?.value;
                i += len + 2;
            }
            _ => {
                let len = path[i..].iter().position(|&c| c == b'.' || c == b'[');
                let end = len.map_or(path.len(), |len| i + len);
                v = get(s, v, &path[i..end])?;
                i = end;
            }
        }
    }
    Some(v)
}

fn hex4(s: &[u8]) -> u32 {
    std::str::from_utf8(s).ok().and_then(|h| u32::from_str_radix(h, 16).ok()).unwrap_or(0xFFFD)
}

/// The text of a valid JSON string, with its quotes, without the escapes.
fn unescape(s: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let (mut i, end) = (1, s.len() - 1);
    while i < end {
        if s[i] != b'\\' {
            out.push(s[i]);
            i += 1;
            continue;
        }
        let c = s[i + 1];
        i += 2;
        match c {
            b'b' => out.push(8),
            b'f' => out.push(12),
            b'n' => out.push(b'\n'),
            b'r' => out.push(b'\r'),
            b't' => out.push(b'\t'),
            b'u' => {
                let mut code = hex4(&s[i..i + 4]);
                i += 4;
                // A surrogate pair is one character.
                if (0xD800..0xDC00).contains(&code) && s[i..end].starts_with(b"\\u") {
                    let low = hex4(&s[i + 2..i + 6]);
                    if (0xDC00..0xE000).contains(&low) {
                        code = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                        i += 6;
                    }
                }
                let c = char::from_u32(code).unwrap_or(char::REPLACEMENT_CHARACTER);
                out.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes());
            }
            other => out.push(other), // " \ /
        }
    }
    out
}

/// Text as a JSON string, with quotes; bytes that are no UTF-8 become U+FFFD.
fn quoted(text: &[u8]) -> Vec<u8> {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in String::from_utf8_lossy(text).chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out.into_bytes()
}

/// The writers: the output, and a string for each <JSON not yet closed.
#[derive(Default)]
pub(crate) struct Writers {
    output: Writer,
    strings: Vec<Writer>,
}

#[derive(Default)]
struct Writer {
    text: Vec<u8>,   // unused for the output
    open: Vec<Open>, // the arrays and objects not yet closed
}

struct Open {
    object: bool,
    items: usize,
    key: bool, // in an object: a key was written, its value comes next
}

fn mismatch(what: &str) -> Throw {
    Throw::with(CONTROL_MISMATCH, what)
}

impl Writers {
    /// What the writers take outside Forth memory.
    fn space(&self) -> usize {
        let strings = self.strings.iter().map(|w| w.text.len()).sum::<usize>();
        let open = self.strings.iter().chain([&self.output]).map(|w| w.open.len()).sum::<usize>();
        strings + open * std::mem::size_of::<Open>()
    }

    fn current(&mut self) -> &mut Writer {
        match self.strings.last_mut() {
            Some(w) => w,
            None => &mut self.output,
        }
    }
}

impl Writer {
    /// Before a value: the comma it needs, if any.
    fn value(&mut self) -> Res<&'static [u8]> {
        match self.open.last_mut() {
            None => Ok(b""),
            Some(o) if o.object => {
                if !o.key {
                    return Err(mismatch("a value in a JSON object needs JSON-KEY first"));
                }
                o.key = false;
                Ok(b"")
            }
            Some(o) => {
                o.items += 1;
                Ok(if o.items > 1 { b"," } else { b"" })
            }
        }
    }

    fn key(&mut self) -> Res<&'static [u8]> {
        match self.open.last_mut() {
            Some(o) if o.object && !o.key => {
                o.items += 1;
                o.key = true;
                Ok(if o.items > 1 { b"," } else { b"" })
            }
            _ => Err(mismatch("JSON-KEY outside a JSON object, or twice")),
        }
    }

    fn close(&mut self, object: bool) -> Res {
        match self.open.last() {
            Some(o) if o.object == object && !o.key => {
                self.open.pop();
                Ok(())
            }
            _ => Err(mismatch(if object {
                "JSON} without its JSON{"
            } else {
                "JSON] without its JSON["
            })),
        }
    }
}

impl Forth {
    /// What the JSON writers take outside Forth memory (`Limits::space`).
    pub(crate) fn json_space(&self) -> usize {
        self.peek_state::<Writers>().map_or(0, Writers::space)
    }

    /// Writes JSON text to the current writer: a string, or the output. The
    /// strings of all <JSON together cannot be larger than the memory.
    fn write_json(&mut self, text: &[u8]) -> Res {
        let room = self.mem.len();
        let writers = self.state::<Writers>();
        let written = writers.strings.iter().map(|w| w.text.len()).sum::<usize>();
        match writers.strings.last_mut() {
            Some(_) if written + text.len() > room => {
                Err(Throw::with(ALLOCATE_FAILED, "the JSON strings do not fit in memory"))
            }
            Some(w) => {
                w.text.extend_from_slice(text);
                self.check_space()
            }
            None => {
                self.emit_bytes(text);
                Ok(())
            }
        }
    }

    /// Writes a value: its comma, if any, then its text.
    fn write_value(&mut self, text: &[u8]) -> Res {
        let comma = self.state::<Writers>().current().value()?;
        self.write_json(comma)?;
        self.write_json(text)
    }

    fn open_json(&mut self, object: bool) -> Res {
        if self.state::<Writers>().current().open.len() >= MAX_DEPTH {
            return Err(mismatch("JSON nested deeper than 512"));
        }
        self.write_value(if object { b"{" } else { b"[" })?;
        self.state::<Writers>().current().open.push(Open { object, items: 0, key: false });
        Ok(())
    }

    fn close_json(&mut self, object: bool) -> Res {
        self.state::<Writers>().current().close(object)?;
        self.write_json(if object { b"}" } else { b"]" })
    }

    /// Forgets what was being written, after an error.
    pub(crate) fn reset_json(&mut self) {
        self.forget_state::<Writers>();
    }

    /// Pops a JSON value ( c-addr u ): where it is in memory, if it is one.
    /// Reading it is work in proportion to its length.
    fn pop_json(&mut self) -> Res<Option<Span>> {
        let (a, n) = self.pop_str()?;
        self.charge(n)?;
        Ok(whole(&self.mem[a..a + n]).map(|(s, e)| (a + s, a + e)))
    }

    /// Charges reading a value through once more, `times` times.
    fn charge_again(&mut self, v: Option<Span>, times: usize) -> Res {
        self.charge(v.map_or(0, |(s, e)| (e - s) * times))
    }

    /// Pushes a piece of a JSON value and a flag, or 0 0 0.
    fn push_span(&mut self, span: Option<Span>) {
        let (a, n, found) = span.map_or((0, 0, 0), |(s, e)| (s, e - s, -1));
        self.push(a as Cell);
        self.push(n as Cell);
        self.push(found);
    }

    /// Pushes an unescaped string, `ALLOCATE`d, and a flag; or 0 0 0.
    fn push_unescaped(&mut self, string: Option<Span>) -> Res {
        match string {
            Some((s, e)) => {
                let text = unescape(&self.mem[s..e]);
                self.push_bytes(&text)?;
                self.push(-1);
            }
            None => self.push_span(None),
        }
        Ok(())
    }
}

/// The text of a value, when it is of the kind that starts with `first`.
fn kind(mem: &[u8], v: Option<Span>, first: impl Fn(u8) -> bool) -> Option<Span> {
    v.filter(|&(s, _)| first(mem[s]))
}

fn word(f: &mut Forth, wid: Cell, name: &str, word: impl Fn(&mut Forth) -> Res + 'static) {
    f.add_word(wid, name, word).expect("JSON is a word list and names are not empty");
}

impl Extension for Json {
    fn name(&self) -> &'static str {
        "FORTHLING-JSON"
    }

    fn install(&self, f: &mut Forth) {
        let wid = f.wordlist("JSON");
        // Reading: a JSON value is ( c-addr u ); a piece of it is too.
        word(f, wid, "JSON-VALID?", |f| {
            let v = f.pop_json()?;
            f.push(flag(v.is_some()));
            Ok(())
        });
        // JSON-TYPE ( json -- n ): 0 null, 1 boolean, 2 number, 3 string,
        // 4 array, 5 object, -1 no valid JSON.
        word(f, wid, "JSON-TYPE", |f| {
            let v = f.pop_json()?;
            let n = v.map_or(-1, |(s, _)| match f.mem[s] {
                b'n' => 0,
                b't' | b'f' => 1,
                b'"' => 3,
                b'[' => 4,
                b'{' => 5,
                _ => 2,
            });
            f.push(n);
            Ok(())
        });
        // JSON-GET ( json c-addr u -- json' flag ): the value of a key.
        word(f, wid, "JSON-GET", |f| {
            let key = f.pop_bytes()?;
            let v = f.pop_json()?;
            f.charge_again(v, 1)?;
            let found = v.and_then(|v| get(&f.mem, v, &key));
            f.push_span(found);
            Ok(())
        });
        // JSON-NTH ( json n -- json' flag ): element n (from 0) of an array,
        // or the value of entry n of an object.
        word(f, wid, "JSON-NTH", |f| {
            let n = f.pop()?;
            let v = f.pop_json()?;
            f.charge_again(v, 1)?;
            let found = v.and_then(|v| nth(&f.mem, v, n)).map(|e| e.value);
            f.push_span(found);
            Ok(())
        });
        // JSON-NAME ( json n -- c-addr u flag ): the key of entry n of an
        // object, ALLOCATEd.
        word(f, wid, "JSON-NAME", |f| {
            let n = f.pop()?;
            let v = f.pop_json()?;
            f.charge_again(v, 1)?;
            let key = v.and_then(|v| nth(&f.mem, v, n)).and_then(|e| e.key);
            f.push_unescaped(key)
        });
        // JSON-LENGTH ( json -- n ): the elements of an array or object; -1
        // for anything else.
        word(f, wid, "JSON-LENGTH", |f| {
            let v = f.pop_json()?;
            f.charge_again(v, 1)?;
            let n = v.and_then(|v| elements(&f.mem, v)).map_or(-1, |e| e.len() as Cell);
            f.push(n);
            Ok(())
        });
        // JSON@ ( json c-addr u -- json' flag ): the value at a path, such as
        // users[0].name, or ["a key"] for a key with a dot in it.
        word(f, wid, "JSON@", |f| {
            let path = f.pop_bytes()?;
            let v = f.pop_json()?;
            let steps = path.iter().filter(|&&c| c == b'.' || c == b'[').count() + 1;
            f.charge_again(v, steps)?;
            let found = v.and_then(|v| follow(&f.mem, v, &path));
            f.push_span(found);
            Ok(())
        });
        // JSON>NUMBER ( json -- n flag ): an integer that fits in a cell.
        word(f, wid, "JSON>NUMBER", |f| {
            let v = f.pop_json()?;
            let number = kind(&f.mem, v, |c| c == b'-' || c.is_ascii_digit());
            let n = number.and_then(|(s, e)| std::str::from_utf8(&f.mem[s..e]).ok()?.parse().ok());
            f.push(n.unwrap_or(0));
            f.push(flag(n.is_some()));
            Ok(())
        });
        // JSON>FLOAT ( json -- flag ) ( F: -- r | ): any number.
        word(f, wid, "JSON>FLOAT", |f| {
            let v = f.pop_json()?;
            let number = kind(&f.mem, v, |c| c == b'-' || c.is_ascii_digit());
            match number.and_then(|(s, e)| std::str::from_utf8(&f.mem[s..e]).ok()?.parse().ok()) {
                Some(r) => {
                    f.fpush(r)?;
                    f.push(-1);
                }
                None => f.push(0),
            }
            Ok(())
        });
        // JSON>FLAG ( json -- flag ok ): true or false.
        word(f, wid, "JSON>FLAG", |f| {
            let v = f.pop_json()?;
            let b = kind(&f.mem, v, |c| c == b't' || c == b'f').map(|(s, _)| f.mem[s] == b't');
            f.push(flag(b == Some(true)));
            f.push(flag(b.is_some()));
            Ok(())
        });
        // JSON>STRING ( json -- c-addr u flag ): a string without its
        // escapes, ALLOCATEd.
        word(f, wid, "JSON>STRING", |f| {
            let v = f.pop_json()?;
            let string = kind(&f.mem, v, |c| c == b'"');
            f.push_unescaped(string)
        });

        // Writing, to the output or between <JSON and JSON> to a string.
        word(f, wid, "JSON{", |f| f.open_json(true));
        word(f, wid, "JSON}", |f| f.close_json(true));
        word(f, wid, "JSON[", |f| f.open_json(false));
        word(f, wid, "JSON]", |f| f.close_json(false));
        // JSON-KEY ( c-addr u -- ): the key of the next value in an object.
        word(f, wid, "JSON-KEY", |f| {
            let key = f.pop_bytes()?;
            let comma = f.state::<Writers>().current().key()?;
            f.write_json(comma)?;
            f.write_json(&quoted(&key))?;
            f.write_json(b":")
        });
        // JSON-STRING ( c-addr u -- )
        word(f, wid, "JSON-STRING", |f| {
            let text = f.pop_bytes()?;
            f.charge(text.len())?;
            f.write_value(&quoted(&text))
        });
        // JSON-NUMBER ( n -- ), always in decimal.
        word(f, wid, "JSON-NUMBER", |f| {
            let n = f.pop()?;
            f.write_value(n.to_string().as_bytes())
        });
        // JSON-FLOAT ( F: r -- ); null for an infinity or a NaN, which JSON
        // does not have.
        word(f, wid, "JSON-FLOAT", |f| {
            let r = f.fpop()?;
            let text = if r.is_finite() { format!("{r:?}") } else { "null".into() };
            f.write_value(text.as_bytes())
        });
        // JSON-FLAG ( flag -- ): true unless 0.
        word(f, wid, "JSON-FLAG", |f| {
            let b = f.pop()? != 0;
            f.write_value(if b { b"true" } else { b"false" })
        });
        word(f, wid, "JSON-NULL", |f| f.write_value(b"null"));
        // JSON-RAW ( json -- ): a JSON value as it is; -24 if it is none.
        word(f, wid, "JSON-RAW", |f| {
            let v = f.pop_json()?;
            let (s, e) = v.ok_or_else(|| Throw::with(INVALID_ARGUMENT, "JSON-RAW needs JSON"))?;
            let text = f.mem[s..e].to_vec();
            f.write_value(&text)
        });
        // <JSON ( -- ) and JSON> ( -- c-addr u ): what is written in between
        // becomes a string, ALLOCATEd; they can be nested.
        word(f, wid, "<JSON", |f| {
            let strings = &mut f.state::<Writers>().strings;
            if strings.len() >= MAX_DEPTH {
                return Err(mismatch("<JSON nested deeper than 512"));
            }
            strings.push(Writer::default());
            Ok(())
        });
        word(f, wid, "JSON>", |f| {
            let w = f
                .state::<Writers>()
                .strings
                .pop()
                .ok_or_else(|| mismatch("JSON> without <JSON"))?;
            if !w.open.is_empty() {
                return Err(mismatch("JSON> before the arrays and objects are closed"));
            }
            f.charge(w.text.len())?;
            f.push_bytes(&w.text)
        });
    }
}
