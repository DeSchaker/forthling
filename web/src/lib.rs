//! forthling for the browser: a WebAssembly module with one session, in the
//! sandbox. Its interface is a few plain exports, so it needs neither
//! wasm-bindgen nor a JavaScript build step; `www/forthling.js` wraps them.
//!
//! The page writes the code into the input buffer (`fl_input` gives room for
//! it), calls `fl_eval`, and reads the answer, a JSON text, from `fl_answer`.
//! The answer has the same shape as a result of the MCP server: `output`,
//! `stack`, `fstack`, `compiling` and `error`. The session's files are in
//! memory; `fl_write_file`, `fl_read_file`, `fl_delete_file` and `fl_files`
//! reach them.

// Lints of clippy::pedantic that the code is clean for; `clippy -D warnings` in CI
// keeps it so. (The rest of pedantic is not enabled: see the open issues.)
#![warn(
    clippy::cast_lossless,
    clippy::doc_markdown,
    clippy::float_cmp,
    clippy::format_push_string,
    clippy::many_single_char_names,
    clippy::match_same_arms,
    clippy::must_use_candidate,
    clippy::redundant_closure_for_method_calls,
    clippy::return_self_not_must_use,
    clippy::semicolon_if_nothing_returned,
    clippy::single_match_else,
    clippy::struct_excessive_bools,
    clippy::unnecessary_literal_bound,
    clippy::uninlined_format_args
)]

use forthling::{Debugger, Forth, Functional, Json, MemoryFs, Throw, TraceEntry, Tutorial};
use std::cell::{Cell, RefCell};
use std::fmt::Write as _;

thread_local! {
    static SESSION: RefCell<Option<Forth>> = const { RefCell::new(None) };
    static INPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static ANSWER: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static NOW: Cell<u64> = const { Cell::new(0) }; // seconds since 1970, from the page
    static RECORD: Cell<usize> = const { Cell::new(0) }; // trace entries to record, 0 for none
    static FILES: RefCell<MemoryFs> = RefCell::new(MemoryFs::new()); // the session's files
}

/// A new session: the sandbox (files only in memory, at most 4 MiB, no
/// terminal input, 10 million steps and 1 MiB of output per run;
/// WebAssembly has no clock for a time limit), the JSON words, TRACE-ON and
/// TRACE-OFF, and the time of the page for TIME&DATE.
fn session() -> Forth {
    let mut f = Forth::sandboxed();
    let files = MemoryFs::with_limit(MemoryFs::SANDBOX_LIMIT);
    f.set_file_system(Box::new(files.clone())).expect("no block buffers in use yet");
    FILES.with(|f| *f.borrow_mut() = files);
    f.install(&Json);
    f.install(&Functional);
    f.install(&forthling_regex::Regex);
    f.install(&Debugger);
    f.install(&Tutorial);
    f.set_clock(|| NOW.with(Cell::get));
    f
}

/// Runs something on the session, which is made on first use.
fn with_session<T>(run: impl FnOnce(&mut Forth) -> T) -> T {
    SESSION.with(|s| run(s.borrow_mut().get_or_insert_with(session)))
}

/// Room for `len` bytes of input; the page writes the code there.
#[unsafe(no_mangle)]
pub extern "C" fn fl_input(len: usize) -> *mut u8 {
    INPUT.with(|input| {
        let mut input = input.borrow_mut();
        input.clear();
        input.resize(len, 0);
        input.as_mut_ptr()
    })
}

/// Records a trace of at most `max` words in the next evaluations, or none
/// for 0; the answer then has `trace` and `trace_dropped`.
#[unsafe(no_mangle)]
pub extern "C" fn fl_record(max: usize) {
    RECORD.with(|r| r.set(max));
}

