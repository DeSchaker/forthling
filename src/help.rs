//! HELP: the stack effect and a line of explanation for every word.
//!
//! The text is data, not code: `help.txt` has one line per word, in sections
//! named after the word list (`[FORTH]`, `[JSON]`). A host that adds words of
//! its own adds the text of them with `Forth::add_help`. The tests walk every
//! word list and fail when a word has no line, and run the examples, so the
//! help cannot drift from the words.
//!
//! A line is `NAME ( effect ) description ;; code => output`: the part from
//! `;;` is optional, and `output` is what `code` prints.

use crate::*;
use std::fmt::Write as _;

/// What the system can say about a word; see `Forth::help`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Help {
    /// The name as given.
    pub name: String,
    /// Whether the word exists (in any word list).
    pub found: bool,
    /// The word list it is in, if the list has a name.
    pub list: Option<String>,
    /// Whether the search order finds it.
    pub visible: bool,
    /// Whether the program defined it, as opposed to the system or an extension.
    pub user: bool,
    /// The stack effect, such as `( x1 x2 -- x2 x1 )`.
    pub effect: Option<String>,
    /// What the word does.
    pub description: Option<String>,
    /// Code that shows the word at work.
    pub example: Option<String>,
    /// What the example prints.
    pub output: Option<String>,
    /// Words with a name like it, when the word is not found.
    pub close: Vec<String>,
}

/// An example of the help text with the output it should give; see `Forth::help_examples`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct HelpExample {
    pub list: String,
    pub name: String,
    pub code: String,
    pub output: String,
}

/// One line of a help file.
struct Entry<'a> {
    list: &'a str,
    name: &'a str,
    effect: &'a str,
    description: &'a str,
    example: Option<(&'a str, &'a str)>,
}

/// The entries of the help texts, in order; a line that is not an entry is skipped.
fn entries<'a>(texts: &'a [&'static str]) -> impl Iterator<Item = Entry<'a>> {
    texts.iter().flat_map(|text| {
        let mut list = "FORTH";
        text.lines().filter_map(move |line| {
            let line = line.trim();
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                list = name;
                return None;
            }
            parse_entry(list, line)
        })
    })
}

fn parse_entry<'a>(list: &'a str, line: &'a str) -> Option<Entry<'a>> {
    if line.starts_with("//") {
        return None;
    }
    let (name, rest) = line.split_once(' ')?;
    if !rest.starts_with("( ") {
        return None;
    }
    // The effect is one group `( ... )`, or more: `( x -- ) ( F: -- r )`.
    let mut end = group_end(rest)?;
    while rest[end..].trim_start().starts_with("( ") {
        end += rest[end..].len() - rest[end..].trim_start().len();
        end += group_end(&rest[end..])?;
    }
    let effect = &rest[..end];
    let (description, example) = match rest[end..].split_once(" ;; ") {
        Some((d, e)) => (d, e.split_once(" => ")),
        None => (&rest[end..], None),
    };
    Some(Entry { list, name, effect, description: description.trim(), example })
}

/// Where the group `( ... )` at the start of `s` ends: after the first ` )` that
/// is followed by a space or the end.
fn group_end(s: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(i) = s[from..].find(" )") {
        let end = from + i + 2;
        if end == s.len() || s[end..].starts_with(' ') {
            return Some(end);
        }
        from = end;
    }
    None
}

impl Forth {
    /// Adds the help text of words that a host added: lines of the form
    /// `NAME ( effect ) description ;; code => output`, under `[WORDLIST]`
    /// headings. Without the `help` feature of the crate this does nothing.
    pub fn add_help(&mut self, text: &'static str) {
        if cfg!(feature = "help") {
            self.help.push(text);
        }
    }

    /// What is known about a word: its stack effect and what it does (from the
    /// help text, or for a word of the program from the comment after its
    /// name), or, when there is no such word, the names that are close.
    #[must_use]
    pub fn help(&self, name: &str) -> Help {
        let name = name.trim();
        let mut h = Help { name: name.to_string(), ..Help::default() };
        let upper = name.to_ascii_uppercase();
        let xt = self.find(&upper).or_else(|| {
            let xts = self.index.get(&upper)?;
            xts.iter().rev().copied().find(|&xt| !self.dict[xt].hidden)
        });
        let Some(xt) = xt else {
            h.close = self.close_names(&upper);
            return h;
        };
        let w = &self.dict[xt];
        h.found = true;
        h.visible = self.find(&upper) == Some(xt);
        h.list = self.wordlists.get(w.wid.wrapping_sub(1)).cloned().flatten();
        h.user = xt >= self.fence;
        if h.user {
            h.effect = w.effect.as_deref().map(str::to_string);
            return h;
        }
        let list = h.list.as_deref().unwrap_or("");
        if let Some(e) = entries(&self.help).find(|e| e.list == list && e.name == upper) {
            h.effect = Some(e.effect.to_string());
            h.description = Some(e.description.to_string());
            if let Some((code, output)) = e.example {
                h.example = Some(code.to_string());
                h.output = Some(output.to_string());
            }
        }
        h
    }

