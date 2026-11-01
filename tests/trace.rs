//! Tracing, through the public API: TRACE-ON in the word list DEBUG, and
//! recording a trace as data with `record_trace`.

use forthling::{Debugger, Forth, LIMIT_EXCEEDED, Limits, TRACE_CELLS, TraceEntry};

fn forth() -> Forth {
    let mut f = Forth::new();
    f.install(&Debugger);
    f
}

fn run(f: &mut Forth, text: &str) -> String {
    f.interpret(text).unwrap();
    String::from_utf8(f.take_output()).unwrap()
}

fn entry(word: &str, depth: usize, stack: &[i64]) -> TraceEntry {
    TraceEntry { word: word.into(), depth, stack: stack.to_vec(), stack_depth: stack.len() }
}

#[test]
fn trace_on_prints_a_line_per_word() {
    let mut f = forth();
    run(&mut f, ": SQ DUP * ; : F 3 SQ 1+ ; ALSO DEBUG");
    let out = run(&mut f, "TRACE-ON F . TRACE-OFF 1 .");
    assert_eq!(out, "F <0>\n  SQ <1> 3\n    DUP <1> 3\n    * <2> 3 3\n  1+ <1> 9\n. <1> 10\n10 1 ");
}

#[test]
fn the_insides_of_system_words_and_compiling_are_not_traced() {
    let mut f = forth();
    run(&mut f, "ALSO DEBUG");
    // `/` is defined in Forth by the system; only the word itself shows.
    assert_eq!(run(&mut f, "TRACE-ON 7 2 / TRACE-OFF"), "/ <2> 7 2\n");
    // IF and ; run while T is compiled; : runs before.
    assert_eq!(run(&mut f, "TRACE-ON : T 1 IF 2 THEN ; TRACE-OFF"), ": <1> 3\n");
}

#[test]
fn a_trace_line_shows_the_top_of_a_deep_stack() {
    let mut f = forth();
    run(&mut f, "ALSO DEBUG : T ;");
    let out = run(&mut f, "1 2 3 4 5 6 7 8 9 10 TRACE-ON T TRACE-OFF");
    assert_eq!(out, "T <10> ... 3 4 5 6 7 8 9 10\n");
    // numbers in BASE, as .S shows them
    let mut f = forth();
    run(&mut f, "ALSO DEBUG : T ;");
    assert_eq!(run(&mut f, "255 HEX TRACE-ON T TRACE-OFF DECIMAL"), "T <1> FF\n");
}

#[test]
fn deep_nesting_is_indented_up_to_a_limit() {
    let mut f = forth();
    run(&mut f, "ALSO DEBUG : R ( n -- ) ?DUP IF 1- RECURSE THEN ;");
    let out = run(&mut f, "TRACE-ON 30 R TRACE-OFF");
    let deepest = out.lines().map(|l| l.len() - l.trim_start().len()).max().unwrap();
    assert_eq!(deepest, 32);
}

#[test]
fn a_host_records_the_trace_as_data() {
    let mut f = forth();
    run(&mut f, ": SQ DUP * ; : F 3 SQ 1+ ;");
    f.record_trace(Some(3));
    assert_eq!(run(&mut f, "F ."), "10 "); // recording prints nothing
    assert_eq!(f.trace_dropped(), 3); // *, 1+ and .
    let trace = f.take_trace();
    assert_eq!(trace, [entry("F", 0, &[]), entry("SQ", 1, &[3]), entry("DUP", 2, &[3])]);
    assert_eq!(f.trace_dropped(), 0);
    f.record_trace(None);
    run(&mut f, "F DROP");
    assert_eq!(f.take_trace(), [] as [forthling::TraceEntry; 0]);
}

#[test]
fn a_recorded_entry_keeps_the_top_of_the_stack() {
    let mut f = forth();
    run(&mut f, ": T ;");
    f.record_trace(Some(10));
    run(&mut f, "1 2 3 4 5 6 7 8 9 10 T");
    let trace = f.take_trace();
    assert_eq!(trace[0].stack, (3..=10).collect::<Vec<i64>>());
    assert_eq!(trace[0].stack.len(), TRACE_CELLS);
    assert_eq!(trace[0].stack_depth, 10);
}

#[test]
fn the_trace_of_a_failing_call_is_kept() {
    let mut f = forth();
    run(&mut f, ": D 0 / ; : M 1 D ;");
    f.record_trace(Some(100));
    assert!(f.interpret("M").is_err());
    let words: Vec<_> = f.take_trace().into_iter().map(|e| e.word).collect();
    assert_eq!(words, ["M", "D", "/"]);
}

#[test]
fn trace_lines_count_towards_the_output_limit() {
    let mut f = forth();
    f.set_limits(Limits { output: Some(100), ..Limits::default() });
    run(&mut f, "ALSO DEBUG");
    let e = f.interpret("TRACE-ON : T 100 0 DO I DROP LOOP ; T").unwrap_err();
    assert_eq!(e.code, LIMIT_EXCEEDED);
    assert_eq!(f.take_output().len(), 100);
}

#[test]
fn a_plain_system_has_no_debug_words() {
    let mut f = Forth::new();
    assert!(f.interpret("ALSO DEBUG").is_err());
    f.interpret("S\" FORTHLING-DEBUG\" ENVIRONMENT? 0= .").unwrap();
    assert_eq!(f.take_output(), b"-1 ");
}
