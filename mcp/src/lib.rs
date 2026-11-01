//! An MCP server for forthling: AI agents run Forth through it, always in
//! the sandbox. The protocol is JSON-RPC, one message per line; `Server`
//! answers one message at a time, and main.rs carries them over stdio.
//!
//! The sandbox is enforced here, not by what the model is told: the tools
//! take only Forth code, no Forth word can change the limits or the
//! permissions, no setting of the server lifts a limit, and the files of a
//! session are in memory (a `MemoryFs`), never on the disk.

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

use forthling::{
    DEFAULT_MEMORY, ErrorContext, Extension, Forth, Limits, MemoryFs, Permission, Resume, Throw,
    TraceEntry,
};
use serde_json::{Map, Value, json};
use std::cell::RefCell;
use std::fmt::Write as _;
use std::io;
use std::rc::Rc;
use std::time::Duration;

/// The protocol versions the server speaks; it answers with the client's
/// version when it knows it, else with LATEST.
pub const PROTOCOL_VERSIONS: [&str; 4] = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];
pub const LATEST: &str = "2025-06-18";

/// The highest settings the server accepts, whoever starts it.
pub const MAX_MEMORY: usize = 64 << 20;
pub const MAX_STEPS: u64 = 1_000_000_000;
pub const MAX_TIME: Duration = Duration::from_secs(60);
pub const MAX_OUTPUT: usize = 16 << 20;
pub const MAX_SPACE: usize = 64 << 20;

/// The most words a trace of `forth_eval` holds; the rest are counted.
pub const MAX_TRACE: usize = 1000;

/// The most BREAKs a result of `forth_eval` shows; the rest are counted.
pub const MAX_BREAKS: usize = 100;

/// What the program looked like at each BREAK of a call, and how many more
/// there were.
#[derive(Default)]
struct Breaks {
    seen: Vec<ErrorContext>,
    dropped: usize,
}

/// What the server runs Forth with: always every limit, and files only in
/// memory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub memory: usize,
    pub steps: u64,
    pub time: Duration,
    pub output: usize,
    pub space: usize,
    /// The hosts that Forth code may send HTTP requests to (`--allow-host`);
    /// none by default, and then the network stays closed. Needs the feature `http`.
    pub hosts: Vec<String>,
}

impl Default for Settings {
    /// The memory of forthling and the limits of its sandbox.
    fn default() -> Self {
        let limits = Limits::SANDBOX;
        Settings {
            memory: DEFAULT_MEMORY,
            steps: limits.steps.unwrap_or(MAX_STEPS),
            time: limits.time.unwrap_or(MAX_TIME),
            output: limits.output.unwrap_or(MAX_OUTPUT),
            space: limits.space.unwrap_or(MAX_SPACE),
            hosts: Vec::new(),
        }
    }
}

impl Settings {
    /// Checks the settings against the highest the server accepts.
    pub fn check(&self) -> Result<(), String> {
        let over = |what: &str, max: String| Err(format!("{what} can be at most {max}"));
        if self.memory > MAX_MEMORY {
            return over("the memory", "64M".into());
        }
        if self.steps > MAX_STEPS {
            return over("--max-steps", MAX_STEPS.to_string());
        }
        if self.time > MAX_TIME {
            return over("--max-time", "60s".into());
        }
        if self.output > MAX_OUTPUT {
            return over("--max-output", "16M".into());
        }
        if self.space > MAX_SPACE {
            return over("--max-space", "64M".into());
        }
        if !self.hosts.is_empty() && !cfg!(feature = "http") {
            return Err("--allow-host needs a build with the feature http".into());
        }
        Ok(())
    }

    fn limits(&self) -> Limits {
        Limits {
            steps: Some(self.steps),
            time: Some(self.time),
            output: Some(self.output),
            space: Some(self.space),
        }
    }
}

/// A JSON-RPC error: its code and message.
type Failure = (i64, String);

const INVALID_PARAMS: i64 = -32602;