/// Runs the input as Forth code, with the time of the page (seconds since
/// 1970); returns the length of the answer.
#[unsafe(no_mangle)]
pub extern "C" fn fl_eval(now: f64) -> usize {
    NOW.with(|n| n.set(now.max(0.0) as u64));
    let code = INPUT.with(|i| String::from_utf8_lossy(&i.borrow()).into_owned());
    let record = RECORD.with(Cell::get);
    let answer = with_session(|f| {
        f.record_trace((record > 0).then_some(record));
        let result = f.interpret(&code);
        f.record_trace(None);
        let dropped = f.trace_dropped();
        let trace = f.take_trace();
        let output = String::from_utf8_lossy(&f.take_output()).into_owned();
        let mut answer = state(f, &output, result.err());
        // What the run took: its steps and the deepest stack.
        answer.pop();
        let _ = write!(answer, ",\"steps\":{},\"max_depth\":{}", f.steps_used(), f.max_depth());
        if record > 0 {
            let entries: Vec<String> = trace.iter().map(entry).collect();
            let _ =
                write!(answer, ",\"trace\":[{}],\"trace_dropped\":{dropped}", entries.join(","));
        }
        answer + "}"
    });
    set_answer(answer)
}

/// The state without running anything; returns the length of the answer.
#[unsafe(no_mangle)]
pub extern "C" fn fl_state() -> usize {
    set_answer(with_session(|f| state(f, "", None)))
}

/// Starts a new session; returns the length of the answer.
#[unsafe(no_mangle)]
pub extern "C" fn fl_reset() -> usize {
    SESSION.with(|s| *s.borrow_mut() = Some(session()));
    fl_state()
}

/// Runs a word of the tutorial, named by the input (`LESSONS`, `LESSON`,
/// `CHECK`, `HINT` or `NEXT`; `LESSON` shows lesson `n`), whatever the
/// search order. The answer is that of `fl_eval`, with `lesson`, the lesson
/// shown last, and `passed`, the lessons done; returns its length.
#[unsafe(no_mangle)]
pub extern "C" fn fl_tutorial(n: i32) -> usize {
    let word = INPUT.with(|i| String::from_utf8_lossy(&i.borrow()).into_owned());
    let answer = with_session(|f| {
        if word.eq_ignore_ascii_case("LESSON") {
            f.push(n.into());
        }
        let result = f.tutorial(&word);
        let output = String::from_utf8_lossy(&f.take_output()).into_owned();
        let mut answer = state(f, &output, result.err());
        let (lesson, passed) = f.tutorial_progress();
        let passed: Vec<String> = passed.iter().map(usize::to_string).collect();
        answer.pop();
        answer + &format!(",\"lesson\":{lesson},\"passed\":[{}]}}", passed.join(","))
    });
    set_answer(answer)
}

/// The lessons of the tutorial as a JSON array of `{"number", "title",
/// "level", "example"}`; returns the length.
#[unsafe(no_mangle)]
pub extern "C" fn fl_lessons() -> usize {
    let lessons: Vec<String> = Tutorial::lessons()
        .iter()
        .map(|l| {
            format!(
                "{{\"number\":{},\"title\":{},\"level\":{},\"example\":{}}}",
                l.number,
                quoted(&l.title),
                quoted(&l.level),
                quoted(&l.example)
            )
        })
        .collect();
    set_answer(format!("[{}]", lessons.join(",")))
}

/// The words of the session, as a JSON array; returns the length.
#[unsafe(no_mangle)]
pub extern "C" fn fl_words() -> usize {
    let words = with_session(|f| {
        let names: Vec<String> = f.program_words().iter().map(|w| quoted(w)).collect();
        format!("[{}]", names.join(","))
    });
    set_answer(words)
}

/// Puts a file in the session's files, replacing one of the same name: the
/// input is its name (`name_len` bytes), then its contents. The answer is
/// `null`, or the reason it failed as a JSON string; returns its length.
#[unsafe(no_mangle)]
pub extern "C" fn fl_write_file(name_len: usize) -> usize {
    with_session(|_| ());
    let input = INPUT.with(|i| i.borrow().clone());
    let (name, contents) = input.split_at(name_len.min(input.len()));
    let name = String::from_utf8_lossy(name);
    let result = FILES.with(|f| f.borrow().insert(&name, contents));
    set_answer(result.map_or_else(|e| quoted(&format!("{name}: {e}")), |()| "null".into()))
}

