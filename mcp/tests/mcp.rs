//! The MCP server: the protocol, the tools, and attempts to leave the sandbox.

use forthling_mcp::{LATEST, MAX_STEPS, Server, Settings};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

fn server() -> Server {
    Server::new(Settings::default())
}

/// Sends a request and returns its answer.
fn ask(s: &mut Server, method: &str, params: Value) -> Value {
    let request = json!({ "jsonrpc": "2.0", "id": 7, "method": method, "params": params });
    let answer: Value = serde_json::from_str(&s.handle(&request.to_string()).unwrap()).unwrap();
    assert_eq!((&answer["jsonrpc"], &answer["id"]), (&json!("2.0"), &json!(7)));
    answer
}

/// Calls a tool and returns its result.
fn call(s: &mut Server, tool: &str, arguments: Value) -> Value {
    let answer = ask(s, "tools/call", json!({ "name": tool, "arguments": arguments }));
    assert!(answer.get("error").is_none(), "{answer}");
    answer["result"].clone()
}

fn eval(s: &mut Server, code: &str) -> Value {
    call(s, "forth_eval", json!({ "code": code }))
}

fn text(result: &Value) -> &str {
    result["content"][0]["text"].as_str().unwrap()
}

fn error_code(answer: &Value) -> i64 {
    answer["error"]["code"].as_i64().unwrap_or_else(|| panic!("no error: {answer}"))
}

