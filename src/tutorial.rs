//! The tutorial: twenty short lessons in Forth, ten for beginners and ten
//! for those who know the basics, inside forthling itself. The extension
//! `Tutorial` (word list TUTORIAL) has LESSONS, LESSON, CHECK, HINT and
//! NEXT; the lessons are in tutorial.txt. CHECK runs the lesson's tests on
//! the learner's words, each from an empty stack, and puts the learner's
//! stacks and output back afterwards.

use crate::*;
use std::sync::OnceLock;

/// One lesson of the tutorial.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Lesson {
    /// From 1.
    pub number: usize,
    pub title: String,
    /// `beginner` or `advanced`.
    pub level: String,
    /// What the lesson explains.
    pub text: String,
    /// Code to try, a line at a time.
    pub example: String,
    /// What the learner is asked to do.
    pub task: String,
    pub hint: String,
    /// A model answer, which passes CHECK.
    pub answer: String,
    checks: Vec<Test>,
}

/// One test of CHECK.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Test {
    /// The learner's data stack holds these cells.
    Stack(Vec<Cell>),
    /// Code that, from an empty stack, leaves these cells.
    Run(String, Vec<Cell>),
    /// Code that prints this text (trailing spaces aside).
    Print(String, String),
}

impl Tutorial {
    /// The lessons, in order.
    #[must_use]
    pub fn lessons() -> &'static [Lesson] {
        lessons()
    }
}

/// The lessons, read from tutorial.txt once.
fn lessons() -> &'static [Lesson] {
    static LESSONS: OnceLock<Vec<Lesson>> = OnceLock::new();
    LESSONS.get_or_init(|| parse(include_str!("tutorial.txt")))
}

/// Reads the lessons: each starts with `@@ lesson Title`, and `@@ level`,
/// `@@ try`, `@@ task`, `@@ check`, `@@ hint` and `@@ answer` start the
/// parts after the text.
fn parse(source: &str) -> Vec<Lesson> {
    let mut lessons: Vec<Lesson> = Vec::new();
    let mut part = "";
    for line in source.lines() {
        if let Some(title) = line.strip_prefix("@@ lesson ") {
            lessons.push(Lesson {
                number: lessons.len() + 1,
                title: title.to_string(),
                level: String::new(),
                text: String::new(),
                example: String::new(),
                task: String::new(),
                hint: String::new(),
                answer: String::new(),
                checks: Vec::new(),
            });
            part = "text";
            continue;
        }
        let lesson = lessons.last_mut().expect("tutorial.txt starts with @@ lesson");
        if let Some(level) = line.strip_prefix("@@ level ") {
            lesson.level = level.to_string();
        } else if let Some(name) = line.strip_prefix("@@ ") {
            part = name;
        } else if part == "check" {
            if !line.trim().is_empty() {
                lesson.checks.push(test(line));
            }
        } else {
            let text = match part {
                "text" => &mut lesson.text,
                "try" => &mut lesson.example,
                "task" => &mut lesson.task,
                "hint" => &mut lesson.hint,
                "answer" => &mut lesson.answer,
                _ => panic!("unknown part @@ {part} in tutorial.txt"),
            };
            text.push_str(line);
            text.push('\n');
        }
    }
    for lesson in &mut lessons {
        for text in [
            &mut lesson.text,
            &mut lesson.example,
            &mut lesson.task,
            &mut lesson.hint,
            &mut lesson.answer,
        ] {
            *text = text.trim_end().to_string();
        }
    }
    lessons
}

/// A line of `@@ check`: `stack 1 2`, `run CODE => 1 2` or `print CODE => text`.
fn test(line: &str) -> Test {
    let cells = |text: &str| -> Vec<Cell> {
        text.split_whitespace().map(|n| n.parse().expect("a number in @@ check")).collect()
    };
    let split = |rest: &str| -> (String, String) {
        let (code, expect) = rest.split_once(" =>").expect("=> in @@ check");
        (code.trim().to_string(), expect.trim().to_string())
    };
    if let Some(rest) = line.strip_prefix("stack") {
        Test::Stack(cells(rest))
    } else if let Some(rest) = line.strip_prefix("run ") {
        let (code, expect) = split(rest);
        Test::Run(code, cells(&expect))
    } else if let Some(rest) = line.strip_prefix("print ") {
        let (code, expect) = split(rest);
        Test::Print(code, expect)
    } else {
        panic!("unknown check in tutorial.txt: {line}");
    }
}