/// One session: a Forth system that keeps its definitions between calls.
pub struct Server {
    settings: Settings,
    forth: Forth,
    data: Rc<RefCell<Vec<u8>>>, // the latest data of forth_eval, which DATA gives
    breaks: Rc<RefCell<Breaks>>, // the BREAKs of the current call
    files: MemoryFs,            // the session's files
}

impl Server {
    #[must_use]
    pub fn new(settings: Settings) -> Self {
        let (data, breaks, files) = (Rc::default(), Rc::default(), files());
        let forth = sandbox(&settings, &data, &breaks, &files);
        Server { settings, forth, data, breaks, files }
    }

    /// Answers one message; None for a notification, which gets no answer.
    pub fn handle(&mut self, line: &str) -> Option<String> {
        let Ok(message) = serde_json::from_str::<Value>(line) else {
            return Some(reply(&Value::Null, Err((-32700, "parse error".into()))));
        };
        let Some(message) = message.as_object() else {
            return Some(reply(&Value::Null, Err((-32600, "not a JSON-RPC message".into()))));
        };
        let id = message.get("id").cloned();
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            // An answer from the client (the server asks nothing), or nonsense.
            return match (id, message.contains_key("result") || message.contains_key("error")) {
                (Some(_), true) | (None, _) => None,
                (Some(id), false) => Some(reply(&id, Err((-32600, "no method".into())))),
            };
        };
        let id = id?; // notifications (initialized, cancelled) need nothing
        let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
        let result = match method {
            "initialize" => Ok(self.initialize(&params)),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tools() })),
            "tools/call" => self.call(&params),
            _ => Err((-32601, format!("unknown method {method}"))),
        };
        Some(reply(&id, result))
    }

    fn initialize(&self, params: &Value) -> Value {
        let asked = params.get("protocolVersion").and_then(Value::as_str);
        let version = asked.filter(|v| PROTOCOL_VERSIONS.contains(v)).unwrap_or(LATEST);
        json!({
            "protocolVersion": version,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": {
                "name": "forthling",
                "title": "forthling",
                "version": env!("CARGO_PKG_VERSION"),
            },
            "instructions": instructions(&self.settings),
        })
    }

    fn eval(&mut self, args: &Map<String, Value>) -> Result<Value, Failure> {
        let Some(code) = args.get("code").and_then(Value::as_str) else {
            return Err((INVALID_PARAMS, "forth_eval needs code, a string".into()));
        };
        match args.get("data") {
            None => {}
            Some(Value::String(data)) if data.len() <= self.settings.memory => {
                *self.data.borrow_mut() = data.clone().into_bytes();
            }
            Some(Value::String(_)) => {
                return Err((INVALID_PARAMS, "data is larger than the Forth memory".into()));
            }
            Some(_) => return Err((INVALID_PARAMS, "data must be a string".into())),
        }
        match args.get("files") {
            None => {}
            Some(Value::Object(files)) => {
                let Some(texts) = files.values().map(Value::as_str).collect::<Option<Vec<_>>>()
                else {
                    return Err((INVALID_PARAMS, "each file must be a string".into()));
                };
                for (name, text) in files.keys().zip(texts) {
                    self.files
                        .insert(name, text)
                        .map_err(|e| (INVALID_PARAMS, format!("file {name}: {e}")))?;
                }
            }
            Some(_) => {
                return Err((INVALID_PARAMS, "files must be an object of texts".into()));
            }
        }
        let trace = match args.get("trace") {
            None | Some(Value::Bool(false)) => false,
            Some(Value::Bool(true)) => true,
            Some(_) => return Err((INVALID_PARAMS, "trace must be a boolean".into())),
        };
        self.forth.record_trace(trace.then_some(MAX_TRACE));
        let result = self.forth.interpret(code);
        self.forth.record_trace(None);
        let dropped = self.forth.trace_dropped();
        let entries = self.forth.take_trace();
        let breaks = std::mem::take(&mut *self.breaks.borrow_mut());
        let output = String::from_utf8_lossy(&self.forth.take_output()).into_owned();
        let mut answer = state(&self.forth, &output, result.err());
        let content = &mut answer["structuredContent"];
        content["steps"] = json!(self.forth.steps_used());
        content["max_depth"] = json!(self.forth.max_depth());
        if trace {
            content["trace"] = entries.iter().map(trace_json).collect();
            content["trace_dropped"] = json!(dropped);
            let text = &mut answer["content"][0]["text"];
            let lines = trace_text(&entries, dropped);
            *text = json!(format!("{lines}{}", text.as_str().unwrap_or_default()));
        }
        if !breaks.seen.is_empty() {
            let content = &mut answer["structuredContent"];
            content["breaks"] = breaks.seen.iter().map(context_json).collect();
            content["breaks_dropped"] = json!(breaks.dropped);
            let text = &mut answer["content"][0]["text"];
            let lines = breaks_text(&breaks);
            *text = json!(format!("{lines}{}", text.as_str().unwrap_or_default()));
        }
        Ok(answer)
    }

    fn call(&mut self, params: &Value) -> Result<Value, Failure> {
        let name = params.get("name").and_then(Value::as_str).unwrap_or_default();
        let none = Map::new();
        let args = match params.get("arguments") {
            None | Some(Value::Null) => &none,
            Some(Value::Object(args)) => args,
            Some(_) => return Err((INVALID_PARAMS, "arguments must be an object".into())),
        };
        let allowed: &[&str] = match name {
            "forth_eval" => &["code", "data", "files", "trace"],
            "forth_stack" | "forth_words" | "forth_reset" => &[],
            _ => return Err((INVALID_PARAMS, format!("unknown tool {name}"))),
        };
        if let Some(key) = args.keys().find(|k| !allowed.contains(&k.as_str())) {
            return Err((INVALID_PARAMS, format!("{name} takes no argument {key}")));
        }
        match name {
            "forth_eval" => self.eval(args),
            "forth_stack" => Ok(state(&self.forth, "", None)),
            "forth_words" => {
                let words = self.forth.program_words();
                let text = if words.is_empty() {
                    "no words defined yet".to_string()
                } else {
                    words.join(" ")
                };
                Ok(json!({
                    "content": [{ "type": "text", "text": text }],
                    "structuredContent": { "words": words },
                }))
            }
            _ => {
                self.data.borrow_mut().clear();
                self.files = files();
                self.forth = sandbox(&self.settings, &self.data, &self.breaks, &self.files);
                Ok(state(&self.forth, "", None))
            }
        }
    }
}

