//! The sandbox: limits and permissions, through the public API.

use forthling::{Forth, LIMIT_EXCEEDED, Limits, NOT_PERMITTED, Permission, Throw};
use std::time::{Duration, Instant};
use std::{env, fs};

fn limited(limits: Limits) -> Forth {
    let mut f = Forth::new();
    f.set_limits(limits);
    f
}

fn steps(n: u64) -> Limits {
    Limits { steps: Some(n), ..Limits::default() }
}

fn fails(f: &mut Forth, text: &str) -> Throw {
    f.interpret(text).expect_err(text)
}

fn output(f: &Forth) -> String {
    String::from_utf8_lossy(f.output()).into_owned()
}

fn temp_file(name: &str, content: &str) -> String {
    let path = env::temp_dir().join(format!("forthling-sandbox-{}-{name}", std::process::id()));
    fs::write(&path, content).unwrap();
    path.to_string_lossy().into_owned()
}

#[test]
fn the_step_limit_stops_endless_loops() {
    let mut f = limited(steps(100_000));
    let e = fails(&mut f, ": FOREVER BEGIN AGAIN ; FOREVER");
    assert_eq!(
        (e.code, e.to_string()),
        (LIMIT_EXCEEDED, "error -258: limit exceeded: steps".into())
    );
    // Each evaluation starts with the full budget.
    f.interpret(": COUNTED 1000 0 DO LOOP ; COUNTED").unwrap();
    // The limit is exact: every word executed is a step (numbers are not).
    let mut f = limited(steps(2));
    f.interpret("1 2 + DROP").unwrap();
    assert_eq!(fails(&mut f, "1 2 + DROP 3 DROP").code, LIMIT_EXCEEDED);
}

#[test]
fn catch_cannot_go_on_after_a_limit() {
    let mut f = limited(steps(100_000));
    f.interpret(": FOREVER BEGIN AGAIN ;").unwrap();
    assert_eq!(fails(&mut f, "' FOREVER CATCH .").code, LIMIT_EXCEEDED);
    assert_eq!(output(&f), "");
    let escape = ": ESCAPE BEGIN ['] FOREVER CATCH DROP AGAIN ; ESCAPE";
    assert_eq!(fails(&mut f, escape).code, LIMIT_EXCEEDED);
}

#[test]
fn the_time_limit() {
    let mut f = limited(Limits { time: Some(Duration::from_millis(50)), ..Limits::default() });
    let start = Instant::now();
    let e = fails(&mut f, ": FOREVER BEGIN AGAIN ; FOREVER");
    assert_eq!(e.to_string(), "error -258: limit exceeded: time");
    assert!(start.elapsed() < Duration::from_secs(5));
    // MS that would pass the limit throws at once instead of sleeping.
    let start = Instant::now();
    assert_eq!(fails(&mut f, "100000 MS").code, LIMIT_EXCEEDED);
    assert!(start.elapsed() < Duration::from_secs(5));
    f.interpret("1 MS").unwrap();
}

#[test]
fn the_output_limit() {
    let mut f = limited(Limits { output: Some(10), ..Limits::default() });
    let e = fails(&mut f, ": STARS 100 0 DO 42 EMIT LOOP ; STARS");
    assert_eq!(e.to_string(), "error -258: limit exceeded: output");
    assert_eq!(output(&f), "**********");
    // Output that passes the limit at the very end still fails the evaluation.
    let mut f = limited(Limits { output: Some(3), ..Limits::default() });
    assert_eq!(fails(&mut f, "S\" hello\" TYPE").code, LIMIT_EXCEEDED);
    assert_eq!(output(&f), "hel");
    f.interpret(".( ok)").unwrap();
    assert_eq!(output(&f), "helok");
}

#[test]
fn a_host_word_shares_the_budget() {
    let mut f = limited(steps(50_000));
    let wid = f.wordlist("FORTH");
    f.add_word(wid, "INNER", |f| f.interpret(": L 100 0 DO LOOP ; L")).unwrap();
    // 1000 INNERs take some 300000 steps; if each one started with a new
    // budget, this would never stop.
    assert_eq!(fails(&mut f, ": OUTER 1000 0 DO INNER LOOP ; OUTER").code, LIMIT_EXCEEDED);
}

#[test]
fn files_can_be_forbidden() {
    let notes = temp_file("notes.txt", "hello");
    let mut f = Forth::new();
    f.permit(Permission::Files, false);
    assert!(!f.permits(Permission::Files));
    let open = format!("S\" {notes}\" R/O OPEN-FILE . .");
    f.interpret(&open).unwrap();
    assert_eq!(output(&f), "-259 0 ");
    f.interpret(&format!("S\" {notes}\" FILE-STATUS . DROP")).unwrap();
    f.interpret(&format!("S\" {notes}\" DELETE-FILE .")).unwrap();
    assert!(output(&f).ends_with("-259 -259 "));
    assert!(fs::metadata(&notes).is_ok());
    let e = fails(&mut f, &format!("S\" {notes}\" INCLUDED"));
    assert_eq!(
        (e.code, e.to_string()),
        (NOT_PERMITTED, format!("error -259: not permitted: INCLUDED {notes}"))
    );
    assert_eq!(fails(&mut f, "1 BLOCK").code, NOT_PERMITTED);
    // The host may still run a file; the Forth code in it may not open others.
    let script = temp_file("script.fs", &format!(".( ran) S\" {notes}\" INCLUDED"));
    assert_eq!(f.include(&script).unwrap_err().code, NOT_PERMITTED);
    assert!(output(&f).ends_with("ran"));
    f.permit(Permission::Files, true);
    let before = output(&f).len();
    f.interpret(&open).unwrap();
    assert!(output(&f)[before..].starts_with("0 "), "{}", output(&f)); // ior 0, then the fileid
    fs::remove_file(&notes).unwrap();
    fs::remove_file(&script).unwrap();
}

