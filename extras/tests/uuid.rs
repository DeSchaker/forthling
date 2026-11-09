//! The UUID word, used from Forth.

use forthling::Forth;
use forthling_extras::Extras;

fn forth() -> Forth {
    let mut f = Forth::new();
    f.install(&Extras);
    f.interpret("ALSO UUID : SHOW ( c-addr u -- ) 2DUP TYPE DROP FREE THROW ;").unwrap();
    f
}

fn uuid(f: &mut Forth) -> String {
    let start = f.output().len();
    f.interpret("UUID4 SHOW").unwrap();
    String::from_utf8_lossy(&f.output()[start..]).into_owned()
}

#[test]
fn shape() {
    let mut f = forth();
    for _ in 0..50 {
        let u = uuid(&mut f);
        let b = u.as_bytes();
        assert_eq!(b.len(), 36, "{u}");
        for (i, c) in u.chars().enumerate() {
            if matches!(i, 8 | 13 | 18 | 23) {
                assert_eq!(c, '-', "{u}");
            } else {
                assert!(matches!(c, '0'..='9' | 'a'..='f'), "{u}");
            }
        }
        assert_eq!(b[14], b'4', "{u}");
        assert!(matches!(b[19], b'8' | b'9' | b'a' | b'b'), "{u}");
    }
}

#[test]
fn seed_makes_it_repeatable() {
    let mut f = forth();
    f.interpret("7 SEED!").unwrap();
    let a = uuid(&mut f);
    let b = uuid(&mut f);
    f.interpret("7 SEED!").unwrap();
    assert_eq!(uuid(&mut f), a);
    assert_eq!(uuid(&mut f), b);
    assert_ne!(a, b);
}

#[test]
fn text_is_allocated_and_charged() {
    let mut f = forth();
    // Freed text leaves the data stack empty and ALLOCATE'd memory returned.
    f.interpret("UUID4 DROP FREE THROW").unwrap();
    assert_eq!(f.depth(), 0);
}

#[test]
fn the_help_line_and_its_example_work() {
    let mut f = Forth::new();
    f.install(&Extras);
    let problems = f.help_problems();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    let examples: Vec<_> = f.help_examples().into_iter().filter(|e| e.list == "UUID").collect();
    assert_eq!(examples.len(), 1);
    for e in examples {
        let mut f = Forth::new();
        f.install(&Extras);
        f.interpret(&e.code).unwrap();
        let out = String::from_utf8_lossy(&f.take_output()).trim().to_string();
        assert_eq!(out, e.output, "{}: {}", e.name, e.code);
    }
}