    /// What HELP prints for a word.
    #[must_use]
    pub fn help_text(&self, name: &str) -> String {
        let h = self.help(name);
        let upper = h.name.to_ascii_uppercase();
        let mut s = String::new();
        if !h.found {
            let _ = writeln!(s, "{upper} is not defined.");
            if !h.close.is_empty() {
                let _ = writeln!(s, "Close: {}", h.close.join(" "));
            }
            return s;
        }
        match &h.effect {
            Some(effect) => _ = writeln!(s, "{upper} {effect}"),
            None => _ = writeln!(s, "{upper}"),
        }
        match (&h.description, h.user) {
            (Some(d), _) => _ = writeln!(s, "{d}"),
            (None, true) if h.effect.is_none() => {
                _ = writeln!(s, "Defined by the program, with no stack effect after its name.");
            }
            (None, true) => _ = writeln!(s, "Defined by the program."),
            (None, false) => _ = writeln!(s, "There is no help text for this word."),
        }
        if let (Some(code), Some(output)) = (&h.example, &h.output) {
            let _ = writeln!(s, "  {code}    \\ prints {output}");
        }
        if let Some(list) = h.list.as_deref().filter(|l| *l != "FORTH" && !h.visible) {
            let _ = writeln!(s, "It is in the word list {list}: ALSO {list} finds it.");
        }
        s
    }

    /// Problems in the help text, one line each, for the tests of a host: a
    /// word of the system or of an extension without a line, and a line
    /// for a word that is not in a word list that exists.
    #[must_use]
    pub fn help_problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let mut seen = HashSet::new();
        for w in &self.dict[..self.fence] {
            let list = self.wordlists.get(w.wid.wrapping_sub(1)).and_then(|l| l.as_deref());
            let Some(list) = list else { continue };
            if w.hidden || w.name.is_empty() || !seen.insert((list, w.name.as_str())) {
                continue;
            }
            if !entries(&self.help).any(|e| e.list == list && e.name == w.name) {
                problems.push(format!("[{list}] {} has no help line", w.name));
            }
        }
        for e in entries(&self.help) {
            let exists = |w: &Word| !w.hidden && w.name == e.name;
            // The word that picks the list of an extension that is not installed has no word.
            let picks_absent_list = e.list == "FORTH"
                && !self.wordlists.iter().any(|l| l.as_deref() == Some(e.name))
                && entries(&self.help).any(|o| o.list == e.name);
            if picks_absent_list {
                continue;
            }
            let listed = self.wordlists.iter().position(|l| l.as_deref() == Some(e.list));
            if let Some(i) = listed {
                if !self.dict[..self.fence].iter().any(|w| exists(w) && w.wid == i + 1) {
                    problems.push(format!("[{}] {} has a help line but no word", e.list, e.name));
                }
            }
        }
        problems
    }

    /// The examples of the help text, to run in a fresh system with the same
    /// extensions and compare with their output.
    #[must_use]
    pub fn help_examples(&self) -> Vec<HelpExample> {
        entries(&self.help)
            .filter_map(|e| {
                let (code, output) = e.example?;
                Some(HelpExample {
                    list: e.list.to_string(),
                    name: e.name.to_string(),
                    code: code.to_string(),
                    output: output.to_string(),
                })
            })
            .collect()
    }

    /// Up to eight names that are close to `upper` (upper case): the words
    /// that start with it or contain it, and those a letter or two away.
    pub(crate) fn close_names(&self, upper: &str) -> Vec<String> {
        let len = upper.chars().count();
        if len == 0 {
            return Vec::new();
        }
        let allowed = match len {
            0..=2 => 0,
            3..=4 => 1,
            _ => 2,
        };
        let mut found: Vec<(usize, &str)> = Vec::new();
        let mut seen = HashSet::new();
        for w in self.dict.iter().rev() {
            if w.hidden || w.name.is_empty() || w.name == upper || !seen.insert(w.name.as_str()) {
                continue;
            }
            let name = w.name.as_str();
            let distance = edit_distance(upper, name);
            let rank = if distance <= allowed {
                distance
            } else if len >= 2 && name.starts_with(upper) {
                allowed + 1
            } else if len >= 3 && name.contains(upper) {
                allowed + 2
            } else {
                continue;
            };
            found.push((rank, name));
        }
        found.sort_unstable();
        found.into_iter().take(8).map(|(_, name)| name.to_string()).collect()
    }

    /// After `: NAME`, keeps the comment `( ... )` that follows on the same
    /// line as the stack effect of the new word, for HELP.
    pub(crate) fn note_effect(&mut self, xt: usize) -> Res {
        let saved = self.get(IN_ADDR);
        let (a, n) = self.parse_name();
        if n == 1 && self.mem[a] == b'(' {
            let (a, n) = self.parse(b')');
            if a + n < self.parse_area().1 {
                let text = self.text(a, n);
                let text = text.trim();
                let effect: String = format!("( {text} )").chars().take(120).collect();
                self.dict[xt].size += effect.len();
                self.space += effect.len();
                self.dict[xt].effect = Some(Rc::from(effect));
                return self.check_space();
            }
        }
        self.set(IN_ADDR, saved);
        Ok(())
    }
}

/// The number of letters to change, add or remove to turn `a` into `b`.
fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = (diagonal + usize::from(ca != cb)).min(above + 1).min(row[j] + 1);
            diagonal = above;
        }
    }
    row[b.len()]
}

pub(crate) fn install(f: &mut Forth) {
    // HELP ( "name" -- ): the stack effect of a word and what it does.
    f.prim("HELP", |f| {
        let (a, n) = f.parse_name();
        let text = if n == 0 {
            "HELP name shows the stack effect of a word and what it does.\n\
             WORDS lists the words; SEE shows how a word is made.\n"
                .to_string()
        } else {
            let name = f.text(a, n);
            f.help_text(&name)
        };
        f.emit_str(&text);
        Ok(())
    });
}