#[test]
fn initialize_and_list() {
    let mut s = server();
    let init =
        ask(&mut s, "initialize", json!({ "protocolVersion": "2025-03-26", "capabilities": {} }));
    assert_eq!(init["result"]["protocolVersion"], "2025-03-26");
    assert_eq!(init["result"]["serverInfo"]["name"], "forthling");
    assert!(init["result"]["capabilities"]["tools"].is_object());
    let instructions = init["result"]["instructions"].as_str().unwrap();
    assert!(instructions.contains("10000000 steps") && instructions.contains("no file access"));
    let unknown = ask(&mut s, "initialize", json!({ "protocolVersion": "1999-01-01" }));
    assert_eq!(unknown["result"]["protocolVersion"], LATEST);
    // A notification gets no answer.
    assert_eq!(s.handle(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#), None);
    assert_eq!(ask(&mut s, "ping", json!({}))["result"], json!({}));
    let tools = ask(&mut s, "tools/list", json!({}));
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["forth_eval", "forth_stack", "forth_words", "forth_reset"]);
}

#[test]
fn a_session_keeps_its_definitions() {
    let mut s = server();
    let r = eval(&mut s, ": SQUARE DUP * ;\n7 SQUARE");
    assert_eq!(text(&r), " ok\nstack: 49");
    assert_eq!(r["structuredContent"]["stack"], json!([49]));
    assert_eq!(r["isError"], false);
    let r = eval(&mut s, "SQUARE . 1.5E0");
    assert_eq!(r["structuredContent"]["output"], "2401 ");
    assert_eq!(text(&r), "2401  ok\nstack: empty\nfloat stack: 1.5E0");
    assert_eq!(r["structuredContent"]["fstack"], json!([1.5]));
    let r = eval(&mut s, ": HALF 2 /");
    assert_eq!(
        (r["structuredContent"]["compiling"].clone(), text(&r)),
        (json!(true), " compiled\nstack: empty\nfloat stack: 1.5E0")
    );
    eval(&mut s, ";");
    assert_eq!(
        call(&mut s, "forth_words", json!({}))["structuredContent"]["words"],
        json!(["HALF", "SQUARE"])
    );
    eval(&mut s, "1 2 3");
    assert_eq!(
        call(&mut s, "forth_stack", json!({}))["structuredContent"]["stack"],
        json!([1, 2, 3])
    );
    let r = call(&mut s, "forth_reset", json!({}));
    assert_eq!(r["structuredContent"]["stack"], json!([]));
    assert_eq!(text(&call(&mut s, "forth_words", json!({}))), "no words defined yet");
}

#[test]
fn forth_errors_are_results() {
    let mut s = server();
    let r = eval(&mut s, ".\" before\" CR 1 2 NOSUCH");
    assert_eq!(r["isError"], true);
    let error = &r["structuredContent"]["error"];
    assert_eq!(
        (error["code"].clone(), error["message"].clone()),
        (json!(-13), json!("undefined word"))
    );
    assert_eq!(error["detail"], "NOSUCH");
    assert_eq!(
        text(&r),
        "before\nerror -13: undefined word: NOSUCH\nat input, line 1, column 19: NOSUCH\n\
         stack at the error: 1 2\nstack: empty"
    );
    let r = eval(&mut s, "IF");
    assert_eq!(r["structuredContent"]["error"]["code"], -14);
}

#[test]
fn errors_show_where_and_the_stacks_then() {
    let mut s = server();
    eval(&mut s, ": STEP 0 / ;\n: MAIN 1 2 STEP ;");
    let r = eval(&mut s, "5\n  MAIN");
    let error = &r["structuredContent"]["error"];
    assert_eq!(error["words"], json!(["MAIN", "STEP", "/"]));
    assert_eq!(error["stack"], json!([5, 1]));
    assert_eq!((error["line"].clone(), error["column"].clone()), (json!(2), json!(3)));
    assert_eq!((error["source"].clone(), error["token"].clone()), (json!("input"), json!("MAIN")));
    assert_eq!(r["structuredContent"]["stack"], json!([]));
    assert!(text(&r).contains("\nin: MAIN > STEP > /\nstack at the error: 5 1\nstack: empty"));
    // Each call tells its steps and the deepest stack.
    let r = eval(&mut s, "1 2 3 + + DROP");
    assert_eq!(r["structuredContent"]["max_depth"], 3);
    assert!(r["structuredContent"]["steps"].as_u64().unwrap() > 0);
    assert!(call(&mut s, "forth_stack", json!({}))["structuredContent"].get("steps").is_none());
}

#[test]
fn the_sandbox_holds() {
    let mut s = server();
    let code =
        |s: &mut Server, forth: &str| eval(s, forth)["structuredContent"]["error"]["code"].clone();
    assert_eq!(code(&mut s, ": FOREVER BEGIN AGAIN ; FOREVER"), -258);
    assert_eq!(code(&mut s, "INCLUDE secrets.fs"), -259);
    assert_eq!(
        eval(&mut s, "S\" secrets.txt\" R/O OPEN-FILE . .")["structuredContent"]["output"],
        "-259 0 "
    );
    assert_eq!(code(&mut s, "1 BLOCK"), -259);
    // KEY gets no input: it must not read the protocol on stdin.
    assert_eq!(code(&mut s, "KEY"), -39);
    #[cfg(feature = "sqlite")]
    {
        let r = eval(&mut s, "ALSO SQLITE S\" notes.db\" DB-OPEN . .");
        assert_eq!(r["structuredContent"]["output"], "-259 0 ");
        let r = eval(
            &mut s,
            "S\" :memory:\" DB-OPEN THROW VALUE DB S\" SELECT 6*7\" DB DB-PREPARE THROW DUP DB-STEP 2DROP 0 SWAP DB-COLUMN-INT .",
        );
        assert_eq!(r["structuredContent"]["output"], "42 ");
    }
    let r = eval(&mut s, "ALSO JSON JSON[ 1 JSON-NUMBER JSON]");
    assert_eq!(r["structuredContent"]["output"], "[1]");
    // A limit of one call does not carry over to the next.
    assert_eq!(eval(&mut s, "1 2 + .")["structuredContent"]["output"], "3 ");
}

#[test]
fn the_tools_take_only_their_arguments() {
    let mut s = server();
    // No argument changes the sandbox; unknown ones are refused.
    let escape = json!({ "name": "forth_eval", "arguments": { "code": "1", "max_steps": 0 } });
    assert_eq!(error_code(&ask(&mut s, "tools/call", escape)), -32602);
    let escape = json!({ "name": "forth_reset", "arguments": { "sandbox": false } });
    assert_eq!(error_code(&ask(&mut s, "tools/call", escape)), -32602);
    assert_eq!(error_code(&ask(&mut s, "tools/call", json!({ "name": "forth_eval" }))), -32602);
    let not_text = json!({ "name": "forth_eval", "arguments": { "code": 42 } });
    assert_eq!(error_code(&ask(&mut s, "tools/call", not_text)), -32602);
    assert_eq!(error_code(&ask(&mut s, "tools/call", json!({ "name": "shell" }))), -32602);
    assert_eq!(error_code(&ask(&mut s, "resources/list", json!({}))), -32601);
    // Nonsense is answered, never a crash.
    let parse = s.handle("{not json").unwrap();
    assert!(parse.contains("-32700") && parse.contains("\"id\":null"), "{parse}");
    assert!(s.handle("[1, 2]").unwrap().contains("-32600"));
    assert!(s.handle(r#"{"id": 1}"#).unwrap().contains("-32600"));
    assert_eq!(s.handle(r#"{"jsonrpc":"2.0","id":3,"result":{}}"#), None);
}

#[test]
fn settings_have_ceilings() {
    assert_eq!(Settings::default().check(), Ok(()));
    let too_many = Settings { steps: MAX_STEPS + 1, ..Settings::default() };
    assert!(too_many.check().is_err());
    let too_long = Settings { time: Duration::from_secs(61), ..Settings::default() };
    assert!(too_long.check().is_err());
    let mut s = Server::new(Settings { steps: 1000, ..Settings::default() });
    let r = eval(&mut s, ": L 2000 0 DO LOOP ; L");
    assert_eq!(r["structuredContent"]["error"]["detail"], "steps");
}

#[test]
fn over_stdio() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_forthling-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let requests = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"forth_eval","arguments":{"code":".\" hi\" CR 6 7 *"}}}"#,
        "",
        "garbage",
    ];
    for r in requests {
        writeln!(stdin, "{r}").unwrap();
    }
    drop(stdin); // the server stops at the end of its input
    let lines: Vec<Value> = BufReader::new(child.stdout.take().unwrap())
        .lines()
        .map(|l| serde_json::from_str(&l.unwrap()).expect("every line is JSON"))
        .collect();
    assert!(child.wait().unwrap().success());
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0]["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(lines[1]["result"]["structuredContent"]["output"], "hi\n");
    assert_eq!(lines[1]["result"]["structuredContent"]["stack"], json!([42]));
    assert_eq!(lines[2]["error"]["code"], -32700);
    let help =
        Command::new(env!("CARGO_BIN_EXE_forthling-mcp")).arg("--max-time=2m").output().unwrap();
    assert_eq!(help.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&help.stderr).contains("at most 60s"));
}