/// The files of a new session: none, and room for `MemoryFs::SANDBOX_LIMIT`
/// bytes.
fn files() -> MemoryFs {
    MemoryFs::with_limit(MemoryFs::SANDBOX_LIMIT)
}

/// A Forth system in the sandbox, with the files in memory, the JSON words,
/// the word DATA and, with the feature `sqlite`, the `SQLite` words for
/// databases in memory.
fn sandbox(
    settings: &Settings,
    data: &Rc<RefCell<Vec<u8>>>,
    breaks: &Rc<RefCell<Breaks>>,
    files: &MemoryFs,
) -> Forth {
    let mut f = Forth::with_memory(settings.memory);
    f.set_limits(settings.limits());
    f.set_file_system(Box::new(files.clone())).expect("no block buffers in use yet");
    // Terminal input would be the protocol itself; KEY and ACCEPT get none.
    f.set_input(Box::new(io::Cursor::new(Vec::new())));
    f.install(&forthling::Json);
    f.install(&forthling::Functional);
    f.install(&forthling::Debugger);
    f.install(&forthling::Tutorial);
    // BREAK does not stop the program, which no one is watching: it records
    // what the program looked like, for the result.
    let breaks = Rc::clone(breaks);
    f.set_debugger(Some(Box::new(move |_, pause| {
        let mut breaks = breaks.borrow_mut();
        if breaks.seen.len() < MAX_BREAKS {
            breaks.seen.push(pause.context.clone());
        } else {
            breaks.dropped += 1;
        }
        Ok(Resume::Continue)
    })));
    #[cfg(feature = "sqlite")]
    f.install(&forthling_sqlite::Sqlite);
    #[cfg(feature = "regex")]
    f.install(&forthling_regex::Regex);
    // The network stays closed unless the server was started with hosts.
    f.permit(Permission::Network, false);
    #[cfg(feature = "http")]
    if !settings.hosts.is_empty() {
        f.install(
            &forthling_http::Http::new().allow_all(settings.hosts.iter().map(String::as_str)),
        );
        f.permit(Permission::Network, true);
    }
    f.install(&Data(Rc::clone(data)));
    f
}