/// Where the learner is.
#[derive(Default)]
struct Progress {
    current: usize,     // the lesson shown last, from 1; 0 for none yet
    passed: Vec<usize>, // the lessons whose CHECK passed, sorted
    evaluate: usize,    // the xt of the system's EVALUATE, for the tests
}

/// The extension with the word list TUTORIAL: `LESSONS`, `LESSON`, `CHECK`,
/// `HINT` and `NEXT`. ENVIRONMENT? knows it as FORTHLING-TUTORIAL.
pub struct Tutorial;

/// The words of the tutorial, by name.
type Command = fn(&mut Forth) -> Res;

const WORDS: [(&str, Command); 5] =
    [("LESSONS", list), ("LESSON", lesson), ("CHECK", check), ("HINT", hint), ("NEXT", next)];

impl Extension for Tutorial {
    fn name(&self) -> &str {
        "FORTHLING-TUTORIAL"
    }

    fn install(&self, f: &mut Forth) {
        f.state::<Progress>().evaluate =
            f.find_in("EVALUATE", FORTH_WID).expect("the system has EVALUATE");
        let wid = f.wordlist("TUTORIAL");
        for (name, word) in WORDS {
            f.add_word(wid, name, word).expect("TUTORIAL is a word list");
        }
    }
}

impl Forth {
    /// Runs a word of the tutorial (`LESSONS`, `LESSON`, which takes its
    /// number from the stack, `CHECK`, `HINT` or `NEXT`) as the program
    /// would, also when TUTORIAL is not in the search order; for a page or
    /// a tool with buttons. The tutorial must be installed.
    pub fn tutorial(&mut self, word: &str) -> Res {
        let found = WORDS.iter().find(|(name, _)| name.eq_ignore_ascii_case(word));
        let Some(&(_, run)) = found else {
            return Err(Throw::with(UNDEFINED_WORD, word));
        };
        self.evaluation(run)
    }

    /// The lesson shown last (0 for none yet) and the lessons that passed
    /// CHECK, in order.
    pub fn tutorial_progress(&mut self) -> (usize, Vec<usize>) {
        let p = self.state::<Progress>();
        (p.current, p.passed.clone())
    }
}

/// LESSONS ( -- ): the list, with a mark at the lessons done.
fn list(f: &mut Forth) -> Res {
    let passed = f.state::<Progress>().passed.clone();
    let mut text = String::new();
    let mut level = "";
    for l in lessons() {
        if l.level != level {
            level = &l.level;
            let heading = if level == "beginner" { "For beginners" } else { "Advanced" };
            text += &format!("{heading}:\n");
        }
        let done = if passed.contains(&l.number) { "done" } else { "" };
        text += &format!("{:4}  {:<28} {done}\n", l.number, l.title);
    }
    text += "n LESSON shows lesson n.\n";
    f.emit_str(&text);
    Ok(())
}

/// LESSON ( n -- ): shows lesson n.
fn lesson(f: &mut Forth) -> Res {
    let n = f.pop()?;
    let count = lessons().len();
    match usize::try_from(n) {
        Ok(n) if (1..=count).contains(&n) => {
            show(f, n);
            Ok(())
        }
        _ => Err(Throw::with(INVALID_ARGUMENT, format!("the lessons are 1 to {count}"))),
    }
}

fn show(f: &mut Forth, n: usize) {
    f.state::<Progress>().current = n;
    let l = &lessons()[n - 1];
    let example: String = l.example.lines().flat_map(|line| ["  ", line, "\n"]).collect();
    let text = format!(
        "\nLesson {n} of {}, {}: {}\n\n{}\n\nTry:\n{example}\nTask:\n{}\n\n\
         CHECK checks your work, HINT gives a hint, NEXT goes on.\n",
        lessons().len(),
        l.level,
        l.title,
        l.text,
        l.task,
    );
    f.emit_str(&text);
}

/// The current lesson, or a word on how to pick one.
fn current(f: &mut Forth) -> Option<&'static Lesson> {
    match f.state::<Progress>().current {
        0 => {
            f.emit_str("Pick a lesson first: 1 LESSON starts, LESSONS lists them all.\n");
            None
        }
        n => Some(&lessons()[n - 1]),
    }
}

