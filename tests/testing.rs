//! The TESTING words, used from Forth.

use forthling::{Forth, Testing};
use std::io::Cursor;

fn forth() -> Forth {
    let mut f = Forth::new();
    f.set_input(Box::new(Cursor::new(Vec::new())));
    f.install(&Testing);
    f.interpret("ALSO TESTING").unwrap();
    f
}

fn run(f: &mut Forth, text: &str) -> String {
    let start = f.output().len();
    f.interpret(text).unwrap_or_else(|e| panic!("{text}: {e}"));
    String::from_utf8_lossy(&f.output()[start..]).into_owned()
}

#[test]
fn the_word_list_is_closed_until_asked_for() {
    let mut f = Forth::new();
    f.install(&Testing);
    assert_eq!(f.interpret("T{ -> }T").unwrap_err().code, -13);
    f.interpret("ALSO TESTING T{ -> }T").unwrap();
    assert_eq!(run(&mut f, "S\" FORTHLING-TESTING\" ENVIRONMENT? . DROP"), "-1 ");
}

#[test]
fn passing_tests_are_counted_and_print_nothing() {
    let mut f = forth();
    let out = run(&mut f, "T{ 1 2 + -> 3 }T  T{ -> }T  T{ 1 2 -> 1 2 }T  T{ 5 DUP -> 5 5 }T");
    assert_eq!(out, "");
    let r = f.take_test_results();
    assert_eq!((r.passed, r.failed), (4, 0));
    assert_eq!(r.failures.len(), 0);
    // The results start again.
    assert_eq!(f.take_test_results().passed, 0);
}

#[test]
fn a_wrong_result_is_printed_and_the_run_goes_on() {
    let mut f = forth();
    let out = run(&mut f, "T{ 1 2 + -> 4 }T\nT{ 1 1 + -> 2 }T\nT{ 1 -> 1 2 }T");
    assert_eq!(
        out,
        "FAIL input:1: incorrect result: expected <1> 4, got <1> 3\n\
         FAIL input:3: wrong number of results: expected <2> 1 2, got <1> 1\n"
    );
    let r = f.take_test_results();
    assert_eq!((r.passed, r.failed), (1, 2));
    assert_eq!(r.failures[0].line, 1);
    assert_eq!((r.failures[0].expected.clone(), r.failures[0].actual.clone()), (vec![4], vec![3]));
    assert_eq!(r.failures[1].line, 3);
    assert_eq!(r.failures[1].source, "input");
}

#[test]
fn the_line_is_that_of_t_brace() {
    let mut f = forth();
    let out = run(&mut f, "T{ 1\n2 +\n-> 4\n}T");
    assert!(out.starts_with("FAIL input:1:"), "{out}");
}

#[test]
fn testing_names_the_group_and_the_stack_below_is_left_alone() {
    let mut f = forth();
    let out = run(&mut f, "77 88\nTESTING addition\nT{ 1 1 + -> 3 }T");
    assert_eq!(
        out,
        "FAIL input:3 (addition): incorrect result: expected <1> 3, got <1> 2\n  stack below: 77 88\n"
    );
    let r = f.take_test_results();
    assert_eq!(r.failures[0].group, "addition");
    assert_eq!(r.failures[0].stack, vec![77, 88]);
    assert_eq!(f.stack(), [77, 88]);
}

#[test]
fn numbers_are_shown_in_the_base_of_the_system() {
    let mut f = forth();
    let out = run(&mut f, "HEX T{ FF -> FE }T");
    assert_eq!(out, "FAIL input:1: incorrect result: expected <1> FE, got <1> FF\n");
}

#[test]
fn a_summary_can_be_printed() {
    let mut f = forth();
    let out = run(&mut f, "T{ 1 -> 1 }T T{ 1 -> 2 }T TEST-SUMMARY");
    assert!(out.ends_with("tests: 1 passed, 1 failed\n"), "{out}");
}

#[test]
fn an_error_inside_a_test_is_not_caught() {
    let mut f = forth();
    assert_eq!(f.interpret("T{ 1 0 / -> 0 }T").unwrap_err().code, -10);
    // The next test starts clean.
    run(&mut f, "T{ 1 -> 1 }T");
    assert_eq!(f.take_test_results().passed, 1);
    // The limits pass through.
    f.set_limits(forthling::Limits { steps: Some(1000), ..forthling::Limits::default() });
    let e = f.interpret(": LOOP-FOREVER BEGIN AGAIN ; T{ LOOP-FOREVER -> }T").unwrap_err();
    assert_eq!(e.code, forthling::LIMIT_EXCEEDED);
}

#[test]
fn misplaced_words_throw() {
    let mut f = forth();
    assert_eq!(f.interpret("->").unwrap_err().code, -22);
    assert_eq!(f.interpret("}T").unwrap_err().code, -22);
    assert_eq!(f.interpret("T{ 1 }T").unwrap_err().code, -22);
    assert_eq!(f.interpret("T{ 1 -> 1 -> }T").unwrap_err().code, -22);
}

#[test]
fn only_the_first_failures_are_kept() {
    let mut f = forth();
    run(&mut f, ": BAD T{ 1 -> 2 }T ;  : MANY 40 0 DO BAD LOOP ;");
    f.interpret("MANY").unwrap();
    let r = f.take_test_results();
    assert_eq!((r.failed, r.failures.len()), (40, 32));
}

#[test]
fn long_lists_are_cut_to_the_top_cells() {
    let mut f = forth();
    let out = run(&mut f, ": FORTY 40 0 DO I LOOP ;  T{ FORTY -> }T");
    assert!(out.contains("... (8 more) 8 9"), "{out}");
    let r = f.take_test_results();
    assert_eq!((r.failures[0].actual.len(), r.failures[0].actual_depth), (32, 40));
}