/// The word DATA, as an extension (FORTHLING-MCP), so that it is the
/// system's and not one of the session's own words.
struct Data(Rc<RefCell<Vec<u8>>>);

impl Extension for Data {
    fn name(&self) -> &'static str {
        "FORTHLING-MCP"
    }

    fn install(&self, f: &mut Forth) {
        // DATA ( -- c-addr u ): the data of forth_eval, ALLOCATEd, for the
        // program to FREE. Data has no line limit, as code has.
        let data = Rc::clone(&self.0);
        let word = move |f: &mut Forth| {
            let data = data.borrow().clone();
            f.charge(data.len())?;
            f.push_bytes(&data)
        };
        f.add_word(1, "DATA", word).expect("FORTH-WORDLIST exists");
    }
}

fn reply(id: &Value, result: Result<Value, Failure>) -> String {
    let message = match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err((code, message)) => {
            json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
        }
    };
    message.to_string()
}

/// The result of a tool that shows the state: the output, the stacks and
/// the error, as text the way the REPL shows them, and as JSON.
fn state(f: &Forth, output: &str, error: Option<Throw>) -> Value {
    let cells = |stack: &[i64]| stack.iter().map(i64::to_string).collect::<Vec<_>>().join(" ");
    let float_cells =
        |stack: &[f64]| stack.iter().map(|r| format!("{r:E}")).collect::<Vec<_>>().join(" ");
    let floats = float_cells(f.fstack());
    let mut text = output.to_string();
    match &error {
        Some(e) => {
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            text += &e.to_string();
            // Where it happened and what the stacks held then, since the
            // error has emptied them.
            if let Some(c) = e.context() {
                if c.column > 0 {
                    let _ = write!(
                        text,
                        "\nat {}, line {}, column {}: {}",
                        c.source, c.line, c.column, c.token
                    );
                }
                if !c.words.is_empty() {
                    let _ = write!(text, "\nin: {}", c.words.join(" > "));
                }
                let at = if c.stack.is_empty() { "empty".to_string() } else { cells(&c.stack) };
                let _ = write!(text, "\nstack at the error: {at}");
                if !c.fstack.is_empty() {
                    let _ = write!(text, "\nfloat stack at the error: {}", float_cells(&c.fstack));
                }
            }
        }
        None if f.in_definition() => text += " compiled",
        None => text += " ok",
    }
    let stack = if f.stack().is_empty() { "empty".to_string() } else { cells(f.stack()) };
    let _ = write!(text, "\nstack: {stack}");
    if !floats.is_empty() {
        let _ = write!(text, "\nfloat stack: {floats}");
    }
    let error = error.map(|e| {
        let mut error = json!({
            "code": e.code,
            "message": e.message(),
            "detail": e.detail(),
            "location": e.location(),
            "hint": e.hint(),
        });
        if let Some(c) = e.context() {
            error["stack"] = json!(c.stack);
            error["fstack"] = json!(c.fstack);
            error["words"] = json!(c.words);
            error["source"] = json!(c.source);
            error["line"] = json!(c.line);
            error["column"] = json!(c.column);
            error["token"] = json!(c.token);
        }
        error
    });
    json!({
        "content": [{ "type": "text", "text": text }],
        "structuredContent": {
            "output": output,
            "stack": f.stack(),
            "fstack": f.fstack(),
            "compiling": f.in_definition(),
            "error": error,
        },
        "isError": error.is_some(),
    })
}

fn trace_json(e: &TraceEntry) -> Value {
    json!({ "word": e.word, "depth": e.depth, "stack": e.stack, "stack_depth": e.stack_depth })
}