/// The contents of the file named by the input, as a JSON string (invalid
/// UTF-8 replaced), or `null` when there is none; returns the length.
#[unsafe(no_mangle)]
pub extern "C" fn fl_read_file() -> usize {
    with_session(|_| ());
    let name = INPUT.with(|i| String::from_utf8_lossy(&i.borrow()).into_owned());
    let contents = FILES.with(|f| f.borrow().get(&name));
    set_answer(contents.map_or_else(|| "null".into(), |c| quoted(&String::from_utf8_lossy(&c))))
}

/// Removes the file named by the input; the answer is `true`, or `false`
/// when there was none; returns its length.
#[unsafe(no_mangle)]
pub extern "C" fn fl_delete_file() -> usize {
    with_session(|_| ());
    let name = INPUT.with(|i| String::from_utf8_lossy(&i.borrow()).into_owned());
    let removed = FILES.with(|f| f.borrow().remove(&name).is_ok());
    set_answer(removed.to_string())
}

/// The session's files as a JSON array of `{"name", "size"}`, sorted by
/// name; returns the length.
#[unsafe(no_mangle)]
pub extern "C" fn fl_files() -> usize {
    with_session(|_| ());
    let files = FILES.with(|f| {
        let f = f.borrow();
        let sizes = f.paths().into_iter().map(|p| {
            let size = f.get(&p).map_or(0, |c| c.len());
            format!("{{\"name\":{},\"size\":{size}}}", quoted(&p))
        });
        sizes.collect::<Vec<_>>().join(",")
    });
    set_answer(format!("[{files}]"))
}

/// Where the latest answer is.
#[unsafe(no_mangle)]
pub extern "C" fn fl_answer() -> *const u8 {
    ANSWER.with(|a| a.borrow().as_ptr())
}

fn set_answer(answer: String) -> usize {
    ANSWER.with(|a| {
        *a.borrow_mut() = answer.into_bytes();
        a.borrow().len()
    })
}

/// The state as JSON: the output, the stacks, and the error if there was one.
pub fn state(f: &Forth, output: &str, error: Option<Throw>) -> String {
    let cells: Vec<String> = f.stack().iter().map(i64::to_string).collect();
    let floats: Vec<String> = f.fstack().iter().map(|&r| float(r)).collect();
    let error = match error {
        None => "null".to_string(),
        Some(e) => {
            // Where it happened and the stacks then, since the error has
            // emptied them.
            let context = e.context().map_or(String::new(), |c| {
                let cells: Vec<String> = c.stack.iter().map(i64::to_string).collect();
                let floats: Vec<String> = c.fstack.iter().map(|&r| float(r)).collect();
                let words: Vec<String> = c.words.iter().map(|w| quoted(w)).collect();
                format!(
                    ",\"stack\":[{}],\"fstack\":[{}],\"words\":[{}],\"source\":{},\"line\":{},\
                     \"column\":{},\"token\":{}",
                    cells.join(","),
                    floats.join(","),
                    words.join(","),
                    quoted(&c.source),
                    c.line,
                    c.column,
                    quoted(&c.token),
                )
            });
            format!(
                "{{\"code\":{},\"message\":{},\"detail\":{},\"hint\":{},\"text\":{}{context}}}",
                e.code,
                quoted(e.message()),
                e.detail().map_or("null".into(), quoted),
                e.hint().map_or("null".into(), quoted),
                quoted(&e.to_string()),
            )
        }
    };
    format!(
        "{{\"output\":{},\"stack\":[{}],\"fstack\":[{}],\"compiling\":{},\"error\":{}}}",
        quoted(output),
        cells.join(","),
        floats.join(","),
        f.in_definition(),
        error,
    )
}

/// A trace entry in JSON.
fn entry(e: &TraceEntry) -> String {
    let cells: Vec<String> = e.stack.iter().map(i64::to_string).collect();
    format!(
        "{{\"word\":{},\"depth\":{},\"stack\":[{}],\"stack_depth\":{}}}",
        quoted(&e.word),
        e.depth,
        cells.join(","),
        e.stack_depth
    )
}

/// A float in JSON; null for an infinity or a NaN.
fn float(r: f64) -> String {
    if r.is_finite() { format!("{r:?}") } else { "null".into() }
}

/// Text as a JSON string.
fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
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
    out
}
