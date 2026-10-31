//! forthling for the browser: a WebAssembly module with one session, in the
//! sandbox. Its interface is a few plain exports, so it needs neither
//! wasm-bindgen nor a JavaScript build step; `www/forthling.js` wraps them.
//!
//! The page writes the code into the input buffer (`fl_input` gives room for
//! it), calls `fl_eval`, and reads the answer, a JSON text, from `fl_answer`.
//! The answer has the same shape as a result of the MCP server: `output`,
//! `stack`, `fstack`, `compiling` and `error`.

use forthling::{Debugger, Forth, Json, Throw};
use std::cell::{Cell, RefCell};

thread_local! {
    static SESSION: RefCell<Option<Forth>> = const { RefCell::new(None) };
    static INPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static ANSWER: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static NOW: Cell<u64> = const { Cell::new(0) }; // seconds since 1970, from the page
}

/// A new session: the sandbox (no files, no terminal input, 10 million
/// steps and 1 MiB of output per run; WebAssembly has no clock for a time
/// limit), the JSON words, TRACE-ON and TRACE-OFF, and the time of the page for TIME&DATE.
fn session() -> Forth {
    let mut f = Forth::sandboxed();
    f.install(&Json);
    f.install(&Debugger);
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

/// Runs the input as Forth code, with the time of the page (seconds since
/// 1970); returns the length of the answer.
#[unsafe(no_mangle)]
pub extern "C" fn fl_eval(now: f64) -> usize {
    NOW.with(|n| n.set(now.max(0.0) as u64));
    let code = INPUT.with(|i| String::from_utf8_lossy(&i.borrow()).into_owned());
    let answer = with_session(|f| {
        let result = f.interpret(&code);
        let output = String::from_utf8_lossy(&f.take_output()).into_owned();
        let mut answer = state(f, &output, result.err());
        // What the run took: its steps and the deepest stack.
        answer.pop();
        answer += &format!(",\"steps\":{},\"max_depth\":{}}}", f.steps_used(), f.max_depth());
        answer
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

/// The words of the session, as a JSON array; returns the length.
#[unsafe(no_mangle)]
pub extern "C" fn fl_words() -> usize {
    let words = with_session(|f| {
        let names: Vec<String> = f.program_words().iter().map(|w| quoted(w)).collect();
        format!("[{}]", names.join(","))
    });
    set_answer(words)
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
                "{{\"code\":{},\"message\":{},\"detail\":{},\"text\":{}{context}}}",
                e.code,
                quoted(e.message()),
                e.detail().map_or("null".into(), quoted),
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
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
