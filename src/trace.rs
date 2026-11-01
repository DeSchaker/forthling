//! Tracing: a line for each word that runs, with how deep it is nested and
//! what the data stack holds when it starts. The program switches it on with
//! TRACE-ON in the word list DEBUG (the extension `Debugger`), and the lines go
//! to the output; a host can record the same as data with `Forth::record_trace`.
//! When neither is on, it costs one test per word.

use crate::*;
use std::fmt::Write as _;

/// The cells of the data stack that a trace line shows, from the top.
pub const TRACE_CELLS: usize = 8;

/// How far a trace line is indented at most: two spaces per level of
/// nesting, up to this many levels.
const TRACE_INDENT: usize = 16;

/// One word in a trace.
#[derive(Clone, Debug, PartialEq)]
pub struct TraceEntry {
    /// The name of the word.
    pub word: String,
    /// How deep it is nested: 0 for a word of the input, 1 for a word that
    /// one of those calls, and so on.
    pub depth: usize,
    /// The data stack when the word starts, the top last; at most the top
    /// `TRACE_CELLS` cells.
    pub stack: Vec<Cell>,
    /// The number of cells on the data stack then, also those not shown.
    pub stack_depth: usize,
}

/// What is being traced, if anything.
#[derive(Default)]
pub(crate) struct Tracer {
    print: bool,              // TRACE-ON: lines to the output
    record: Option<usize>,    // record_trace: at most this many entries
    entries: Vec<TraceEntry>, // recorded so far
    dropped: usize,           // entries past the maximum
    off: Option<usize>,       // the xt of TRACE-OFF, which gets no line
}

/// The extension with the word list DEBUG: `TRACE-ON`, `TRACE-OFF` and
/// `BREAK` (see debug.rs).
/// ENVIRONMENT? knows it as FORTHLING-DEBUG.
pub struct Debugger;

impl Extension for Debugger {
    fn name(&self) -> &'static str {
        "FORTHLING-DEBUG"
    }

    fn install(&self, f: &mut Forth) {
        let wid = f.wordlist("DEBUG");
        let add = |f: &mut Forth, name, word: fn(&mut Forth) -> Res| {
            f.add_word(wid, name, word).expect("DEBUG is a word list and names are not empty");
        };
        // TRACE-ON ( -- ): from the next word on, print a trace line for each.
        add(f, "TRACE-ON", |f| {
            f.tracer.print = true;
            f.update_watching();
            Ok(())
        });
        // TRACE-OFF ( -- ): no more trace lines.
        add(f, "TRACE-OFF", |f| {
            f.tracer.print = false;
            f.update_watching();
            Ok(())
        });
        f.tracer.off = Some(f.dict.len() - 1);
        // BREAK ( -- ): stops the program for the host's debugger, if it has one.
        add(f, "BREAK", |f| f.pause(Reason::Break, "BREAK".to_string()));
    }
}

impl Forth {
    /// Starts recording a trace entry for each word that runs, at most `max`
    /// of them (those past it are counted, see `trace_dropped`), or stops
    /// with None. `take_trace` gives what was recorded. Recording does not
    /// print; TRACE-ON in the program does, independently.
    pub fn record_trace(&mut self, max: Option<usize>) {
        self.tracer.record = max;
        self.update_watching();
    }

    /// The trace recorded so far, which the system then forgets, as it
    /// forgets the number dropped.
    pub fn take_trace(&mut self) -> Vec<TraceEntry> {
        self.tracer.dropped = 0;
        std::mem::take(&mut self.tracer.entries)
    }

    /// How many entries did not fit in the maximum of `record_trace` since
    /// the last `take_trace`.
    #[must_use]
    pub fn trace_dropped(&self) -> usize {
        self.tracer.dropped
    }

    /// Whether `invoke` calls `watch` for each word: when tracing, or when
    /// the debugger is to stop at a coming word.
    pub(crate) fn update_watching(&mut self) {
        self.watching =
            self.tracer.print || self.tracer.record.is_some() || self.debug.stop.is_some();
    }

    /// The name of a word as a trace or a pause shows it.
    pub(crate) fn word_name(&self, xt: usize) -> Option<String> {
        match self.dict.get(xt) {
            Some(w) if w.name.is_empty() => Some(":NONAME".to_string()),
            Some(w) => Some(w.name.clone()),
            None => None,
        }
    }

    /// The word `xt` is about to run: traces it and lets the debugger stop
    /// at it. The insides of the system's own words (such as `/`, defined in
    /// Forth) are passed over, as are the immediate words that run while a
    /// definition is compiled, the system's immediate words in the input
    /// (such as \ ( [IF], which read the input rather than run the
    /// program), and TRACE-OFF.
    #[cold]
    #[inline(never)]
    pub(crate) fn watch(&mut self, xt: usize) -> Res {
        let caller = self.frames.last().map(|f| f.xt);
        let reads_input =
            caller.is_none() && xt < self.fence && self.dict.get(xt).is_some_and(|w| w.immediate);
        if caller.is_some_and(|c| c < self.fence)
            || caller.is_none() && self.current.is_some()
            || reads_input
            || Some(xt) == self.tracer.off
        {
            return Ok(());
        }
        let Some(word) = self.word_name(xt) else {
            return Ok(());
        };
        if self.tracer.print || self.tracer.record.is_some() {
            self.trace(word.clone());
        }
        self.step_into(word)
    }

    /// Prints or records a trace line for `word`.
    fn trace(&mut self, word: String) {
        let depth = self.frames.len();
        let stack_depth = self.stack.len();
        let shown = stack_depth.saturating_sub(TRACE_CELLS);
        if self.tracer.print {
            // A trace line starts on a line of its own, also after `.`.
            let mut line = match self.out.last() {
                Some(&c) if c != b'\n' => "\n".to_string(),
                _ => String::new(),
            };
            line += &"  ".repeat(depth.min(TRACE_INDENT));
            let _ = write!(line, "{word} <{stack_depth}>");
            if shown > 0 {
                line += " ...";
            }
            for &n in &self.stack[shown..] {
                line.push(' ');
                line += &self.fmt_num(n);
            }
            line.push('\n');
            self.emit_bytes(line.as_bytes());
        }
        if let Some(max) = self.tracer.record {
            if self.tracer.entries.len() < max {
                let stack = self.stack[shown..].to_vec();
                let entry = TraceEntry { word, depth, stack, stack_depth };
                self.tracer.entries.push(entry);
            } else {
                self.tracer.dropped += 1;
            }
        }
    }
}
