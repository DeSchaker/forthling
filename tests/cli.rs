//! The forthling program, run as a process: the debugger in the terminal.

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Runs the program on `file` (Forth source) with `input` on stdin.
fn run(file: &str, input: &str) -> String {
    static RUNS: AtomicUsize = AtomicUsize::new(0);
    let n = RUNS.fetch_add(1, Ordering::Relaxed);
    let name = format!("forthling-cli-{}-{n}.fs", std::process::id());
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, file).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_forthling"))
        .arg(&path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    std::fs::remove_file(&path).ok();
    String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr)
}

const PROGRAM: &str = "ALSO DEBUG\n: SQ DUP * ;\n: F 3 SQ 1+ ;\n: MAIN BREAK F 2 SQ + ;\nMAIN .\n";
const PROMPT: &str = "s step, n over, f out, c continue, q quit> ";

#[test]
fn the_debugger_steps_through_a_program() {
    // The input is not echoed here, as a terminal would.
    let out = run(PROGRAM, "s\ns\nf\nn\nc\n");
    let expected = [
        "break: MAIN > BREAK  <0>\n",
        "step: MAIN > F  <0>\n",
        "step: MAIN > F > SQ  <1> 3\n",
        "step: MAIN > SQ  <2> 10 2\n",
        "step: MAIN > +  <2> 10 4\n",
        "14 ",
    ]
    .join(PROMPT);
    assert_eq!(out, expected);
    // after output that does not end a line, a pause starts a new one
    let out = run("ALSO DEBUG 7 . BREAK 8 .", "c\n");
    assert_eq!(out, format!("7 \nbreak: BREAK  <0>\n{PROMPT}8 "));
}

#[test]
fn quit_interrupts_and_the_end_of_input_continues() {
    let out = run(PROGRAM, "q\n");
    assert!(out.ends_with("error -28: user interrupt\n"), "{out:?}");
    // an unknown command asks again; no input at all runs on
    let out = run(PROGRAM, "x\n");
    assert!(out.ends_with(&format!("{PROMPT}{PROMPT}14 ")), "{out:?}");
}
