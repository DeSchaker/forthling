//! The BENCHMARK words, used from Forth.

use forthling::{Benchmark, Cell, Forth, Limits};
use std::io::Cursor;

fn forth() -> Forth {
    let mut f = Forth::new();
    f.set_input(Box::new(Cursor::new(Vec::new())));
    f.install(&Benchmark);
    f.interpret("ALSO BENCHMARK").unwrap();
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
fn the_word_list_is_closed_until_asked_for() {
    let mut f = Forth::new();
    f.install(&Benchmark);
    assert_eq!(code(&mut f, ": NOOP ; ' NOOP BENCH"), -13);
    f.interpret("ALSO BENCHMARK ' NOOP BENCH DROP").unwrap();
    assert_eq!(run(&mut f, "S\" FORTHLING-BENCHMARK\" ENVIRONMENT? . DROP"), "-1 ");
}

#[test]
fn bench_runs_the_word_once_and_gives_its_time() {
    let mut f = forth();
    run(&mut f, "VARIABLE N  : COUNT-IT 1 N +! ;  : SLEEP 20 MS ;");
    assert_eq!(run(&mut f, "' COUNT-IT BENCH DROP N @ ."), "1 ");
    // 20 ms of sleep are at least 20000 microseconds.
    assert_eq!(run(&mut f, "' SLEEP BENCH 20000 >= ."), "-1 ");
    assert_eq!(run(&mut f, "DEPTH ."), "0 ");
}

#[test]
fn bench_best_runs_the_word_n_times_and_gives_the_shortest() {
    let mut f = forth();
    run(&mut f, "VARIABLE N  : COUNT-IT 1 N +! ;  : SLEEP 20 MS ;");
    assert_eq!(run(&mut f, "7 ' COUNT-IT BENCH-BEST DROP N @ ."), "7 ");
    assert_eq!(run(&mut f, "3 ' SLEEP BENCH-BEST 20000 >= ."), "-1 ");
    assert_eq!(run(&mut f, "DEPTH ."), "0 ");
}

#[test]
fn bad_arguments_throw() {
    let mut f = forth();
    run(&mut f, ": NOOP ;");
    assert_eq!(code(&mut f, "0 ' NOOP BENCH-BEST"), -24);
    assert_eq!(code(&mut f, "-3 ' NOOP BENCH-BEST"), -24);
    assert_eq!(code(&mut f, "99999999 BENCH"), -24);
    assert_eq!(code(&mut f, "-1 BENCH"), -24);
    assert_eq!(code(&mut f, "1 99999999 BENCH-BEST"), -24);
    assert_eq!(code(&mut f, "BENCH"), -4);
}

#[test]
fn an_error_in_the_word_comes_through() {
    let mut f = forth();
    run(&mut f, ": BAD 1 0 / ;");
    assert_eq!(code(&mut f, "' BAD BENCH"), -10);
    assert_eq!(code(&mut f, "5 ' BAD BENCH-BEST"), -10);
}

#[test]
fn the_limits_hold_inside_bench() {
    let mut f = forth();
    f.set_limits(Limits { steps: Some(10_000), ..Limits::default() });
    run(&mut f, ": SPIN 1000000 0 DO LOOP ;");
    assert_eq!(code(&mut f, "' SPIN BENCH"), forthling::LIMIT_EXCEEDED);
}

#[test]
fn the_definitions_file_runs_with_the_default_memory() {
    let mut f = Forth::new();
    f.set_input(Box::new(Cursor::new(Vec::new())));
    f.install(&Benchmark);
    f.install(&forthling::Strings);
    let src = std::fs::read_to_string("benches/definitions.fs").unwrap();
    f.interpret(&src).unwrap();
    let out = String::from_utf8_lossy(f.output()).into_owned();
    for case in ["define-5000", "constants-5000", "lookup-numbers", "lookup-names"] {
        assert!(out.contains(case), "{case} is missing in {out}");
    }
}

#[test]
fn a_host_clock_in_microseconds_is_read_at_each_use() {
    use std::cell::Cell as Counter;
    use std::rc::Rc;
    let ticks = Rc::new(Counter::new(5_000_000_u64));
    let mut f = Forth::new();
    f.set_input(Box::new(Cursor::new(Vec::new())));
    f.install(&Benchmark);
    let clock = Rc::clone(&ticks);
    // Each reading moves the clock on by 250 microseconds.
    f.set_clock_micros(move || {
        clock.set(clock.get() + 250);
        clock.get()
    });
    f.interpret("ALSO BENCHMARK : NOOP ;").unwrap();
    // BENCH reads the clock twice around the run: one step of 250.
    assert_eq!(run(&mut f, "' NOOP BENCH ."), "250 ");
    assert_eq!(run(&mut f, "3 ' NOOP BENCH-BEST ."), "250 ");
    // TIME&DATE reads whole seconds of the same clock.
    f.interpret("TIME&DATE 2DROP 2DROP 2DROP").unwrap();
}
