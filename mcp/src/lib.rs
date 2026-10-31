//! An MCP server for forthling: AI agents run Forth through it, always in
//! the sandbox. The protocol is JSON-RPC, one message per line; `Server`
//! answers one message at a time, and main.rs carries them over stdio.
//!
//! The sandbox is enforced here, not by what the model is told: the tools
//! take only Forth code, no Forth word can change the limits or the
//! permissions, and no setting of the server opens files or lifts a limit.

use forthling::{DEFAULT_MEMORY, Extension, Forth, Limits, Permission, Throw};
use serde_json::{Map, Value, json};
use std::cell::RefCell;
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

/// What the server runs Forth with: always every limit, never files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub memory: usize,
    pub steps: u64,
    pub time: Duration,
    pub output: usize,
    pub space: usize,
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
}

impl Server {
    pub fn new(settings: Settings) -> Self {
        let data = Rc::default();
        Server { settings, forth: sandbox(&settings, &data), data }
    }

    /// Answers one message; None for a notification, which gets no answer.
    pub fn handle(&mut self, line: &str) -> Option<String> {
        let Ok(message) = serde_json::from_str::<Value>(line) else {
            return Some(reply(Value::Null, Err((-32700, "parse error".into()))));
        };
        let Some(message) = message.as_object() else {
            return Some(reply(Value::Null, Err((-32600, "not a JSON-RPC message".into()))));
        };
        let id = message.get("id").cloned();
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            // An answer from the client (the server asks nothing), or nonsense.
            return match (id, message.contains_key("result") || message.contains_key("error")) {
                (Some(_), true) | (None, _) => None,
                (Some(id), false) => Some(reply(id, Err((-32600, "no method".into())))),
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
        Some(reply(id, result))
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

    fn call(&mut self, params: &Value) -> Result<Value, Failure> {
        let name = params.get("name").and_then(Value::as_str).unwrap_or_default();
        let none = Map::new();
        let args = match params.get("arguments") {
            None | Some(Value::Null) => &none,
            Some(Value::Object(args)) => args,
            Some(_) => return Err((INVALID_PARAMS, "arguments must be an object".into())),
        };
        let allowed: &[&str] = match name {
            "forth_eval" => &["code", "data"],
            "forth_stack" | "forth_words" | "forth_reset" => &[],
            _ => return Err((INVALID_PARAMS, format!("unknown tool {name}"))),
        };
        if let Some(key) = args.keys().find(|k| !allowed.contains(&k.as_str())) {
            return Err((INVALID_PARAMS, format!("{name} takes no argument {key}")));
        }
        match name {
            "forth_eval" => {
                let Some(code) = args.get("code").and_then(Value::as_str) else {
                    return Err((INVALID_PARAMS, "forth_eval needs code, a string".into()));
                };
                match args.get("data") {
                    None => {}
                    Some(Value::String(data)) if data.len() <= self.settings.memory => {
                        *self.data.borrow_mut() = data.clone().into_bytes();
                    }
                    Some(Value::String(_)) => {
                        return Err((
                            INVALID_PARAMS,
                            "data is larger than the Forth memory".into(),
                        ));
                    }
                    Some(_) => return Err((INVALID_PARAMS, "data must be a string".into())),
                }
                let result = self.forth.interpret(code);
                let output = String::from_utf8_lossy(&self.forth.take_output()).into_owned();
                Ok(state(&self.forth, &output, result.err()))
            }
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
                self.forth = sandbox(&self.settings, &self.data);
                Ok(state(&self.forth, "", None))
            }
        }
    }
}

/// A Forth system in the sandbox, with the JSON words, the word DATA and,
/// with the feature `sqlite`, the SQLite words for databases in memory.
fn sandbox(settings: &Settings, data: &Rc<RefCell<Vec<u8>>>) -> Forth {
    let mut f = Forth::with_memory(settings.memory);
    f.set_limits(settings.limits());
    f.permit(Permission::Files, false);
    // Terminal input would be the protocol itself; KEY and ACCEPT get none.
    f.set_input(Box::new(io::Cursor::new(Vec::new())));
    f.install(&forthling::Json);
    #[cfg(feature = "sqlite")]
    f.install(&forthling_sqlite::Sqlite);
    f.install(&Data(Rc::clone(data)));
    f
}

/// The word DATA, as an extension (FORTHLING-MCP), so that it is the
/// system's and not one of the session's own words.
struct Data(Rc<RefCell<Vec<u8>>>);

impl Extension for Data {
    fn name(&self) -> &str {
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

fn reply(id: Value, result: Result<Value, Failure>) -> String {
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
    let floats = f.fstack().iter().map(|r| format!("{r:E}")).collect::<Vec<_>>().join(" ");
    let mut text = output.to_string();
    match &error {
        Some(e) => {
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            text += &e.to_string();
        }
        None if f.in_definition() => text += " compiled",
        None => text += " ok",
    }
    let stack = if f.stack().is_empty() { "empty".to_string() } else { cells(f.stack()) };
    text += &format!("\nstack: {stack}");
    if !floats.is_empty() {
        text += &format!("\nfloat stack: {floats}");
    }
    let error = error.map(|e| {
        json!({
            "code": e.code,
            "message": e.message(),
            "detail": e.detail(),
            "location": e.location(),
        })
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
                },
                "required": ["code", "message"],
            },
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
            "description": "Runs Forth-2012 code in the session and returns what it printed, the data stack and the float stack, and the error if there was one. Definitions, variables and the stacks stay for the next call. Large input goes in data, which the code reads with DATA.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "code": { "type": "string", "description": "Forth source; it may have several lines, each at most 1024 characters" },
                    "data": { "type": "string", "description": "Text for the word DATA ( -- c-addr u ), which gives a copy in ALLOCATEd memory (FREE it); without a line limit, for large input such as JSON. It stays for later calls until another call sends data." },
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
            "description": "Starts a new Forth session: every definition, variable and stack value is gone, and so is the data.",
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
    format!(
        "forthling runs Forth-2012 code in a sandbox. Use forth_eval to run code; definitions, \
         variables and the stacks stay between calls, and forth_reset starts over. Each result \
         shows the output and the data stack (the top last). Words are not case sensitive; `.` \
         prints and drops the top of the stack; `: NAME ... ;` defines a word; IF, DO, BEGIN and \
         the like work only inside a definition; `S\" text\"` gives ( c-addr u ) and TYPE prints \
         it. For more than two or three values, name them with locals rather than juggling \
         them with ROT, PICK and ROLL: `: HYP2 {{: a b -- c :}} a a * b b * + ;` (the last \
         name gets the top). An error empties the stacks. A line of code, and so a string in `S\" \"`, is at most \
         1024 characters: give larger input, such as a JSON document, as the `data` argument of \
         forth_eval, and read it in the code with `DATA ( -- c-addr u )`, an ALLOCATEd copy. \
         `ALSO JSON` gives JSON words: `JSON@` reads a path such \
         as `users[0].name` from JSON text, and `JSON{{ S\" key\" JSON-KEY 1 JSON-NUMBER JSON}}` \
         writes JSON.{sqlite} Each call may take at most {} steps, {} ms and {} bytes of output; \
         the session's definitions and the like at most {} bytes in all; there is no file access.",
        s.steps,
        s.time.as_millis(),
        s.output,
        s.space,
    )
}
