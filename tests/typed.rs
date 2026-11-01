//! The macro `forth_word!`: typed words, tested from outside like a host.

use forthling::{Cell, Forth, Res, Throw, forth_word};
use std::cell::Cell as Shared;
use std::rc::Rc;

fn run(f: &mut Forth, text: &str) -> String {
    f.interpret(text).unwrap();
    String::from_utf8(f.take_output()).unwrap()
}

#[test]
fn arguments_come_in_stack_order() {
    let mut f = Forth::new();
    forth_word!(f, 1, "SUB", (a: i64, b: i64) -> i64 { a - b }).unwrap();
    // cells and floats each in their own order
    forth_word!(f, 1, "MIX", (n: i64, r: f64, m: i64) -> f64 { n as f64 * 100.0 + r * 10.0 + m as f64 })
        .unwrap();
    assert_eq!(run(&mut f, "10 3 SUB . 1 2E0 3 MIX F."), "7 123. ");
}

#[test]
fn results_go_back_in_order() {
    let mut f = Forth::new();
    forth_word!(f, 1, "SPLIT", (n: i64) -> (i64, i64) { (n / 10, n % 10) }).unwrap();
    forth_word!(f, 1, "BOTH", () -> (i64, f64, bool) { (1, 2.5, true) }).unwrap();
    f.interpret("42 SPLIT BOTH").unwrap();
    assert_eq!(f.stack(), [4, 2, 1, -1]);
    assert_eq!(f.fstack(), [2.5]);
}

#[test]
fn flags_and_strings() {
    let mut f = Forth::new();
    forth_word!(f, 1, "NOT", (b: bool) -> bool { !b }).unwrap();
    forth_word!(f, 1, "REV", (s: String) -> String { s.chars().rev().collect() }).unwrap();
    forth_word!(f, 1, "LEN", (b: Vec<u8>) -> Cell { b.len() as Cell }).unwrap();
    forth_word!(f, 1, "GREET", () -> &'static str { "hello" }).unwrap();
    assert_eq!(run(&mut f, "0 NOT . 7 NOT ."), "-1 0 ");
    // a string result is ALLOCATEd; the program frees it
    assert_eq!(run(&mut f, "S\" abc\" REV 2DUP TYPE DROP FREE ."), "cba0 ");
    assert_eq!(run(&mut f, "S\" h\u{e9}\" LEN . GREET TYPE"), "3 hello");
}

#[test]
fn a_word_without_a_result_and_with_state() {
    let mut f = Forth::new();
    let count = Rc::new(Shared::new(0));
    let seen = Rc::clone(&count);
    forth_word!(f, 1, "TICK", (n: i64) { seen.set(seen.get() + n) }).unwrap();
    f.interpret("1 TICK 2 TICK : T 3 TICK ; T").unwrap();
    assert_eq!(count.get(), 6);
    assert_eq!(f.stack(), []);
}

#[test]
fn a_result_of_res_can_throw() {
    let mut f = Forth::new();
    forth_word!(f, 1, "SAFE/", (a: i64, b: i64) -> Res<i64> {
        if b == 0 {
            return Err(Throw::new(-10));
        }
        Ok(a / b)
    })
    .unwrap();
    assert_eq!(run(&mut f, "7 2 SAFE/ . 7 0 ' SAFE/ CATCH ."), "3 -10 ");
    assert_eq!(f.interpret("1 0 SAFE/").unwrap_err().code, -10);
}

#[test]
fn too_few_arguments_take_nothing() {
    let mut f = Forth::new();
    forth_word!(f, 1, "ADD3", (a: i64, b: i64, c: i64) -> i64 { a + b + c }).unwrap();
    forth_word!(f, 1, "SCALE", (n: i64, r: f64) -> f64 { n as f64 * r }).unwrap();
    let e = f.interpret("1 2 ADD3").unwrap_err();
    assert_eq!(e.code, -4);
    assert_eq!(e.context().unwrap().stack, [1, 2]);
    let e = f.interpret("5 SCALE").unwrap_err();
    assert_eq!(e.code, -45);
    assert_eq!(e.context().unwrap().stack, [5]);
}

#[test]
fn words_go_into_the_word_list_given() {
    let mut f = Forth::new();
    let wid = f.wordlist("UTIL");
    forth_word!(f, wid, "TWICE", (n: i64) -> i64 { 2 * n }).unwrap();
    assert_eq!(f.interpret("3 TWICE").unwrap_err().code, -13);
    assert_eq!(run(&mut f, "ALSO UTIL 3 TWICE ."), "6 ");
    // an empty name is refused, as by add_word
    assert_eq!(forth_word!(f, wid, "", () {}).unwrap_err().code, -16);
}
