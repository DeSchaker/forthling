//! The host API, used from outside the crate as an extension would.

use forthling::{Cell, Extension, Forth, Throw};
use std::cell::Cell as Shared;
use std::rc::Rc;

#[derive(Default)]
struct Counter(Cell);

struct Demo;

impl Extension for Demo {
    fn name(&self) -> &str {
        "FORTHLING-DEMO"
    }

    fn version(&self) -> Cell {
        3
    }

    fn install(&self, f: &mut Forth) {
        let wid = f.wordlist("DEMO");
        f.add_word(wid, "COUNT+", |f| {
            let counter = f.state::<Counter>();
            counter.0 += 1;
            let n = counter.0;
            f.push(n);
            Ok(())
        })
        .unwrap();
        f.add_word(wid, "GREETING", |f| f.push_bytes(b"hello")).unwrap();
        f.add_word(wid, "SHOUT", |f| {
            let text = f.pop_bytes()?;
            f.push_bytes(&text.to_ascii_uppercase())
        })
        .unwrap();
        f.add_word(wid, "FAIL", |_| Err(Throw::with(-24, "on purpose"))).unwrap();
        f.add_word(wid, "HALF", |f| {
            let r = f.fpop()?;
            f.fpush(r / 2.0)
        })
        .unwrap();
    }
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
fn extension_words_are_in_their_own_word_list() {
    let mut f = Forth::new();
    f.install(&Demo);
    assert_eq!(code(&mut f, "COUNT+"), -13);
    assert_eq!(run(&mut f, "ALSO DEMO ORDER"), "Search order: DEMO FORTH\nDefinitions: FORTH\n");
    assert_eq!(run(&mut f, "COUNT+ . COUNT+ . PREVIOUS"), "1 2 ");
    assert_eq!(code(&mut f, "COUNT+"), -13);
}

#[test]
fn environment_knows_installed_extensions() {
    let mut f = Forth::new();
    assert_eq!(run(&mut f, "S\" FORTHLING-DEMO\" ENVIRONMENT? ."), "0 ");
    f.install(&Demo);
    assert_eq!(run(&mut f, "S\" forthling-demo\" ENVIRONMENT? . ."), "-1 3 ");
}

#[test]
fn strings_floats_and_errors_cross_the_boundary() {
    let mut f = Forth::new();
    f.install(&Demo);
    let out = run(&mut f, "ALSO DEMO GREETING 2DUP TYPE SPACE SHOUT 2DUP TYPE DROP FREE .");
    assert_eq!(out, "hello HELLO0 ");
    assert_eq!(run(&mut f, "3E0 HALF F."), "1.5 ");
    assert_eq!(run(&mut f, "' FAIL CATCH ."), "-24 ");
    let e = f.interpret("FAIL").unwrap_err();
    assert_eq!(e.to_string(), "error -24: invalid numeric argument: on purpose");
    assert_eq!(code(&mut f, "-1 5 SHOUT"), -9);
}

#[test]
fn extension_words_belong_to_the_system() {
    let mut f = Forth::new();
    f.install(&Demo);
    assert_eq!(run(&mut f, "ALSO DEMO SEE GREETING"), "GREETING is a host word\n");
    assert_eq!(code(&mut f, "DEMO DEFINITIONS FORGET GREETING"), -15);
}

#[test]
fn a_host_program_adds_words_directly() {
    let mut f = Forth::new();
    let seen = Rc::new(Shared::new(0));
    let log = Rc::clone(&seen);
    f.add_word(1, "REPORT", move |f| {
        log.set(f.pop()?);
        Ok(())
    })
    .unwrap();
    assert_eq!(run(&mut f, ": T 6 7 * REPORT ; T DEPTH ."), "0 ");
    assert_eq!(seen.get(), 42);
    assert_eq!(f.wordlist("forth"), 1);
    let wid = f.wordlist("Tools2");
    assert_eq!(f.wordlist("TOOLS2"), wid);
    assert_eq!(f.add_word(99, "X", |_| Ok(())).unwrap_err().code, -24);
    assert_eq!(f.add_word(wid, "", |_| Ok(())).unwrap_err().code, -16);
    f.push(5);
    assert_eq!((f.depth(), f.pop().unwrap()), (1, 5));
}