#[test]
fn data_for_large_input() {
    let mut s = server();
    // A JSON document far longer than a line of code.
    let rows: Vec<Vec<u32>> =
        (1..=100).map(|n| (0..n).map(|i| (i * 7 + n) % 100).collect()).collect();
    let json = serde_json::to_string(&rows).unwrap();
    assert!(json.len() > 10_000);
    let r = call(
        &mut s,
        "forth_eval",
        json!({ "code": "ALSO JSON DATA 2DUP JSON-LENGTH . NIP .", "data": json }),
    );
    assert_eq!(r["structuredContent"]["output"], format!("100 {} ", json.len()));
    // The data stays for later calls, until other data comes.
    let r = eval(&mut s, "DATA 2DUP S\" [3]\" JSON@ DROP JSON-LENGTH . DROP FREE .");
    assert_eq!(r["structuredContent"]["output"], "4 0 ");
    let r = call(&mut s, "forth_eval", json!({ "code": "DATA TYPE", "data": "hello" }));
    assert_eq!(r["structuredContent"]["output"], "hello");
    call(&mut s, "forth_reset", json!({}));
    assert_eq!(eval(&mut s, "DATA . DROP")["structuredContent"]["output"], "0 ");
    // Data must be text, and fit in the memory.
    let not_text = json!({ "name": "forth_eval", "arguments": { "code": "1", "data": 5 } });
    assert_eq!(error_code(&ask(&mut s, "tools/call", not_text)), -32602);
    let mut small = Server::new(Settings { memory: 64 << 10, ..Settings::default() });
    let too_big =
        json!({ "name": "forth_eval", "arguments": { "code": "1", "data": "x".repeat(70_000) } });
    assert_eq!(error_code(&ask(&mut small, "tools/call", too_big)), -32602);
    let init = ask(&mut s, "initialize", json!({}));
    assert!(init["result"]["instructions"].as_str().unwrap().contains("1024 characters"));
}

/// The example of locals in the instructions runs, and `llms.txt` in the
/// root of the repository names the same limits as the server.
#[test]
fn the_instructions_and_llms_txt_agree() {
    let mut s = server();
    let init = ask(&mut s, "initialize", json!({}));
    let instructions = init["result"]["instructions"].as_str().unwrap().to_string();
    let start = instructions.find("`: HYP2").expect("an example of locals") + 1;
    let example = &instructions[start..start + instructions[start..].find('`').unwrap()];
    let r = eval(&mut s, &format!("{example} 3 4 HYP2"));
    assert_eq!(r["structuredContent"]["stack"], json!([25]), "{r}");

    let llms = include_str!("../../llms.txt");
    for fact in [
        "10000000 steps",
        "5000 ms",
        "1048576 bytes of output",
        "4194304 bytes in all",
        "1024 characters",
        "DATA ( -- c-addr u )",
    ] {
        assert!(instructions.contains(fact), "the instructions lack {fact:?}");
        assert!(llms.contains(fact), "llms.txt lacks {fact:?}");
    }
}

#[test]
fn sql_stops_in_time() {
    let mut s = Server::new(Settings { time: Duration::from_millis(300), ..Settings::default() });
    let start = std::time::Instant::now();
    let endless = "ALSO SQLITE S\" :memory:\" DB-OPEN THROW VALUE DB \
                   S\" WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM c) SELECT count(*) FROM c\" \
                   DB DB-PREPARE THROW DB-STEP";
    let r = eval(&mut s, endless);
    if cfg!(feature = "sqlite") {
        assert_eq!(r["structuredContent"]["error"]["detail"], "time");
    }
    assert!(start.elapsed() < Duration::from_secs(3), "{:?}", start.elapsed());
}
