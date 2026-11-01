//! The tutorial (src/tutorial.rs and src/tutorial.txt).

use forthling::{Debugger, Forth, Json, Tutorial};
use std::io;

/// A system as the program, MCP and the playground have it.
fn system() -> Forth {
    let mut f = Forth::sandboxed();
    f.set_input(Box::new(io::Cursor::new(Vec::new())));
    f.install(&Json);
    f.install(&Debugger);
    f.install(&Tutorial);
    f
}

fn run(f: &mut Forth, code: &str) -> String {
    f.interpret(code).unwrap_or_else(|e| panic!("{code}: {e}"));
    String::from_utf8(f.take_output()).unwrap()
}

#[test]
fn there_are_ten_lessons_for_each_level() {
    let lessons = Tutorial::lessons();
    assert_eq!(lessons.len(), 20);
    for (i, l) in lessons.iter().enumerate() {
        assert_eq!(l.number, i + 1);
        assert_eq!(l.level, if i < 10 { "beginner" } else { "advanced" }, "{}", l.title);
        for part in [&l.title, &l.text, &l.example, &l.task, &l.hint, &l.answer] {
            assert!(!part.is_empty(), "lesson {} has an empty part", l.number);
        }
        // Lines fit a terminal of 80 columns, with the indent of Try.
        for line in l.text.lines().chain(l.task.lines()).chain(l.example.lines()) {
            assert!(line.chars().count() <= 76, "lesson {}: {line}", l.number);
        }
    }
}

#[test]
fn every_example_runs() {
    for l in Tutorial::lessons() {
        let mut f = system();
        f.interpret(&l.example)
            .unwrap_or_else(|e| panic!("lesson {}: {e}\n{}", l.number, l.example));
    }
}

#[test]
fn every_model_answer_passes() {
    for l in Tutorial::lessons() {
        let mut f = system();
        run(&mut f, &format!("ALSO TUTORIAL {} LESSON", l.number));
        run(&mut f, &l.answer);
        let out = run(&mut f, "CHECK");
        assert!(out.starts_with("Well done!"), "lesson {}: {out}", l.number);
    }
}

#[test]
fn a_lesson_shows_its_parts() {
    let mut f = system();
    let out = run(&mut f, "ALSO TUTORIAL 4 LESSON");
    assert!(out.starts_with("\nLesson 4 of 20, beginner: Defining words\n\n"), "{out}");
    assert!(out.contains("Try:\n  : SQUARE ( n -- n*n ) DUP * ;\n"), "{out}");
    assert!(out.contains("Task:\nDefine CUBE ( n -- n*n*n )."), "{out}");
    assert!(out.ends_with("CHECK checks your work, HINT gives a hint, NEXT goes on.\n"));
    assert!(run(&mut f, "HINT").starts_with("Hint: DUP twice"));
    assert!(run(&mut f, "NEXT").contains("Lesson 5 of 20, beginner: Printing"));
    let e = f.interpret("21 LESSON").unwrap_err();
    assert_eq!(e.to_string(), "error -24: invalid numeric argument: the lessons are 1 to 20");
}

#[test]
fn check_says_what_is_wrong_and_leaves_the_learner_alone() {
    let mut f = system();
    assert!(run(&mut f, "ALSO TUTORIAL CHECK").starts_with("Pick a lesson first"));
    run(&mut f, "4 LESSON");
    let out = run(&mut f, "1 2 HEX CHECK");
    assert_eq!(
        out,
        "Not yet: `2 CUBE` fails with error -13: undefined word.\nTry again, or type HINT for a hint.\n"
    );
    // The stack, BASE and the output are as they were.
    assert_eq!(f.stack(), [1, 2]);
    assert_eq!(
        run(&mut f, "DECIMAL 2DROP : CUBE DUP * ; CHECK"),
        "Not yet: `2 CUBE` should leave 8, but it leaves 4.\nTry again, or type HINT for a hint.\n"
    );
    let out = run(&mut f, ": CUBE DUP DUP * * ; CHECK");
    assert_eq!(out, "Well done! Lesson 4 is done. NEXT goes on to: Printing.\n");
    // A test that prints, and one of the stack.
    run(&mut f, "5 LESSON : GREET .\" Hello\" ;");
    let out = run(&mut f, "CHECK");
    assert!(
        out.starts_with(
            "Not yet: `GREET` should print \"Hello, Forth!\", but it prints \"Hello\"."
        ),
        "{out}"
    );
    run(&mut f, "1 LESSON 6 7");
    assert!(run(&mut f, "CHECK").starts_with("Not yet: the stack should be 42, but it is 6 7."));
    // LESSONS marks the lessons done.
    let list = run(&mut f, "LESSONS");
    assert!(list.starts_with("For beginners:\n   1  The stack"), "{list}");
    assert!(list.contains("   4  Defining words               done\n"), "{list}");
    assert!(list.contains("Advanced:\n  11  Recursion"), "{list}");
    assert_eq!(f.tutorial_progress(), (1, vec![4]));
}

#[test]
fn an_endless_word_stops_check() {
    let mut f = system();
    run(&mut f, "ALSO TUTORIAL 4 LESSON : CUBE BEGIN AGAIN ;");
    assert_eq!(f.interpret("CHECK").unwrap_err().code, -258);
}

#[test]
fn a_host_runs_the_words_without_the_search_order() {
    let mut f = system();
    f.push(2);
    f.tutorial("lesson").unwrap();
    assert!(String::from_utf8(f.take_output()).unwrap().contains("Lesson 2 of 20"));
    f.tutorial("CHECK").unwrap();
    assert!(String::from_utf8(f.take_output()).unwrap().starts_with("Not yet"));
    assert_eq!(f.tutorial("WORDS").unwrap_err().code, -13);
    // Forth::new() has no tutorial: a standard program sees no extra words.
    assert_eq!(Forth::new().interpret("ALSO TUTORIAL").unwrap_err().code, -13);
}
