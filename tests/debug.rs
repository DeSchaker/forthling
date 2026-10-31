//! The debugger: BREAK and a host function that says how to go on.

use forthling::{Debugger, Forth, Limits, Pause, Reason, Resume, Throw};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

const PROGRAM: &str = ": SQ DUP * ; : F 3 SQ 1+ ; : MAIN BREAK F 2 SQ + ;";

/// A system with PROGRAM and a debugger that answers each pause with the
/// next of `answers` (then Continue), and keeps the pauses.
fn debugged(answers: &[Resume]) -> (Forth, Rc<RefCell<Vec<Pause>>>) {
    let mut f = Forth::new();
    f.install(&Debugger);
    f.interpret(&format!("ALSO DEBUG {PROGRAM}")).unwrap();
    let pauses = Rc::new(RefCell::new(Vec::new()));
    let seen = Rc::clone(&pauses);
    // The answers last first, so that pop gives the next.
    let mut answers: Vec<Resume> = answers.iter().rev().copied().collect();
    f.set_debugger(Some(Box::new(move |_, pause| {
        seen.borrow_mut().push(pause.clone());
        Ok(answers.pop().unwrap_or(Resume::Continue))
    })));
    (f, pauses)
}

fn words(pauses: &RefCell<Vec<Pause>>) -> Vec<(String, usize)> {
    pauses.borrow().iter().map(|p| (p.word.clone(), p.depth)).collect()
}

fn list(words: &[(&str, usize)]) -> Vec<(String, usize)> {
    words.iter().map(|&(w, d)| (w.to_string(), d)).collect()
}

#[test]
fn break_hands_the_state_to_the_host() {
    let (mut f, pauses) = debugged(&[]);
    f.interpret("7 MAIN").unwrap();
    assert_eq!(f.stack(), [7, 14]);
    let pauses = pauses.borrow();
    assert_eq!(pauses.len(), 1);
    let p = &pauses[0];
    assert_eq!((p.reason, p.word.as_str(), p.depth), (Reason::Break, "BREAK", 1));
    assert_eq!(p.context.stack, [7]);
    assert_eq!(p.context.words, ["MAIN", "BREAK"]);
    assert_eq!((p.context.line, p.context.column, p.context.token.as_str()), (1, 3, "MAIN"));
}

#[test]
fn step_stops_at_every_word() {
    let steps = [Resume::Step; 20];
    let (mut f, pauses) = debugged(&steps);
    f.interpret("MAIN").unwrap();
    assert_eq!(f.stack(), [14]);
    let expected = [
        ("BREAK", 1),
        ("F", 1),
        ("SQ", 2),
        ("DUP", 3),
        ("*", 3),
        ("1+", 2),
        ("SQ", 1),
        ("DUP", 2),
        ("*", 2),
        ("+", 1),
    ];
    assert_eq!(words(&pauses), list(&expected));
    assert!(pauses.borrow().iter().skip(1).all(|p| p.reason == Reason::Step));
    // the stack before each word
    assert_eq!(pauses.borrow()[4].context.stack, [3, 3]);
}

#[test]
fn over_and_out_skip_what_is_inside() {
    let (mut f, pauses) = debugged(&[Resume::Over, Resume::Over, Resume::Over]);
    f.interpret("MAIN").unwrap();
    assert_eq!(words(&pauses), list(&[("BREAK", 1), ("F", 1), ("SQ", 1), ("+", 1)]));
    // into F, into SQ, then out: on once F has finished
    let (mut f, pauses) = debugged(&[Resume::Step, Resume::Step, Resume::Out]);
    f.interpret("MAIN").unwrap();
    assert_eq!(words(&pauses), list(&[("BREAK", 1), ("F", 1), ("SQ", 2), ("SQ", 1)]));
}

#[test]
fn stepping_goes_on_to_the_next_word_of_the_input() {
    let (mut f, pauses) = debugged(&[Resume::Out, Resume::Step]);
    f.interpret("MAIN 1 DROP").unwrap();
    assert_eq!(words(&pauses), list(&[("BREAK", 1), ("DROP", 0)]));
    // and ends with the evaluation
    f.interpret("5 DROP").unwrap();
    assert_eq!(pauses.borrow().len(), 2);
}

#[test]
fn an_error_from_the_host_is_thrown() {
    let mut f = Forth::new();
    f.install(&Debugger);
    f.interpret("ALSO DEBUG : T BREAK 1 ;").unwrap();
    f.set_debugger(Some(Box::new(|_, _| Err(Throw::new(-28)))));
    assert_eq!(f.interpret("T").unwrap_err().code, -28);
    f.interpret(": U ['] T CATCH ; U").unwrap();
    assert_eq!(f.stack(), [-28]);
}

#[test]
fn without_a_debugger_break_does_nothing() {
    let mut f = Forth::new();
    f.install(&Debugger);
    f.interpret(&format!("ALSO DEBUG {PROGRAM} MAIN")).unwrap();
    assert_eq!(f.stack(), [14]);
    let (mut f, pauses) = debugged(&[]);
    f.set_debugger(None);
    f.interpret("MAIN").unwrap();
    assert!(pauses.borrow().is_empty());
}

#[test]
fn a_pause_does_not_count_towards_the_time_limit() {
    let (mut f, _) = debugged(&[]);
    f.set_limits(Limits { time: Some(Duration::from_millis(200)), ..Limits::default() });
    f.set_debugger(Some(Box::new(|_, _| {
        std::thread::sleep(Duration::from_millis(400));
        Ok(Resume::Continue)
    })));
    f.interpret("MAIN").unwrap();
}
