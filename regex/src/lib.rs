//! Regular expressions for forthling: the word list REGEX. Patterns are
//! matched in time linear in the text (no catastrophic backtracking), which
//! the sandbox needs, and touch nothing outside Forth, so no permission is
//! needed. Strings that a word returns are `ALLOCATE`d, for the program to
//! FREE; positions are byte offsets into the text.
//!
//! ```
//! use forthling::Forth;
//!
//! let mut f = Forth::new();
//! f.install(&forthling_regex::Regex);
//! f.interpret("ALSO REGEX  S\" order 66\" S\" [0-9]+\" RE-FIND . . .")?;
//! assert_eq!(f.output(), b"-1 2 6 ");
//! # Ok::<(), forthling::Throw>(())
//! ```
//!
//! The syntax is that of the `regex-lite` crate: Perl-like, without the
//! Unicode classes such as `\p{L}`.

use forthling::{Cell, Extension, Forth, Res, Throw, to_cell};
use regex_lite::{Regex as Compiled, RegexBuilder};
use std::collections::HashMap;

/// The throw of a pattern that does not compile, or of a text that is not
/// UTF-8, or of a result that would be larger than the Forth memory.
pub const REGEX_ERROR: Cell = -262;

/// The largest compiled pattern, in bytes.
const SIZE_LIMIT: usize = 1 << 20;
/// How deep a pattern may nest its groups.
const NEST_LIMIT: u32 = 100;
/// The patterns kept compiled; the cache starts again when it is full.
const CACHE: usize = 32;

/// The extension: `forth.install(&Regex)`.
pub struct Regex;

#[derive(Default)]
struct State {
    cache: HashMap<String, Compiled>,
}

impl State {
    fn compile(&mut self, pattern: &str) -> Res<Compiled> {
        if let Some(re) = self.cache.get(pattern) {
            return Ok(re.clone());
        }
        let re = RegexBuilder::new(pattern)
            .size_limit(SIZE_LIMIT)
            .nest_limit(NEST_LIMIT)
            .build()
            .map_err(|e| Throw::with(REGEX_ERROR, format!("bad pattern {pattern}: {e}")))?;
        if self.cache.len() >= CACHE {
            self.cache.clear();
        }
        self.cache.insert(pattern.to_string(), re.clone());
        Ok(re)
    }
}

fn utf8(bytes: Vec<u8>, what: &str) -> Res<String> {
    String::from_utf8(bytes)
        .map_err(|_| Throw::with(REGEX_ERROR, format!("the {what} is not UTF-8")))
}

/// Pops a pattern ( c-addr u ) and compiles it.
fn pop_pattern(f: &mut Forth) -> Res<Compiled> {
    let pattern = utf8(f.pop_bytes()?, "pattern")?;
    f.state::<State>().compile(&pattern)
}

/// Pops a text ( c-addr u ), and counts its length as work.
fn pop_text(f: &mut Forth) -> Res<String> {
    let text = utf8(f.pop_bytes()?, "text")?;
    f.charge(text.len())?;
    Ok(text)
}

fn flag(b: bool) -> Cell {
    if b { -1 } else { 0 }
}

/// Pushes ( start len true ) or ( false ).
fn push_span(f: &mut Forth, span: Option<(usize, usize)>) {
    match span {
        Some((start, end)) => {
            f.push(to_cell(start));
            f.push(to_cell(end - start));
            f.push(-1);
        }
        None => f.push(0),
    }
}

/// A word of the REGEX word list; the word list exists once installed.
fn word(f: &mut Forth, name: &str, word: impl Fn(&mut Forth) -> Res + 'static) {
    let wid = f.wordlist("REGEX");
    f.add_word(wid, name, word).expect("REGEX is a word list and names are not empty");
}

impl Extension for Regex {
    fn name(&self) -> &'static str {
        "FORTHLING-REGEX"
    }

    fn install(&self, f: &mut Forth) {
        // RE-MATCH? ( c-addr u re-addr re-u -- flag ): is there a match
        // anywhere in the text? Anchor the pattern with ^ and $ to match
        // all of it.
        word(f, "RE-MATCH?", |f| {
            let re = pop_pattern(f)?;
            let text = pop_text(f)?;
            f.push(flag(re.is_match(&text)));
            Ok(())
        });
        // RE-FIND ( c-addr u re-addr re-u -- start len true | false ): the
        // first match, as a byte offset and a length in the text.
        word(f, "RE-FIND", |f| {
            let re = pop_pattern(f)?;
            let text = pop_text(f)?;
            let span = re.find(&text).map(|m| (m.start(), m.end()));
            push_span(f, span);
            Ok(())
        });
        // RE-GROUP ( c-addr u re-addr re-u n -- start len true | false ):
        // group n of the first match (0 is the whole match, 1 the first
        // group in parentheses); false when there is no match or the group
        // took no part in it.
        word(f, "RE-GROUP", |f| {
            let n = f.pop()?;
            let re = pop_pattern(f)?;
            let text = pop_text(f)?;
            let group = usize::try_from(n).ok().filter(|&n| n < re.captures_len());
            let Some(group) = group else {
                return Err(Throw::with(REGEX_ERROR, format!("the pattern has no group {n}")));
            };
            let span =
                re.captures(&text).and_then(|caps| caps.get(group)).map(|m| (m.start(), m.end()));
            push_span(f, span);
            Ok(())
        });
        // RE-COUNT ( c-addr u re-addr re-u -- n ): the matches that do not
        // overlap.
        word(f, "RE-COUNT", |f| {
            let re = pop_pattern(f)?;
            let text = pop_text(f)?;
            f.push(to_cell(re.find_iter(&text).count()));
            Ok(())
        });
        // RE-REPLACE ( c-addr u re-addr re-u rep-addr rep-u -- c-addr' u' ):
        // replaces every match by the replacement, in which $1 is group 1,
        // ${name} a named group and $$ a dollar; the new text is ALLOCATEd.
        word(f, "RE-REPLACE", |f| {
            let replacement = utf8(f.pop_bytes()?, "replacement")?;
            let re = pop_pattern(f)?;
            let text = pop_text(f)?;
            let room = f.memory().len();
            let mut out = String::new();
            let mut last = 0;
            for caps in re.captures_iter(&text) {
                let whole = caps.get(0).expect("group 0 is the whole match");
                out.push_str(&text[last..whole.start()]);
                caps.expand(&replacement, &mut out);
                last = whole.end();
                if out.len() > room {
                    return Err(Throw::with(
                        REGEX_ERROR,
                        "the result of the replacement is larger than the Forth memory",
                    ));
                }
            }
            out.push_str(&text[last..]);
            f.charge(out.len())?;
            f.push_bytes(out.as_bytes())
        });
    }
}