/// The trace as text, a line per word as TRACE-ON prints it (indented by the
/// depth, in decimal), and a line for the words that did not fit.
fn trace_text(entries: &[TraceEntry], dropped: usize) -> String {
    let mut text = String::from("trace:\n");
    for e in entries {
        text += &"  ".repeat(e.depth.min(16));
        let _ = write!(text, "{} <{}>", e.word, e.stack_depth);
        if e.stack.len() < e.stack_depth {
            text += " ...";
        }
        for n in &e.stack {
            let _ = write!(text, " {n}");
        }
        text.push('\n');
    }
    if dropped > 0 {
        let _ = writeln!(text, "... and {dropped} more words");
    }
    text + "end of trace\n"
}

/// The state at an error or a BREAK, as JSON.
fn context_json(c: &ErrorContext) -> Value {
    json!({
        "stack": c.stack,
        "fstack": c.fstack,
        "words": c.words,
        "source": c.source,
        "line": c.line,
        "column": c.column,
        "token": c.token,
    })
}

/// The BREAKs as text: a line each, with where and the stacks.
fn breaks_text(breaks: &Breaks) -> String {
    let mut text = String::new();
    for (i, c) in breaks.seen.iter().enumerate() {
        let stack: Vec<String> = c.stack.iter().map(i64::to_string).collect();
        let _ = writeln!(
            text,
            "break {}: in {}, at {}, line {}, column {}: stack <{}> {}",
            i + 1,
            c.words.join(" > "),
            c.source,
            c.line,
            c.column,
            stack.len(),
            stack.join(" ")
        );
        if !c.fstack.is_empty() {
            let floats: Vec<String> = c.fstack.iter().map(|r| format!("{r:E}")).collect();
            let _ = writeln!(text, "  float stack: {}", floats.join(" "));
        }
    }
    if breaks.dropped > 0 {
        let _ = writeln!(text, "... and {} more breaks", breaks.dropped);
    }
    text
}

/// The schema of `state`.
fn state_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "output": { "type": "string", "description": "what the code printed" },
            "stack": { "type": "array", "items": { "type": "integer" }, "description": "the data stack, the top last" },
            "fstack": { "type": "array", "items": { "type": ["number", "null"] }, "description": "the float stack, the top last" },
            "compiling": { "type": "boolean", "description": "whether a colon definition is still open" },
            "error": {
                "type": ["object", "null"],
                "properties": {
                    "code": { "type": "integer", "description": "the THROW code" },
                    "message": { "type": "string" },
                    "detail": { "type": ["string", "null"] },
                    "location": { "type": ["string", "null"] },
                    "hint": { "type": ["string", "null"], "description": "how to put the error right, such as the ALSO that finds an undefined word" },
                    "stack": { "type": "array", "items": { "type": "integer" }, "description": "the data stack at the error, before it was emptied; a word that took its arguments before it failed has taken them" },
                    "fstack": { "type": "array", "items": { "type": ["number", "null"] }, "description": "the float stack at the error" },
                    "words": { "type": "array", "items": { "type": "string" }, "description": "the words that were running, outermost first, such as MAIN, STEP, /" },
                    "source": { "type": "string", "description": "input (the code of this call), evaluate (a string given to EVALUATE) or block n" },
                    "line": { "type": "integer", "description": "the line in that source, from 1" },
                    "column": { "type": "integer", "description": "the column of token, in characters from 1; 0 when no word was read" },
                    "token": { "type": "string", "description": "the last word read: the one that failed, or the one that ran the code that failed" },
                },
                "required": ["code", "message"],
            },
            "steps": { "type": "integer", "description": "forth_eval only: the steps this call took, of its limit" },
            "max_depth": { "type": "integer", "description": "forth_eval only: the deepest the data stack was during this call" },
            "trace": {
                "type": "array",
                "description": "forth_eval with trace only: a word per entry, in the order they ran",
                "items": {
                    "type": "object",
                    "properties": {
                        "word": { "type": "string" },
                        "depth": { "type": "integer", "description": "0 for a word of the code, 1 for a word it calls, and so on" },
                        "stack": { "type": "array", "items": { "type": "integer" }, "description": "the data stack when the word started, the top last; at most the top 8 cells" },
                        "stack_depth": { "type": "integer", "description": "the number of cells on the data stack then" },
                    },
                    "required": ["word", "depth", "stack", "stack_depth"],
                },
            },
            "trace_dropped": { "type": "integer", "description": "forth_eval with trace only: the words past the first 1000, which the trace leaves out" },
            "breaks": {
                "type": "array",
                "description": "forth_eval only, when BREAK ran: what the program looked like at each BREAK (ALSO DEBUG), at most 100; BREAK does not stop the program",
                "items": {
                    "type": "object",
                    "properties": {
                        "stack": { "type": "array", "items": { "type": "integer" } },
                        "fstack": { "type": "array", "items": { "type": ["number", "null"] } },
                        "words": { "type": "array", "items": { "type": "string" }, "description": "the words that were running, outermost first, ending in BREAK" },
                        "source": { "type": "string" },
                        "line": { "type": "integer" },
                        "column": { "type": "integer" },
                        "token": { "type": "string" },
                    },
                },
            },
            "breaks_dropped": { "type": "integer", "description": "the BREAKs past the first 100" },
        },
        "required": ["output", "stack", "fstack", "compiling", "error"],
    })
}

