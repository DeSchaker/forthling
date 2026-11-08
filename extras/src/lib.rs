//! Small building blocks for forthling, each a word list of its own: CSV, and
//! encodings and hashes now, dates, URLs and UUIDs to come. They only compute
//! on text in the Forth memory, with no dependencies, so they need no
//! permission and work in the sandbox, in the MCP server, in the Python module
//! and in the playground. Strings that a word returns are `ALLOCATE`d, for the
//! program to FREE.
//!
//! ```
//! use forthling::Forth;
//!
//! let mut f = Forth::new();
//! f.install(&forthling_extras::Extras);
//! f.interpret("ALSO CSV  S\" ab,c\" CSV-FIELD DROP TYPE 2DROP")?;
//! assert_eq!(f.output(), b"ab");
//! # Ok::<(), forthling::Throw>(())
//! ```

mod csv;
mod encoding;

use forthling::{Cell, Extension, Forth, Res};

/// The throw of text that is not valid CSV (an unterminated quote, text after
/// a closing quote) and of a separator that cannot be one.
pub const EXTRAS_ERROR: Cell = -265;

/// The extension: `forth.install(&Extras)`.
pub struct Extras;

/// A word of the word list `list`; the word list exists once installed.
fn word(f: &mut Forth, list: &str, name: &str, word: impl Fn(&mut Forth) -> Res + 'static) {
    let wid = f.wordlist(list);
    f.add_word(wid, name, word).expect("the list is a word list and names are not empty");
}

impl Extension for Extras {
    fn name(&self) -> &'static str {
        "FORTHLING-EXTRAS"
    }

    fn install(&self, f: &mut Forth) {
        f.add_help(include_str!("help.txt"));
        csv::install(f);
        encoding::install(f);
    }
}
