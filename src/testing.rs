//! Test words: the word list TESTING, installed as the extension `Testing`.
//! `T{ ... -> ... }T` is the syntax of the Forth-2012 test suite: the cells
//! the code before `->` leaves must equal the cells after it. A wrong result
//! is printed and counted, and the run goes on; the host takes the counts
//! with `Forth::take_test_results`. An error inside a test is not caught: it
//! ends the run as anywhere else, so the limits (-258) cannot be hidden.

use crate::*;
use std::fmt::Write as _;

/// The failures kept for the host; the others are only counted.
const MAX_FAILURES: usize = 32;
/// The cells kept of each list in a failure; the top ones.
const MAX_CELLS: usize = 32;

/// The extension: `forth.install(&Testing)`; ENVIRONMENT? knows it as
/// FORTHLING-TESTING.
pub struct Testing;

/// A test that gave other results than it should.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TestFailure {
    /// The input of the test, as in `ErrorContext::source`.
    pub source: String,
    /// The line of `T{`, from 1.
    pub line: usize,
    /// The text of the last `TESTING` before the test; empty if none.
    pub group: String,
    /// The cells after `->`, the top last (at most the top 32).
    pub expected: Vec<Cell>,
    /// How many cells there were after `->`.
    pub expected_depth: usize,
    /// The cells the code before `->` left, the top last (at most the top 32).
    pub actual: Vec<Cell>,
    /// How many cells the code left.
    pub actual_depth: usize,
    /// The stack below the test at that moment, the top last (at most the top 32).
    pub stack: Vec<Cell>,
}

/// What the tests since the last `Forth::take_test_results` found.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TestResults {
    pub passed: usize,
    pub failed: usize,
    /// The first 32 failures; `failed` counts them all.
    pub failures: Vec<TestFailure>,
}

/// The test in progress: set by `T{`, completed by `->`.
struct Open {
    start: usize, // the depth of the data stack at `T{`
    actual: Option<Vec<Cell>>,
    source: String,
    line: usize,
}

#[derive(Default)]
struct TestState {
    open: Option<Open>,
    group: String,
    results: TestResults,
}

impl Extension for Testing {
    fn name(&self) -> &'static str {
        "FORTHLING-TESTING"
    }

    fn install(&self, f: &mut Forth) {
        let wid = f.wordlist("TESTING");
        f.add_word(wid, "T{", test_start).expect("TESTING is a word list");
        f.add_word(wid, "->", test_arrow).expect("TESTING is a word list");
        f.add_word(wid, "}T", test_end).expect("TESTING is a word list");
        f.add_word(wid, "TESTING", test_group).expect("TESTING is a word list");
        f.add_word(wid, "TEST-SUMMARY", test_summary).expect("TESTING is a word list");
    }
}

impl Forth {
    /// The results of the `T{ ... -> ... }T` tests since the last call, and
    /// starts counting again. Without the extension `Testing` it is empty.
    pub fn take_test_results(&mut self) -> TestResults {
        let state = self
            .host_state
            .get_mut(&std::any::TypeId::of::<TestState>())
            .and_then(|s| s.downcast_mut::<TestState>());
        state.map_or_else(TestResults::default, |s| {
            s.open = None;
            std::mem::take(&mut s.results)
        })
    }
}

fn top(cells: &[Cell]) -> Vec<Cell> {
    cells[cells.len().saturating_sub(MAX_CELLS)..].to_vec()
}

/// `T{ ( -- )`: starts a test.
#[allow(clippy::unnecessary_wraps, reason = "a word body must return Res")]
fn test_start(f: &mut Forth) -> Res {
    let (source, line, ..) = f.place(f.first_line);
    let start = f.stack.len();
    f.state::<TestState>().open = Some(Open { start, actual: None, source, line });
    Ok(())
}