fn tools() -> Value {
    let no_arguments = json!({ "type": "object", "properties": {}, "additionalProperties": false });
    json!([
        {
            "name": "forth_eval",
            "title": "Run Forth",
            "description": "Runs Forth-2012 code in the session and returns what it printed, the data stack and the float stack, and the error if there was one. Definitions, variables, files and the stacks stay for the next call. Large input goes in data, which the code reads with DATA; source files to INCLUDE go in files.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "code": { "type": "string", "description": "Forth source; it may have several lines, each at most 1024 characters" },
                    "trace": { "type": "boolean", "description": "If true, the result also gives a trace: each word that ran (not inside the built-in words), with how deep it was nested and the data stack when it started; at most 1000 words. For finding where code goes wrong." },
                    "data": { "type": "string", "description": "Text for the word DATA ( -- c-addr u ), which gives a copy in ALLOCATEd memory (FREE it); without a line limit, for large input such as JSON. It stays for later calls until another call sends data." },
                    "files": { "type": "object", "additionalProperties": { "type": "string" }, "description": "Files to put in the session's file system before the code runs, as {\"name\": \"text\"}, replacing files of the same name; the code can INCLUDE them or open them with the FILE words. The files are in memory and stay until forth_reset." },
                },
                "required": ["code"],
                "additionalProperties": false,
            },
            "outputSchema": state_schema(),
            "annotations": { "readOnlyHint": false, "destructiveHint": false, "idempotentHint": false, "openWorldHint": false },
        },
        {
            "name": "forth_stack",
            "title": "Show the stacks",
            "description": "Shows the data stack and the float stack, without running anything.",
            "inputSchema": no_arguments,
            "outputSchema": state_schema(),
            "annotations": { "readOnlyHint": true, "openWorldHint": false },
        },
        {
            "name": "forth_words",
            "title": "List my words",
            "description": "Lists the words defined in this session, the latest first (not the built-in words).",
            "inputSchema": no_arguments,
            "outputSchema": {
                "type": "object",
                "properties": { "words": { "type": "array", "items": { "type": "string" } } },
                "required": ["words"],
            },
            "annotations": { "readOnlyHint": true, "openWorldHint": false },
        },
        {
            "name": "forth_reset",
            "title": "Start over",
            "description": "Starts a new Forth session: every definition, variable and stack value is gone, and so are the data and the files.",
            "inputSchema": no_arguments,
            "outputSchema": state_schema(),
            "annotations": { "readOnlyHint": false, "destructiveHint": true, "idempotentHint": true, "openWorldHint": false },
        },
    ])
}