#[test]
fn a_sandboxed_system() {
    let mut f = Forth::sandboxed();
    assert_eq!(f.limits(), Limits::SANDBOX);
    // Files, but only in memory (tests/vfs.rs).
    assert!(f.permits(Permission::Files));
    assert_eq!(fails(&mut f, "KEY").code, -39); // no terminal input: end of input
    assert_eq!(fails(&mut f, ": FOREVER BEGIN AGAIN ; FOREVER").code, LIMIT_EXCEEDED);
    f.interpret(": SQUARE DUP * ; 12 SQUARE .").unwrap();
    assert_eq!(output(&f), "144 ");
    assert_eq!(Forth::new().limits(), Limits::default());
}

#[test]
fn big_words_cost_steps_in_proportion() {
    // A MOVE of 64 KiB costs 1024 steps, so ten of them pass 10000.
    let mut f = Forth::with_memory(1 << 20);
    f.set_limits(steps(10_000));
    let moves = "CREATE BUF 131072 ALLOT : M 10 0 DO BUF BUF 65536 + 65536 MOVE LOOP ; M";
    assert_eq!(fails(&mut f, moves).code, LIMIT_EXCEEDED);
    // The time limit holds for them too, not only after 4096 of them.
    let mut f = Forth::with_memory(4 << 20);
    f.set_limits(Limits { time: Some(Duration::from_millis(200)), ..Limits::default() });
    let start = Instant::now();
    let copies = "CREATE BUF 2097152 ALLOT : C BEGIN BUF BUF 1048576 + 1048576 MOVE AGAIN ; C";
    assert_eq!(fails(&mut f, copies).to_string(), "error -258: limit exceeded: time");
    assert!(start.elapsed() < Duration::from_secs(2), "{:?}", start.elapsed());
}

#[test]
fn search_takes_linear_time() {
    // aaaa…a in aaaa…a: a naive search would compare some 10^10 bytes.
    let mut f = Forth::with_memory(4 << 20);
    let start = Instant::now();
    let search = ": A ( n -- c-addr n ) HERE OVER ALLOT SWAP 2DUP [CHAR] a FILL ; \
                  1000000 A 200000 A 2DUP + 1- CHAR b SWAP C! SEARCH . 2DROP";
    f.interpret(search).unwrap();
    assert_eq!(output(&f), "0 ");
    assert!(start.elapsed() < Duration::from_secs(2), "{:?}", start.elapsed());
    let mut f = Forth::new();
    f.interpret(r#"S" hello world" S" o w" SEARCH . TYPE  S" abc" S" " SEARCH . TYPE"#).unwrap();
    assert_eq!(output(&f), "-1 o world-1 abc");
}

#[test]
fn the_space_limit() {
    let mut f = limited(Limits { space: Some(20_000), ..Limits::default() });
    assert_eq!(f.space(), 0);
    f.interpret(": DEF S\" : X 1 2 3 ;\" EVALUATE ;").unwrap();
    let used = f.space();
    assert!(used > 0 && used < 1000, "{used}");
    let e = fails(&mut f, ": MANY 10000 0 DO DEF LOOP ; MANY");
    assert_eq!(e.to_string(), "error -258: limit exceeded: space");
    assert!(f.space() <= 20_000);
    // MARKER gives the space back.
    let mut f = limited(Limits { space: Some(20_000), ..Limits::default() });
    f.interpret(": DEF S\" : X 1 2 3 ;\" EVALUATE ;").unwrap();
    let before = f.space();
    f.interpret("MARKER GONE : MORE 50 0 DO DEF LOOP ; MORE").unwrap();
    assert!(f.space() > before);
    f.interpret("GONE").unwrap();
    assert_eq!(f.space(), before);
    // So do the words of a definition that went wrong, and code being compiled
    // counts while it grows.
    f.interpret(": GEN 0 DO POSTPONE DUP LOOP ; IMMEDIATE").unwrap();
    let before = f.space();
    assert_eq!(fails(&mut f, ": BIG [ 1000000 ] GEN ;").code, LIMIT_EXCEEDED);
    assert_eq!(f.space(), before);
    assert_eq!(
        fails(&mut f, ": LISTS 100000 0 DO WORDLIST DROP LOOP ; LISTS").code,
        LIMIT_EXCEEDED
    );
    // Replacing a REPLACES text takes no more space.
    let mut f = limited(Limits { space: Some(20_000), ..Limits::default() });
    f.interpret(r#": R 1000 0 DO S" some text" S" name" REPLACES LOOP ; R"#).unwrap();
    assert!(f.space() < 20_000);
}
