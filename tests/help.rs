//! The help text: every word has a line, and every example prints what it says.

#![cfg(feature = "help")]

use forthling::*;

fn all() -> Forth {
    let mut f = Forth::new();
    f.install(&Json);
    f.install(&Functional);
    f.install(&Strings);
    f.install(&Collections);
    f.install(&Fixed);
    f.install(&Canvas);
    f.install(&OperatingSystem);
    f.install(&Benchmark);
    f.install(&Testing);
    f.install(&Debugger);
    f.install(&Tutorial);
    f
}

fn run(f: &mut Forth, code: &str) -> String {
    f.interpret(code).unwrap_or_else(|e| panic!("{code}: {e}"));
    String::from_utf8_lossy(&f.take_output()).trim().to_string()
}

#[test]
fn every_word_has_a_help_line() {
    let problems = all().help_problems();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn every_example_prints_what_it_says() {
    let examples = all().help_examples();
    assert!(examples.len() > 150, "only {} examples", examples.len());
    let mut wrong = Vec::new();
    for e in examples {
        let mut f = all();
        let got = match f.interpret(&e.code) {
            Ok(()) => String::from_utf8_lossy(&f.take_output()).trim().to_string(),
            Err(err) => format!("error: {err}"),
        };
        if got != e.output {
            wrong.push(format!(
                "[{}] {}: {} printed {got:?}, not {:?}",
                e.list, e.name, e.code, e.output
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn help_shows_effect_and_description() {
    let mut f = all();
    let text = run(&mut f, "HELP SWAP");
    assert!(text.starts_with("SWAP ( x1 x2 -- x2 x1 )\n"), "{text}");
    assert!(text.contains("Exchange"), "{text}");
    assert!(text.contains(r"1 2 SWAP . .    \ prints 1 2"), "{text}");
}

#[test]
fn help_names_the_word_list_when_it_is_not_in_the_order() {
    let mut f = all();
    let text = run(&mut f, "HELP MAP");
    assert!(text.contains("ALSO FUNCTIONAL finds it."), "{text}");
    let text = run(&mut f, "ALSO FUNCTIONAL HELP MAP");
    assert!(!text.contains("finds it"), "{text}");
}

#[test]
fn help_for_a_name_that_does_not_exist_gives_close_names() {
    let mut f = all();
    let text = run(&mut f, "HELP DUPP");
    assert!(text.starts_with("DUPP is not defined."), "{text}");
    assert!(text.contains("Close:") && text.contains("DUP"), "{text}");
}

#[test]
fn help_for_a_word_of_the_program_shows_its_comment() {
    let mut f = all();
    let text = run(&mut f, ": SQ ( n -- n*n ) DUP * ; HELP SQ");
    assert!(text.starts_with("SQ ( n -- n*n )"), "{text}");
    let text = run(&mut f, ": PLAIN 1 ; HELP PLAIN");
    assert!(text.contains("no stack effect"), "{text}");
    // The comment does not change what the word does.
    assert_eq!(run(&mut f, "3 SQ . PLAIN ."), "9 1");
}

#[test]
fn the_hint_of_an_undefined_word_names_close_words() {
    let err = all().interpret("DUPP").unwrap_err();
    let hint = err.hint().unwrap_or_default();
    assert!(hint.contains("DUP") && hint.contains("HELP"), "{hint}");
}

#[test]
fn add_help_adds_the_text_of_a_host_word() {
    let mut f = Forth::new();
    f.add_help("[FORTH]\nDUP ( x -- x x ) Own text. ;; 1 DUP . . => 1 1\n");
    // The first line found wins, so a host cannot hide the system text by accident;
    // a line for a word of the host would be read the same way.
    let h = f.help("DUP");
    assert!(h.found && !h.user);
    assert!(h.description.is_some());
}