/// What the model is told when it connects. It describes the limits; it is
/// not what enforces them.
fn instructions(s: &Settings) -> String {
    let sqlite = if cfg!(feature = "sqlite") {
        " `ALSO SQLITE` gives SQLite words for databases in memory (`S\" :memory:\" DB-OPEN`)."
    } else {
        ""
    };
    let regex = if cfg!(feature = "regex") {
        " `ALSO REGEX` gives regular expressions in linear time: `RE-MATCH?`, `RE-FIND`, `RE-GROUP`, `RE-COUNT` and `RE-REPLACE`, which take the text and then the pattern, each as ( c-addr u )."
    } else {
        ""
    };
    let http = if cfg!(feature = "http") && !s.hosts.is_empty() {
        format!(
            " `ALSO HTTP` gives an HTTP client for these hosts only: {}. `HTTP-GET ( url-addr url-u -- body-addr body-u status )` and `HTTP-POST ( url-addr url-u body-addr body-u type-addr type-u -- body-addr' body-u' status )` give the body, ALLOCATEd, and the status, a number such as 404 that is no error; `HTTP-HEADER ( name-addr name-u value-addr value-u -- )` sets a header for the next request only, and `HTTP-RESPONSE-HEADER ( name-addr name-u -- value-addr value-u true | false )` reads one of the response. A failed request throws -263. Put the strings of one call in a definition: the strings of `S\"` in the interpreter share a buffer.",
            s.hosts.join(", ")
        )
    } else {
        String::new()
    };
    format!(
        "forthling runs Forth-2012 code in a sandbox. Use forth_eval to run code; definitions, \
         variables and the stacks stay between calls, and forth_reset starts over. Each result \
         shows the output and the data stack (the top last). Words are not case sensitive; `.` \
         prints and drops the top of the stack; `: NAME ... ;` defines a word; IF, DO, BEGIN and \
         the like work only inside a definition; `S\" text\"` gives ( c-addr u ) and TYPE prints \
         it. For more than two or three values, name them with locals rather than juggling \
         them with ROT, PICK and ROLL: `: HYP2 {{: a b -- c :}} a a * b b * + ;` (the last \
         name gets the top). An error empties the stacks, but the result shows them as they were at the error, with where it happened and the words that were running. To see what code does word by word, pass `trace: true` to forth_eval: the result then lists each word that ran with the data stack when it started. Or put BREAK (after `ALSO DEBUG`) where you want to look: the result then shows the stacks and the running words at each BREAK. A line of code, and so a string in `S\" \"`, is at most \
         1024 characters: give larger input, such as a JSON document, as the `data` argument of \
         forth_eval, and read it in the code with `DATA ( -- c-addr u )`, an ALLOCATEd copy. \
         `ALSO JSON` gives JSON words: `JSON@` reads a path such \
         as `users[0].name` from JSON text, and `JSON{{ S\" key\" JSON-KEY 1 JSON-NUMBER JSON}}` \
         writes JSON. `ALSO FUNCTIONAL` gives `MAP FOLD SCAN FILTER RANGE ANY? ALL? FIND COUNT-IF FLAT-MAP ITERATE REVERSE TAKE DROP-N ZIP SUM PRODUCT MIN-OF MAX-OF` on the data stack, and the same on a block of cells in memory (`MAP-CELLS FOLD-CELLS SCAN-CELLS FILTER-CELLS ANY?-CELLS` and more), \
         such as `1 6 RANGE [: DUP * ;] MAP 5 ' + FOLD .` (55). `ALSO TUTORIAL` gives a Forth course of 20 lessons to walk a person \
         through: `LESSONS`, `n LESSON`, `CHECK` (tests their answer), `HINT`, `NEXT`.{sqlite}{regex}{http} \
         Each call may take at most {} steps, {} ms and {} bytes of output; \
         the session's definitions and the like at most {} bytes in all. Files are in memory, \
         never on the disk, at most {} bytes in all: INCLUDE, REQUIRE, the FILE words \
         (`S\" out.txt\" W/O CREATE-FILE`) and BLOCK work on them. Give the code source files \
         with the `files` argument of forth_eval, {{\"name.fs\": \"text\"}}, and INCLUDE them; \
         files stay until forth_reset.",
        s.steps,
        s.time.as_millis(),
        s.output,
        s.space,
        MemoryFs::SANDBOX_LIMIT,
    )
}