/// HINT ( -- ): a hint for the current lesson.
fn hint(f: &mut Forth) -> Res {
    if let Some(l) = current(f) {
        f.emit_str(&format!("Hint: {}\n", l.hint));
    }
    Ok(())
}

/// NEXT ( -- ): the lesson after the current one.
fn next(f: &mut Forth) -> Res {
    let n = f.state::<Progress>().current + 1;
    if n > lessons().len() {
        f.emit_str("That was the last lesson. LESSONS lists them all.\n");
    } else {
        show(f, n);
    }
    Ok(())
}

/// CHECK ( -- ): runs the tests of the current lesson.
fn check(f: &mut Forth) -> Res {
    let Some(l) = current(f) else { return Ok(()) };
    for t in &l.checks {
        if let Some(wrong) = trial(f, t)? {
            f.emit_str(&format!("Not yet: {wrong}.\nTry again, or type HINT for a hint.\n"));
            return Ok(());
        }
    }
    let passed = &mut f.state::<Progress>().passed;
    if let Err(at) = passed.binary_search(&l.number) {
        passed.insert(at, l.number);
    }
    let text = match lessons().get(l.number) {
        Some(next) => {
            format!("Well done! Lesson {} is done. NEXT goes on to: {}.\n", l.number, next.title)
        }
        None => "Well done! That was the last lesson of the tutorial.\n".to_string(),
    };
    f.emit_str(&text);
    Ok(())
}

/// Runs one test; what was wrong, or None when it passed.
fn trial(f: &mut Forth, test: &Test) -> Res<Option<String>> {
    let (code, outcome) = match test {
        Test::Stack(expected) => {
            if f.stack == *expected {
                return Ok(None);
            }
            return Ok(Some(format!(
                "the stack should be {}, but it is {}",
                cells(expected),
                cells(&f.stack)
            )));
        }
        Test::Run(code, _) | Test::Print(code, _) => (code, run(f, code)?),
    };
    let (result, printed) = outcome;
    let stack = match result {
        Ok(stack) => stack,
        Err(e) => return Ok(Some(format!("`{code}` fails with {}", Throw::new(e)))),
    };
    Ok(match test {
        Test::Run(_, expected) if stack != *expected => Some(format!(
            "`{code}` should leave {}, but it leaves {}",
            cells(expected),
            cells(&stack)
        )),
        Test::Print(_, expected) if printed.trim_end() != expected => Some(format!(
            "`{code}` should print \"{expected}\", but it prints \"{}\"",
            printed.trim_end()
        )),
        _ => None,
    })
}

/// Runs code from empty stacks in decimal and catches what it throws; the
/// stack it leaves (or the THROW code) and what it prints. The stacks,
/// BASE and the output are put back as they were.
type Outcome = (Result<Vec<Cell>, Cell>, String);

fn run(f: &mut Forth, code: &str) -> Res<Outcome> {
    let stack = std::mem::take(&mut f.stack);
    let fstack = std::mem::take(&mut f.fstack);
    let base = f.get(BASE_ADDR);
    let printed_from = f.out.len();
    f.set(BASE_ADDR, 10);
    let evaluate = f.state::<Progress>().evaluate;
    let result = f.push_bytes(code.as_bytes()).and_then(|()| {
        let text = f.stack[0] as usize;
        let caught = f.catch(evaluate as Cell);
        f.heap.free(text);
        caught
    });
    let left = std::mem::replace(&mut f.stack, stack);
    f.fstack = fstack;
    f.set(BASE_ADDR, base);
    let printed = String::from_utf8_lossy(&f.out.split_off(printed_from)).into_owned();
    match result? {
        // A limit reached cannot be caught; the program stops here.
        LIMIT_EXCEEDED => Err(Throw::with(LIMIT_EXCEEDED, "CHECK")),
        0 => Ok((Ok(left), printed)),
        code => Ok((Err(code), printed)),
    }
}

/// Cells as the stack shows them, or `empty`.
fn cells(stack: &[Cell]) -> String {
    if stack.is_empty() {
        return "empty".to_string();
    }
    stack.iter().map(Cell::to_string).collect::<Vec<_>>().join(" ")
}
