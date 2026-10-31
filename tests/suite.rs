//! The Forth-2012 test suite (tests/forth2012-test-suite, see ROADMAP.md).

use forthling::Forth;
use std::path::{Path, PathBuf};
use std::{env, fs, io, process};

fn output(f: Forth) -> String {
    String::from_utf8(f.output().to_vec()).unwrap()
}

fn suite_file(name: &str) -> Option<String> {
    let path: PathBuf =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/forth2012-test-suite/src").join(name);
    if path.exists() {
        Some(path.to_string_lossy().into_owned())
    } else {
        eprintln!("skipped: {} not found (see ROADMAP.md)", path.display());
        None
    }
}

#[test]
fn suite_prelimtest() {
    let Some(path) = suite_file("prelimtest.fth") else { return };
    let mut f = Forth::new();
    f.include(&path).unwrap_or_else(|e| panic!("{e}\n{}", String::from_utf8_lossy(f.output())));
    let out = output(f);
    assert!(!out.contains("Error #"), "{out}");
    assert!(out.contains("Pass #23: testing S\""), "{out}");
    assert!(out.contains("\n0 tests failed out of"), "{out}");
}

#[test]
fn suite_core() {
    let (Some(tester), Some(core)) = (suite_file("tester.fr"), suite_file("core.fr")) else {
        return;
    };
    let mut f = Forth::new();
    // core.fr's ACCEPT test asks for a line of input
    f.set_input(Box::new(io::Cursor::new(b"typed line\n".to_vec())));
    f.include(&tester).unwrap();
    let result = f.include(&core);
    let out = String::from_utf8_lossy(f.output()).into_owned();
    result.unwrap_or_else(|e| panic!("{e}\n{out}"));
    assert!(!out.contains("INCORRECT RESULT"), "{out}");
    assert!(!out.contains("WRONG NUMBER OF RESULTS"), "{out}");
    assert!(out.contains("RECEIVED: \"typed line\""), "{out}");
    assert!(out.contains("End of Core word set tests"), "{out}");
    f.interpret("#ERRORS @ .").unwrap();
    assert!(output(f).ends_with("0 "));
}

/// Runs test files after what runtests.fth loads before them, checks
/// that no test failed, and returns the output; None when the suite is
/// missing.
fn run_suite(test_files: &[&str]) -> Option<String> {
    let mut files = vec!["tester.fr", "core.fr", "coreplustest.fth", "utilities.fth"];
    files.push("errorreport.fth");
    files.extend(test_files);
    let paths = files.iter().map(|name| suite_file(name)).collect::<Option<Vec<_>>>()?;
    let mut f = Forth::new();
    // core.fr's ACCEPT test asks for a line of input
    f.set_input(Box::new(io::Cursor::new(b"typed line\n".to_vec())));
    // blocktest.fth writes blocks; keep them out of the project directory
    let name = Path::new(test_files[0]).file_name().unwrap().to_string_lossy();
    let blocks = env::temp_dir().join(format!("forthling-{}-{name}.fb", process::id()));
    f.set_block_file(&blocks).unwrap();
    let result = paths.iter().try_for_each(|path| f.include(path));
    fs::remove_file(&blocks).ok();
    let out = output(f);
    result.unwrap_or_else(|e| panic!("{e}\n{out}"));
    assert!(!out.contains("INCORRECT RESULT"), "{out}");
    assert!(!out.contains("WRONG NUMBER OF RESULTS"), "{out}");
    assert!(!out.contains("FIND returns a TRUE value"), "{out}");
    assert!(out.contains("End of additional Core tests"), "{out}");
    Some(out)
}

#[test]
fn suite_core_ext() {
    let Some(out) = run_suite(&["coreexttest.fth"]) else { return };
    assert!(out.contains("End of Core Extension word tests"), "{out}");
}

#[test]
fn suite_exception() {
    let Some(out) = run_suite(&["exceptiontest.fth"]) else { return };
    assert!(out.contains("End of Exception word tests"), "{out}");
}

#[test]
fn suite_string() {
    let Some(out) = run_suite(&["stringtest.fth"]) else { return };
    assert!(out.contains("End of String word tests"), "{out}");
}

#[test]
fn suite_tools() {
    let Some(out) = run_suite(&["toolstest.fth"]) else { return };
    assert!(out.contains("End of Programming Tools word tests"), "{out}");
    // the part that needs the SEARCH word set ran too
    assert!(out.contains("NAME>INTERPRET returns an execution token"), "{out}");
}

