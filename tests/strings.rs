//! The STRINGS words, used from Forth.

use forthling::{Cell, Forth, Limits, MemoryFs, Strings};
use std::io::Cursor;

fn forth() -> Forth {
    let mut f = Forth::new();
    f.set_input(Box::new(Cursor::new(Vec::new())));
    f.install(&Strings);
    f.interpret("ALSO DYNAMIC-STRINGS").unwrap();
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

#[test]
fn store_fetch_and_free() {
    let mut f = forth();
    run(&mut f, "$VARIABLE S");
    assert_eq!(run(&mut f, "S $@ TYPE S $@LEN ."), "0 ");
    assert_eq!(run(&mut f, r#"S" hello" S $! S $@ TYPE SPACE S $@LEN ."#), "hello 5 ");
    assert_eq!(run(&mut f, r#"S" world" S $! S $@ TYPE"#), "world");
    assert_eq!(run(&mut f, "S $FREE S @ . S $@LEN ."), "0 0 ");
    // storing a string into itself
    run(&mut f, r#"S" abc" S $!"#);
    assert_eq!(run(&mut f, "S $@ S $! S $@ TYPE"), "abc");
    assert_eq!(run(&mut f, "S $@ S $+! S $@ TYPE"), "abcabc");
    assert_eq!(run(&mut f, "S $INIT S $@LEN ."), "0 ");
}

#[test]
fn append_insert_delete_and_length() {
    let mut f = forth();
    run(&mut f, "$VARIABLE S");
    assert_eq!(run(&mut f, r#"S" foo" S $+! S" bar" S $+! S $@ TYPE"#), "foobar");
    assert_eq!(run(&mut f, r#"S" XY" S 3 $INS S $@ TYPE"#), "fooXYbar");
    assert_eq!(run(&mut f, r#"S" >" S 0 $INS S" <" S 99 $INS S $@ TYPE"#), ">fooXYbar<");
    assert_eq!(run(&mut f, "S 4 2 $DEL S $@ TYPE"), ">foobar<");
    assert_eq!(run(&mut f, "S 7 99 $DEL S -5 1 $DEL S $@ TYPE"), "foobar");
    assert_eq!(run(&mut f, "3 S $!LEN S $@ TYPE S $@LEN ."), "foo3 ");
    assert_eq!(run(&mut f, "5 S $!LEN S $@ + 1- C@ ."), "0 ");
    assert_eq!(run(&mut f, "2 S $+!LEN S $@LEN . S $@ + 1- C@ ."), "7 0 ");
    assert_eq!(run(&mut f, "0 S $!LEN S @ ."), "0 ");
    assert_eq!(code(&mut f, "-1 S $!LEN"), -24);
}

#[test]
fn split_and_iterate() {
    let mut f = forth();
    assert_eq!(run(&mut f, r#"S" key=value" 61 $SPLIT 2SWAP TYPE 124 EMIT TYPE"#), "key|value");
    assert_eq!(run(&mut f, r#"S" nosplit" 61 $SPLIT NIP . NIP ."#), "0 7 ");
    run(&mut f, "$VARIABLE S  : P ( a u -- ) 60 EMIT TYPE 62 EMIT ;");
    assert_eq!(run(&mut f, r#"S" a,bb,,c" S $! S 44 ' P $ITER"#), "<a><bb><><c>");
    assert_eq!(run(&mut f, r#"S" a," S $! S 44 ' P $ITER"#), "<a><>");
    assert_eq!(run(&mut f, "S $FREE S 44 ' P $ITER"), "");
}

#[test]
fn output_into_strings() {
    let mut f = forth();
    run(&mut f, "$VARIABLE S");
    run(&mut f, r#": T ." n=" 42 . ;"#);
    assert_eq!(run(&mut f, "' T $TMP TYPE"), "n=42 ");
    assert_eq!(run(&mut f, "' T S $EXEC ' T S $EXEC S $@ TYPE"), "n=42 n=42 ");
    // the output of a word does not reach the screen
    assert_eq!(run(&mut f, "' T S $EXEC"), "");
    // an error discards what was caught
    run(&mut f, r#": B ." lost" -1 THROW ;"#);
    assert_eq!(code(&mut f, "' B $TMP"), -1);
    assert_eq!(run(&mut f, r"1 2 +  ."), "3 ");
}

#[test]
fn arrays() {
    let mut f = forth();
    run(&mut f, "$[]VARIABLE A  : P ( a u -- ) 60 EMIT TYPE 62 EMIT ;");
    assert_eq!(run(&mut f, "A $[]# ."), "0 ");
    assert_eq!(run(&mut f, r#"S" one" A $[]+! S" two" A $[]+! A $[]# ."#), "2 ");
    assert_eq!(run(&mut f, "0 A $[]@ TYPE 1 A $[]@ TYPE"), "onetwo");
    assert_eq!(
        run(&mut f, r#"S" four" 3 A $[]! A $[]# . 2 A $[]@ NIP . A ' P $[]MAP"#),
        "4 0 <one><two><><four>"
    );
    // reading past the end does not grow the array
    assert_eq!(run(&mut f, "9 A $[]@ NIP . A $[]# ."), "0 4 ");
    assert_eq!(code(&mut f, "-1 A $[]"), -24);
    assert_eq!(run(&mut f, "A $[]FREE A $[]# . A @ ."), "0 0 ");
}

#[test]
fn files() {
    let mut f = forth();
    f.set_file_system(Box::new(MemoryFs::new())).unwrap();
    run(&mut f, "$VARIABLE S  $[]VARIABLE A");
    run(
        &mut f,
        r#"S" t.txt" W/O CREATE-FILE THROW DUP S" l1" ROT WRITE-LINE THROW DUP S" " ROT WRITE-LINE THROW DUP S" l3" ROT WRITE-FILE THROW CLOSE-FILE THROW"#,
    );
    assert_eq!(run(&mut f, r#"S" t.txt" S $SLURP-FILE S $@LEN ."#), "6 ");
    assert_eq!(
        run(&mut f, r#"S" t.txt" A $[]SLURP-FILE A $[]# . 1 A $[]@ NIP . 2 A $[]@ TYPE"#),
        "3 0 l3"
    );
    assert_eq!(
        run(&mut f, r#"S" t.txt" R/O OPEN-FILE THROW DUP S $SLURP CLOSE-FILE THROW S $@LEN ."#),
        "6 "
    );
}

#[test]
fn names_stay_in_their_word_list() {
    let mut f = Forth::new();
    f.install(&Strings);
    assert_eq!(code(&mut f, "$VARIABLE S"), -13);
    assert_eq!(run(&mut f, r#"S" FORTHLING-DYNAMIC-STRINGS" ENVIRONMENT? . ."#), "-1 1 ");
    // the system is as it was after the install
    assert_eq!(run(&mut f, "GET-ORDER NIP ."), "1 ");
    // new definitions go to FORTH, not to STRINGS
    run(&mut f, ": SQUARE DUP * ; 7 SQUARE .");
    run(&mut f, "ALSO DYNAMIC-STRINGS $VARIABLE S PREVIOUS");
}

#[test]
fn memory_comes_from_the_heap() {
    let mut f = Forth::with_memory(64 * 1024);
    f.install(&Strings);
    f.interpret("ALSO DYNAMIC-STRINGS $VARIABLE S").unwrap();
    // a string larger than memory fails with ALLOCATE's error, not a panic
    assert_eq!(code(&mut f, "1000000 S $!LEN"), -59);
    assert_eq!(run(&mut f, "S @ ."), "0 ");
    run(&mut f, "100 S $!LEN S $FREE");
}

#[test]
fn install_ignores_the_limits_of_the_program() {
    let mut f = Forth::new();
    f.set_limits(Limits { steps: Some(10), space: Some(10), ..Limits::default() });
    f.install(&Strings);
    f.set_limits(Limits::default());
    run(&mut f, "ALSO DYNAMIC-STRINGS $VARIABLE S");
}
