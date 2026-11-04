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

#[test]
fn the_sandbox_has_files_in_memory() {
    let dir = std::env::temp_dir().join(format!("forthling-cli-sandbox-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("lib.fs"), ": SQ DUP * ;").unwrap();
    let program = "INCLUDE lib.fs 6 SQ .\n\
        S\" out.txt\" W/O CREATE-FILE THROW VALUE OUT  S\" hi\" OUT WRITE-FILE THROW  OUT CLOSE-FILE THROW\n\
        CREATE B 9 ALLOT  S\" out.txt\" R/O OPEN-FILE THROW VALUE IN  B 9 IN READ-FILE THROW B SWAP TYPE\n\
        S\" main.fs\" FILE-STATUS NIP .  S\" other.fs\" FILE-STATUS NIP .\n";
    std::fs::write(dir.join("main.fs"), program).unwrap();
    std::fs::write(dir.join("other.fs"), "1").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_forthling"))
        .args(["--sandbox", "lib.fs", "main.fs"])
        .current_dir(&dir)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    // The files given are there, what it writes it can read back, the other
    // file on the disk is not there, and nothing is written to the disk.
    assert_eq!(text, "36 hi0 -67 ");
    assert!(!dir.join("out.txt").exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_functional_words_are_installed() {
    let out = run("ALSO FUNCTIONAL\n1 6 RANGE [: DUP * ;] MAP 5 ' + FOLD .\n", "");
    assert!(out.contains("55 "), "{out}");
}

/// Runs the program on several files (name, source) with `--sandbox` or not.
fn run_files(files: &[(&str, &str)], flags: &[&str]) -> (String, String, i32) {
    static DIRS: AtomicUsize = AtomicUsize::new(0);
    let n = DIRS.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("forthling-cli-files-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let paths: Vec<_> = files
        .iter()
        .map(|(name, source)| {
            let path = dir.join(name);
            std::fs::write(&path, source).unwrap();
            path
        })
        .collect();
    let out = Command::new(env!("CARGO_BIN_EXE_forthling"))
        .args(flags)
        .args(&paths)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    std::fs::remove_dir_all(&dir).ok();
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

#[test]
fn a_script_takes_the_rest_of_the_files_as_arguments() {
    let script = "#! /usr/bin/env forthling\nALSO OPERATING-SYSTEM\nARGC @ .\nNEXT-ARG BASENAME TYPE SPACE ARGC @ .\n";
    let (out, err, code) = run_files(&[("one.fs", script), ("two.fs", ".\" never\"")], &[]);
    assert_eq!((out.as_str(), err.as_str(), code), ("2 two.fs 1 ", "", 0));
}

#[test]
fn files_are_included_in_order_without_arguments_taken() {
    let (out, _, code) = run_files(&[("a.fs", ".\" a\""), ("b.fs", ".\" b\"")], &[]);
    assert_eq!((out.as_str(), code), ("ab", 0));
}

#[test]
fn stderr_goes_to_standard_error_in_order() {
    let source = "ALSO OPERATING-SYSTEM\n.\" out\" S\" err\" STDERR WRITE-LINE DROP\n";
    let (out, err, _) = run_files(&[("e.fs", source)], &[]);
    assert_eq!((out.as_str(), err.trim()), ("out", "err"));
}

#[test]
fn the_sandbox_has_no_machine() {
    let source = "ALSO OPERATING-SYSTEM\nS\" echo hi\" SYSTEM\n";
    let (out, err, code) = run_files(&[("s.fs", source)], &["--sandbox"]);
    assert_eq!(out, "");
    assert!(err.contains("-259") || err.to_lowercase().contains("permitted"), "{err}");
    assert_eq!(code, 1);
}

#[test]
fn test_mode_reports_the_tests_and_sets_the_exit_code() {
    let good = "ALSO TESTING\nT{ 1 2 + -> 3 }T\nT{ 2 2 * -> 4 }T\n";
    let (out, _, code) = run_files(&[("good.fs", good)], &["--test"]);
    assert_eq!((out.as_str(), code), ("tests: 2 passed, 0 failed\n", 0));

    let bad = "ALSO TESTING\nTESTING sums\nT{ 1 2 + -> 4 }T\nT{ 1 1 + -> 2 }T\n";
    let (out, _, code) = run_files(&[("bad.fs", bad)], &["--test"]);
    assert_eq!(code, 1);
    assert!(out.contains("bad.fs:3 (sums): incorrect result: expected <1> 4, got <1> 3"), "{out}");
    assert!(out.ends_with("tests: 1 passed, 1 failed\n"), "{out}");

    // The counts run over all the files; an error ends the run with exit code 1.
    let (out, err, code) =
        run_files(&[("a.fs", good), ("b.fs", "T{ 1 0 / -> 0 }T\n")], &["--test"]);
    assert_eq!(code, 1);
    assert!(out.ends_with("tests: 2 passed, 0 failed\n") && err.contains("-10"), "{out} {err}");

    // Without --test nothing is counted or printed, and the exit code is 0.
    let (out, _, code) = run_files(&[("bad.fs", bad)], &[]);
    assert_eq!(code, 0);
    assert!(out.contains("FAIL ") && !out.contains("tests:"), "{out}");
}

#[test]
fn test_mode_needs_a_file() {
    let out = Command::new(env!("CARGO_BIN_EXE_forthling")).arg("--test").output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("--test needs a file"));
}
