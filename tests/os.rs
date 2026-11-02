//! The OPERATING-SYSTEM words, used from Forth.

use forthling::{Cell, Forth, NOT_PERMITTED, OperatingSystem, Permission};
use std::io::Cursor;

fn forth() -> Forth {
    let mut f = Forth::new();
    f.set_input(Box::new(Cursor::new(Vec::new())));
    f.install(&OperatingSystem);
    f.interpret("ALSO OPERATING-SYSTEM").unwrap();
    f
}

fn run(f: &mut Forth, text: &str) -> String {
    let start = f.output().len();
    f.interpret(text).unwrap_or_else(|e| panic!("{text}: {e}"));
    String::from_utf8_lossy(&f.output()[start..]).into_owned()
}

fn code(f: &mut Forth, text: &str) -> Cell {
    f.interpret(text).expect_err(text).code
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("forthling-os-{}-{name}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    dir
}

#[test]
fn the_word_list_is_closed_until_asked_for() {
    let mut f = Forth::new();
    f.install(&OperatingSystem);
    assert_eq!(code(&mut f, "S\" x\" GETENV"), -13);
    f.interpret("ALSO OPERATING-SYSTEM").unwrap();
    f.interpret("S\" x\" GETENV 2DROP").unwrap();
}

#[test]
fn system_runs_a_command_and_keeps_its_status() {
    let mut f = forth();
    // The command writes to the real standard output, not to the Forth output.
    assert_eq!(run(&mut f, "S\" echo hi\" SYSTEM $? ."), "0 ");
    assert_eq!(run(&mut f, "S\" exit 3\" SYSTEM $? ."), "3 ");
    assert_eq!(run(&mut f, "SH exit 4"), "");
    assert_eq!(run(&mut f, "$? ."), "4 ");
}

#[test]
fn getenv_gives_the_value_or_nothing() {
    let mut f = forth();
    // Cargo sets this for every test run, so the test needs no `set_var` (which is `unsafe`).
    let name = "CARGO_PKG_NAME";
    assert_eq!(run(&mut f, &format!("S\" {name}\" GETENV TYPE")), "forthling");
    assert_eq!(run(&mut f, "S\" FORTHLING_OS_NOT_SET\" GETENV NIP ."), "0 ");
}

#[test]
fn the_arguments_are_in_argc_argv_and_arg() {
    let mut f = forth();
    f.set_arguments(["prog", "one", "two"].map(String::from).to_vec());
    assert_eq!(run(&mut f, "ARGC @ ."), "3 ");
    assert_eq!(run(&mut f, "1 ARG TYPE SPACE 2 ARG TYPE"), "one two");
    assert_eq!(run(&mut f, "3 ARG NIP ."), "0 ");
    assert_eq!(run(&mut f, "ARGV @ 1 CELLS + @ ARGV @ @ - 0<> ."), "-1 ");
    assert_eq!(run(&mut f, "NEXT-ARG TYPE ARGC @ ."), "one2 ");
    assert_eq!(run(&mut f, "SHIFT-ARGS ARGC @ . NEXT-ARG NIP ."), "1 0 ");
}

#[test]
fn paths_are_split_and_matched() {
    let mut f = forth();
    assert_eq!(run(&mut f, "S\" a/b/c.fs\" BASENAME TYPE"), "c.fs");
    assert_eq!(run(&mut f, "S\" a/b/c.fs\" DIRNAME TYPE"), "a/b/");
    assert_eq!(run(&mut f, "S\" c.fs\" DIRNAME NIP ."), "0 ");
    assert_eq!(run(&mut f, "S\" c.fs\" S\" *.fs\" FILENAME-MATCH ."), "-1 ");
    assert_eq!(run(&mut f, "S\" c.fs\" S\" ?.f?\" FILENAME-MATCH ."), "-1 ");
    assert_eq!(run(&mut f, "S\" c.fs\" S\" *.4th\" FILENAME-MATCH ."), "0 ");
    assert_eq!(run(&mut f, "S\" \" S\" *\" FILENAME-MATCH ."), "-1 ");
}

#[test]
fn directories_are_made_read_and_changed() {
    let dir = scratch("dirs");
    let path = dir.join("a").join("b");
    let (path, root) =
        (path.to_string_lossy().replace('\\', "/"), dir.to_string_lossy().replace('\\', "/"));
    let mut f = forth();
    assert_eq!(run(&mut f, &format!("S\" {path}\" 511 MKDIR-PARENTS .")), "0 ");
    assert_eq!(run(&mut f, &format!("S\" {path}\" 511 MKDIR-PARENTS .")), "0 "); // exists already
    std::fs::write(dir.join("a").join("x.txt"), "x").unwrap();
    f.interpret("CREATE BUF 64 ALLOT").unwrap();
    f.interpret(&format!("S\" {root}/a\" OPEN-DIR DROP CONSTANT D")).unwrap();
    run(&mut f, ": NEXT BUF 64 D READ-DIR . IF BUF SWAP TYPE ELSE DROP .\" END\" THEN ;");
    let mut names = Vec::new();
    loop {
        let out = run(&mut f, "NEXT");
        if out.ends_with("END") {
            assert_eq!(out, "0 END");
            break;
        }
        names.push(out);
        assert!(names.len() < 10, "{names:?}");
    }
    names.sort();
    assert_eq!(names, ["0 b", "0 x.txt"]);
    assert_eq!(run(&mut f, "D CLOSE-DIR ."), "0 ");
    assert_ne!(run(&mut f, "D CLOSE-DIR ."), "0 "); // closed twice
    // The process directory is shared by the tests, so SET-DIR goes to where we are.
    let here = std::env::current_dir().unwrap().to_string_lossy().replace('\\', "/");
    assert_eq!(run(&mut f, &format!("S\" {here}\" SET-DIR .")), "0 ");
    assert_eq!(run(&mut f, "PAD 512 GET-DIR NIP 0<> ."), "-1 ");
    assert_ne!(run(&mut f, &format!("S\" {root}/nothing\" SET-DIR .")), "0 ");
    assert_ne!(run(&mut f, "S\" /no/such/dir\" OPEN-DIR NIP ."), "0 ");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_pipe_reads_the_output_of_a_command() {
    let mut f = forth();
    f.interpret("CREATE LINE 80 ALLOT").unwrap();
    assert_eq!(
        run(
            &mut f,
            "S\" echo piped\" R/O OPEN-PIPE . CONSTANT P LINE 80 P READ-LINE . . LINE SWAP TYPE"
        ),
        "0 0 -1 piped"
    );
    assert_eq!(run(&mut f, "LINE 80 P READ-LINE . . DROP"), "0 0 ");
    assert_eq!(run(&mut f, "P CLOSE-PIPE . ."), "0 0 ");
}

#[test]
fn a_pipe_writes_to_a_command() {
    let mut f = forth();
    let out = scratch("sink");
    let sink = out.to_string_lossy().replace('\\', "/");
    let command =
        if cfg!(windows) { format!("findstr x > {sink}") } else { format!("cat > {sink}") };
    run(&mut f, &format!("S\" {command}\" W/O OPEN-PIPE . CONSTANT P"));
    assert_eq!(run(&mut f, "S\" xyz\" P WRITE-LINE ."), "0 ");
    assert_eq!(run(&mut f, "P CLOSE-PIPE . ."), "0 0 ");
    assert_eq!(std::fs::read_to_string(&out).unwrap().trim(), "xyz");
    std::fs::remove_file(&out).unwrap();
}

#[test]
fn stdout_is_the_output_of_the_system() {
    let mut f = forth();
    assert_eq!(run(&mut f, "S\" a\" STDOUT WRITE-LINE . S\" b\" TYPE"), "a\n0 b");
    assert_eq!(run(&mut f, ": HELLO .\" hi\" ; ' HELLO STDOUT OUTFILE-EXECUTE"), "hi");
    assert_eq!(run(&mut f, "STDOUT STDOUT = ."), "-1 ");
}

#[test]
fn stdin_is_the_input_of_the_system() {
    let mut f = Forth::new();
    f.set_input(Box::new(Cursor::new(b"first\nsecond\n".to_vec())));
    f.install(&OperatingSystem);
    f.interpret("ALSO OPERATING-SYSTEM CREATE LINE 80 ALLOT").unwrap();
    assert_eq!(run(&mut f, "LINE 80 STDIN READ-LINE . . LINE SWAP TYPE"), "0 -1 first");
    assert_eq!(run(&mut f, "LINE 80 STDIN READ-LINE . . LINE SWAP TYPE"), "0 -1 second");
    assert_eq!(run(&mut f, "LINE 80 STDIN READ-LINE . . DROP"), "0 0 ");
}

#[test]
fn infile_execute_feeds_a_file_to_key_and_accept() {
    let mut f = forth();
    run(
        &mut f,
        "S\" in.txt\" W/O CREATE-FILE DROP CONSTANT W S\" line one\" W WRITE-LINE DROP S\" two\" W WRITE-LINE DROP W CLOSE-FILE DROP",
    );
    run(&mut f, "S\" in.txt\" R/O OPEN-FILE DROP CONSTANT R CREATE B 40 ALLOT");
    assert_eq!(run(&mut f, ": GO B 40 ACCEPT B SWAP TYPE ; ' GO R INFILE-EXECUTE"), "line one");
    // The rest of the file is still there for the file words.
    assert_eq!(run(&mut f, "B 40 R READ-LINE . . B SWAP TYPE"), "0 -1 two");
    run(&mut f, "R CLOSE-FILE DROP S\" in.txt\" DELETE-FILE DROP");
}

#[test]
fn outfile_execute_sends_output_to_a_file() {
    let mut f = forth();
    run(&mut f, "S\" out.txt\" W/O CREATE-FILE DROP CONSTANT W");
    assert_eq!(run(&mut f, ": GO .\" captured\" ; ' GO W OUTFILE-EXECUTE"), "");
    run(&mut f, "W CLOSE-FILE DROP");
    assert_eq!(std::fs::read_to_string("out.txt").unwrap(), "captured");
    std::fs::remove_file("out.txt").unwrap();
}

#[test]
fn a_denied_system_refuses() {
    let mut f = forth();
    f.permit(Permission::System, false);
    for text in [
        "S\" echo\" SYSTEM",
        "SH echo",
        "S\" PATH\" GETENV",
        "0 ARG",
        "NEXT-ARG",
        "SHIFT-ARGS",
        "STDERR",
        "PAD 99 GET-DIR",
    ] {
        assert_eq!(code(&mut f, text), NOT_PERMITTED, "{text}");
    }
    // The words with an ior give it as the ior.
    assert_eq!(run(&mut f, "S\" echo\" R/O OPEN-PIPE . ."), format!("{NOT_PERMITTED} 0 "));
    assert_eq!(run(&mut f, "S\" .\" OPEN-DIR . ."), format!("{NOT_PERMITTED} 0 "));
    assert_eq!(run(&mut f, "S\" .\" SET-DIR ."), format!("{NOT_PERMITTED} "));
    assert_eq!(run(&mut f, "S\" x\" 511 MKDIR-PARENTS ."), format!("{NOT_PERMITTED} "));
    // What only works on text, and the system's own input and output, stay.
    assert_eq!(run(&mut f, "S\" a/b\" BASENAME TYPE STDOUT 0<> ."), "b-1 ");
}

#[test]
fn a_sandboxed_system_has_no_machine() {
    let mut f = Forth::sandboxed();
    f.install(&OperatingSystem);
    assert!(!f.permits(Permission::System));
    f.interpret("ALSO OPERATING-SYSTEM").unwrap();
    assert_eq!(code(&mut f, "S\" echo hi\" SYSTEM"), NOT_PERMITTED);
}

#[test]
fn a_shebang_line_is_not_forth() {
    let mut f = Forth::new();
    let fs = forthling::MemoryFs::new();
    fs.insert("s.fs", b"#! /usr/bin/env forthling\n1 2 + .\n".to_vec()).unwrap();
    f.set_file_system(Box::new(fs)).unwrap();
    f.interpret("S\" s.fs\" INCLUDED").unwrap();
    assert_eq!(String::from_utf8_lossy(f.output()), "3 ");
}