/// `-> ( ... -- )`: takes the cells since `T{` as the actual results.
fn test_arrow(f: &mut Forth) -> Res {
    let len = f.stack.len();
    let Some(open) = f.state::<TestState>().open.as_mut().filter(|o| o.actual.is_none()) else {
        return Err(Throw::with(CONTROL_MISMATCH, "-> without T{".to_string()));
    };
    open.start = open.start.min(len);
    let start = open.start;
    let actual = f.stack.split_off(start);
    if let Some(open) = f.state::<TestState>().open.as_mut() {
        open.actual = Some(actual);
    }
    Ok(())
}

/// `}T ( ... -- )`: takes the cells since `->` as the expected results and
/// compares.
fn test_end(f: &mut Forth) -> Res {
    let open = match f.state::<TestState>().open.take() {
        Some(o) if o.actual.is_some() => o,
        other => {
            f.state::<TestState>().open = other;
            return Err(Throw::with(CONTROL_MISMATCH, "}T without T{ ... ->".to_string()));
        }
    };
    let start = open.start.min(f.stack.len());
    let expected = f.stack.split_off(start);
    let actual = open.actual.unwrap_or_default();
    if expected == actual {
        f.state::<TestState>().results.passed += 1;
        return Ok(());
    }
    let failure = TestFailure {
        source: open.source,
        line: open.line,
        group: f.state::<TestState>().group.clone(),
        expected: top(&expected),
        expected_depth: expected.len(),
        actual: top(&actual),
        actual_depth: actual.len(),
        stack: top(&f.stack),
    };
    let text = failure_text(f, &failure);
    if f.out.last().is_some_and(|&b| b != b'\n') {
        f.emit_bytes(b"\n");
    }
    f.emit_bytes(text.as_bytes());
    let results = &mut f.state::<TestState>().results;
    results.failed += 1;
    if results.failures.len() < MAX_FAILURES {
        results.failures.push(failure);
    }
    Ok(())
}

/// The failure as one line (and a second for the stack below, if any), in
/// the base of the system.
fn failure_text(f: &Forth, x: &TestFailure) -> String {
    let cells = |cells: &[Cell], depth: usize| {
        let mut s = String::new();
        if depth > cells.len() {
            let _ = write!(s, "... ({} more) ", depth - cells.len());
        }
        let list: Vec<String> = cells.iter().map(|&c| f.fmt_num(c)).collect();
        s + &list.join(" ")
    };
    let group = if x.group.is_empty() { String::new() } else { format!(" ({})", x.group) };
    let kind = if x.expected_depth == x.actual_depth {
        "incorrect result"
    } else {
        "wrong number of results"
    };
    let mut text = format!(
        "FAIL {}:{}{group}: {kind}: expected <{}> {}, got <{}> {}\n",
        x.source,
        x.line,
        x.expected_depth,
        cells(&x.expected, x.expected_depth),
        x.actual_depth,
        cells(&x.actual, x.actual_depth),
    );
    if !x.stack.is_empty() {
        let _ = writeln!(text, "  stack below: {}", cells(&x.stack, x.stack.len()));
    }
    text
}

/// `TESTING ( "text" -- )`: names the tests that follow; the text is the rest
/// of the line.
#[allow(clippy::unnecessary_wraps, reason = "a word body must return Res")]
fn test_group(f: &mut Forth) -> Res {
    let (a, n) = f.parse(b'\n');
    let text = String::from_utf8_lossy(&f.mem[a..a + n]).trim().to_string();
    f.state::<TestState>().group = text;
    Ok(())
}

/// `TEST-SUMMARY ( -- )`: prints how many tests passed and failed since the
/// results were last taken.
#[allow(clippy::unnecessary_wraps, reason = "a word body must return Res")]
fn test_summary(f: &mut Forth) -> Res {
    let results = &f.state::<TestState>().results;
    let text = format!("tests: {} passed, {} failed\n", results.passed, results.failed);
    if f.out.last().is_some_and(|&b| b != b'\n') {
        f.emit_bytes(b"\n");
    }
    f.emit_bytes(text.as_bytes());
    Ok(())
}
