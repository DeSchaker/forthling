//! The FUNCTIONAL words, used from Forth.

use forthling::{Cell, Forth, Functional, Limits};

fn forth() -> Forth {
    let mut f = Forth::new();
    f.install(&Functional);
    f.interpret("ALSO FUNCTIONAL").unwrap();
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
fn map_with_primitives_and_quotations() {
    let mut f = forth();
    assert_eq!(run(&mut f, "1 2 3 3 ' 1+ MAP . . ."), "4 3 2 ");
    assert_eq!(run(&mut f, "1 2 3 3 [: DUP * ;] MAP . . ."), "9 4 1 ");
    assert_eq!(run(&mut f, "1 2 3 3 [: 1+ ;] MAP . . ."), "4 3 2 ");
    assert_eq!(run(&mut f, "0 ' 1+ MAP DEPTH ."), "0 ");
    // the cells under the items stay
    assert_eq!(run(&mut f, "99 1 2 2 ' NEGATE MAP . . ."), "-2 -1 99 ");
}

#[test]
fn fold_and_scan() {
    let mut f = forth();
    assert_eq!(run(&mut f, "1 2 3 4 4 ' + FOLD ."), "10 ");
    assert_eq!(run(&mut f, "5 1 ' + FOLD ."), "5 ");
    assert_eq!(run(&mut f, "1 2 3 4 4 ' MAX FOLD ."), "4 ");
    assert_eq!(run(&mut f, "10 3 2 3 [: - ;] FOLD ."), "5 ");
    assert_eq!(run(&mut f, "1 2 3 4 4 ' + SCAN . . . ."), "10 6 3 1 ");
    assert_eq!(run(&mut f, "0 ' + SCAN DEPTH ."), "0 ");
    assert_eq!(code(&mut f, "0 ' + FOLD"), -24);
}

#[test]
fn filter_and_range() {
    let mut f = forth();
    assert_eq!(run(&mut f, "1 2 3 4 5 5 [: 1 AND ;] FILTER . . . ."), "3 5 3 1 ");
    assert_eq!(run(&mut f, "1 2 3 3 [: 9 < INVERT ;] FILTER ."), "0 ");
    assert_eq!(run(&mut f, "3 7 RANGE . . . . ."), "4 6 5 4 3 ");
    assert_eq!(run(&mut f, "5 5 RANGE ."), "0 ");
    assert_eq!(run(&mut f, "5 2 RANGE ."), "0 ");
    assert_eq!(run(&mut f, "1 6 RANGE [: DUP * ;] MAP 5 ' + FOLD ."), "55 ");
    assert_eq!(code(&mut f, "0 1000000000000 RANGE"), -3);
}

#[test]
fn the_stack_effect_is_checked() {
    let mut f = forth();
    assert_eq!(code(&mut f, "1 2 2 [: DROP ;] MAP"), -24);
    assert_eq!(code(&mut f, "1 2 2 [: DUP ;] MAP"), -24);
    assert_eq!(code(&mut f, "1 2 2 [: 2DROP ;] FOLD"), -24);
    assert_eq!(code(&mut f, "1 2 2 ' DROP FILTER"), -24);
    assert_eq!(code(&mut f, "1 2 3 ' 1+ MAP"), -24);
    assert_eq!(code(&mut f, "1 -1 ' 1+ MAP"), -24);
    assert_eq!(code(&mut f, "1 1 -5 MAP"), -24);
    assert_eq!(code(&mut f, "1 1 ' + MAP"), -4);
}

#[test]
fn errors_inside_the_token_pass_through() {
    let mut f = forth();
    assert_eq!(code(&mut f, "1 0 2 [: 0 SWAP / ;] MAP"), -10);
    assert_eq!(run(&mut f, "1 0 2 [: 0 SWAP / ;] ' MAP CATCH ."), "-10 ");
}

#[test]
fn the_step_limit_applies_inside_the_token() {
    let mut f = Forth::new();
    f.install(&Functional);
    f.interpret("ALSO FUNCTIONAL").unwrap();
    f.set_limits(Limits { steps: Some(5000), ..Limits::default() });
    assert_eq!(code(&mut f, "0 20000 RANGE ' 1+ MAP"), forthling::LIMIT_EXCEEDED);
}

#[test]
fn environment_and_word_list() {
    let mut f = Forth::new();
    f.install(&Functional);
    assert_eq!(run(&mut f, r#"S" FORTHLING-FUNCTIONAL" ENVIRONMENT? . ."#), "-1 1 ");
    assert!(f.interpret("1 2 2 ' + FOLD").is_err()); // not in the search order
}