/// filetest.fth uses words from coreexttest.fth. It creates its files in
/// the current directory and deletes them again.
#[test]
fn suite_file_access() {
    let Some(out) = run_suite(&["coreexttest.fth", "filetest.fth"]) else { return };
    assert!(out.contains("End of File-Access word set tests"), "{out}");
}

#[test]
fn suite_double() {
    let Some(out) = run_suite(&["doubletest.fth"]) else { return };
    assert!(out.contains("End of Double-Number word tests"), "{out}");
}

#[test]
fn suite_facility() {
    let Some(out) = run_suite(&["facilitytest.fth"]) else { return };
    assert!(out.contains("End of Facility word tests"), "{out}");
}

#[test]
fn suite_locals() {
    let Some(out) = run_suite(&["localstest.fth"]) else { return };
    assert!(out.contains("End of Locals word set tests"), "{out}");
    assert!(!out.contains("priority of Locals not fully tested"), "{out}");
}

#[test]
fn suite_search() {
    let Some(out) = run_suite(&["searchordertest.fth"]) else { return };
    assert!(out.contains("End of Search Order word tests"), "{out}");
}

#[test]
fn suite_block() {
    let Some(out) = run_suite(&["blocktest.fth"]) else { return };
    assert!(out.contains("End of Block word tests"), "{out}");
}

/// The floating-point tests in fp/, in the order of runfptests.fth. They use
/// their own tester, ttester.fs.
#[test]
fn suite_floating() {
    let files = [
        "fp/ttester.fs",
        "fp/fatan2-test.fs",
        "fp/ieee-arith-test.fs",
        "fp/ieee-fprox-test.fs",
        "fp/fpzero-test.4th",
        "fp/fpio-test.4th",
        "fp/to-float-test.4th",
        "fp/paranoia.4th",
        "fp/ak-fp-test.fth",
    ];
    let Some(paths) = files.iter().map(|name| suite_file(name)).collect::<Option<Vec<_>>>() else {
        return;
    };
    let mut f = Forth::new();
    f.set_input(Box::new(io::Cursor::new(Vec::new())));
    let result = paths.iter().try_for_each(|path| {
        if path.ends_with("fpzero-test.4th") {
            // fpzero-test.4th uses F>, which is not a standard word
            f.interpret("[UNDEFINED] F> [IF] : F> FSWAP F< ; [THEN]")?;
        }
        f.include(path)
    });
    let out = output(f);
    result.unwrap_or_else(|e| panic!("{e}\n{out}"));
    assert!(!out.contains("INCORRECT"), "{out}");
    assert!(!out.contains("WRONG NUMBER"), "{out}");
    assert_eq!(out.matches("#ERRORS: ").count(), 5, "{out}");
    assert_eq!(out.matches("#ERRORS: 0 ").count(), 5, "{out}");
    assert!(out.contains("No failures, defects nor flaws have been discovered."), "{out}");
    assert!(out.contains("You might see 0.00024 : 0.00024 "), "{out}");
    assert!(out.contains("End of ak-fp-test.fth"), "{out}");
}

/// The XCHAR tests are forthling's own (tests/xchartest.fth); the suite has none.
#[test]
fn suite_xchar() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/xchartest.fth");
    let Some(out) = run_suite(&[path.to_str().unwrap()]) else { return };
    assert!(out.contains("End of XCHAR word tests"), "{out}");
}

#[test]
fn suite_memory() {
    let Some(out) = run_suite(&["memorytest.fth"]) else { return };
    assert!(out.contains("End of Memory-Allocation word tests"), "{out}");
}

#[test]
fn suite_tester() {
    let Some(path) = suite_file("tester.fr") else { return };
    let mut f = Forth::new();
    f.include(&path).unwrap();
    f.interpret("T{ 1 2 + -> 3 }T  T{ 1 2 SWAP -> 2 1 }T  T{ -> }T").unwrap();
    f.interpret("#ERRORS @ .").unwrap();
    assert_eq!(String::from_utf8_lossy(f.output()), "0 ");
    f.interpret("T{ 1 2 + -> 4 }T").unwrap();
    f.interpret("T{ 1 2 -> 3 }T").unwrap();
    f.interpret("#ERRORS @ .").unwrap();
    let out = output(f);
    assert!(out.contains("INCORRECT RESULT: T{ 1 2 + -> 4 }T"), "{out}");
    assert!(out.contains("WRONG NUMBER OF RESULTS: T{ 1 2 -> 3 }T"), "{out}");
    assert!(out.ends_with("2 "), "{out}");
}
