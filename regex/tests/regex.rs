//! The REGEX words, used from Forth.

use forthling::{Cell, Forth, Limits};
use forthling_regex::{REGEX_ERROR, Regex};

fn forth() -> Forth {
    let mut f = Forth::new();
    f.install(&Regex);
    f.interpret("ALSO REGEX").unwrap();
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
fn match_anywhere_or_anchored() {
    let mut f = forth();
    assert_eq!(run(&mut f, r#"S" order 66" S" [0-9]+" RE-MATCH? ."#), "-1 ");
    assert_eq!(run(&mut f, r#"S" order" S" [0-9]+" RE-MATCH? ."#), "0 ");
    assert_eq!(run(&mut f, r#"S" a1" S" ^[a-z]+$" RE-MATCH? ."#), "0 ");
    assert_eq!(run(&mut f, r#"S" abc" S" ^[a-z]+$" RE-MATCH? ."#), "-1 ");
}

#[test]
fn find_gives_start_and_length() {
    let mut f = forth();
    assert_eq!(run(&mut f, r#"S" order 66" S" [0-9]+" RE-FIND . . ."#), "-1 2 6 ");
    assert_eq!(run(&mut f, r#"S" order" S" [0-9]+" RE-FIND ."#), "0 ");
    // The offset reaches the matching text.
    f.interpret(r#": SHOW  S" order 66!" 2DUP S" [0-9]+" RE-FIND DROP ROT DROP >R + R> TYPE ;"#)
        .unwrap();
    assert_eq!(run(&mut f, "SHOW"), "66");
}

#[test]
fn groups() {
    let mut f = forth();
    let date = r#"S" on 2026-11-01." S" (\d+)-(\d+)-(\d+)""#;
    assert_eq!(run(&mut f, &format!("{date} 2 RE-GROUP . . .")), "-1 2 8 ");
    assert_eq!(run(&mut f, &format!("{date} 0 RE-GROUP . . .")), "-1 10 3 ");
    assert_eq!(run(&mut f, r#"S" b" S" (a)?b" 1 RE-GROUP ."#), "0 ");
    assert_eq!(run(&mut f, r#"S" xyz" S" (a)" 1 RE-GROUP ."#), "0 ");
    assert_eq!(code(&mut f, r#"S" a" S" (a)" 2 RE-GROUP"#), REGEX_ERROR);
    assert_eq!(code(&mut f, r#"S" a" S" (a)" -1 RE-GROUP"#), REGEX_ERROR);
}

#[test]
fn count_matches() {
    let mut f = forth();
    assert_eq!(run(&mut f, r#"S" a1 b22 c333" S" [0-9]+" RE-COUNT ."#), "3 ");
    assert_eq!(run(&mut f, r#"S" abc" S" [0-9]+" RE-COUNT ."#), "0 ");
}

#[test]
fn replace_with_groups_and_free() {
    let mut f = forth();
    let words = [
        (r#"S" 2026-11-01" S" (\d+)-(\d+)-(\d+)" S" $3/$2/$1""#, "01/11/2026"),
        (r#"S" a b  c" S" \s+" S" _""#, "a_b_c"),
        (r#"S" cost" S" cost" S" $$5""#, "$5"),
        (r#"S" abc" S" x" S" y""#, "abc"),
    ];
    for (i, (args, want)) in words.iter().enumerate() {
        f.interpret(&format!(": R{i}  {args} RE-REPLACE 2DUP TYPE DROP FREE THROW ;")).unwrap();
        assert_eq!(run(&mut f, &format!("R{i}")), *want);
    }
}

#[test]
fn replace_stays_within_the_memory() {
    let mut f = forth();
    // A text of 60000 bytes with the empty pattern matches 60001 times, each
    // replaced by 60000 bytes.
    f.interpret("CREATE TXT 60000 ALLOT  : R  TXT 60000 S\" \" TXT 60000 RE-REPLACE ;").unwrap();
    assert_eq!(code(&mut f, "R"), REGEX_ERROR);
}

#[test]
fn bad_patterns_and_texts_throw_by_name() {
    let mut f = forth();
    assert_eq!(code(&mut f, r#"S" a" S" (" RE-MATCH?"#), REGEX_ERROR);
    assert_eq!(code(&mut f, r#"S" a" S" a{99999999}" RE-MATCH?"#), REGEX_ERROR);
    let deep = format!("S\" a\" S\" {}a{}\" RE-MATCH?", "(".repeat(200), ")".repeat(200));
    assert_eq!(code(&mut f, &deep), REGEX_ERROR);
    // Bytes that are not UTF-8.
    f.interpret("CREATE B 255 C, 254 C,").unwrap();
    assert_eq!(code(&mut f, r#"B 2 S" a" RE-MATCH?"#), REGEX_ERROR);
}

#[test]
fn a_pattern_that_backtracks_in_other_engines_stays_fast() {
    let mut f = forth();
    f.interpret(r#": AS  S" aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa!" ;"#).unwrap();
    assert_eq!(run(&mut f, r#"AS S" (a+)+$" RE-MATCH? ."#), "0 ");
}

#[test]
fn the_work_is_charged() {
    let mut f = Forth::sandboxed();
    f.install(&Regex);
    f.set_limits(Limits { steps: Some(1000), ..Limits::default() });
    f.interpret("ALSO REGEX").unwrap();
    f.interpret("CREATE T 100000 ALLOT").unwrap();
    let e = f.interpret("T 100000 S\" x\" RE-MATCH?").unwrap_err();
    assert_eq!(e.code, forthling::LIMIT_EXCEEDED, "{e}");
}
