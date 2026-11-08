//! The URL words, used from Forth.

use forthling::{Cell, Forth, Limits};
use forthling_extras::{EXTRAS_ERROR, Extras};

fn forth() -> Forth {
    let mut f = Forth::new();
    f.install(&Extras);
    f.interpret("ALSO URL").unwrap();
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

/// Shows the text that a word gives and frees it.
const SHOW: &str = ": SHOW ( c-addr u -- ) 2DUP TYPE DROP FREE THROW ;";

const URL: &str = r#"S" https://user:pw@example.com:8080/a/b.html?x=1&y=2#top""#;

#[test]
fn percent_both_ways() {
    let mut f = forth();
    f.interpret(SHOW).unwrap();
    assert_eq!(run(&mut f, r#"S" a b&c=d/~-._" >PERCENT SHOW"#), "a%20b%26c%3Dd%2F~-._");
    assert_eq!(run(&mut f, r#"S" a%20b%26c%3dd+" PERCENT> SHOW"#), "a b&c=d+");
    assert_eq!(run(&mut f, r#"S" " >PERCENT SHOW"#), "");
    // Bytes of a multi-byte character are all encoded.
    assert_eq!(run(&mut f, "S\" \u{e9}\" >PERCENT SHOW"), "%C3%A9");
    assert_eq!(run(&mut f, r#"S" %C3%A9" PERCENT> SHOW"#), "\u{e9}");
    assert_eq!(run(&mut f, r#"S" %00x" PERCENT> >PERCENT SHOW"#), "%00x");
}

#[test]
fn bad_percent_text_throws() {
    let mut f = forth();
    for bad in ["%", "a%2", "%zz", "%%20"] {
        assert_eq!(code(&mut f, &format!("S\" {bad}\" PERCENT>")), EXTRAS_ERROR, "{bad}");
    }
}

#[test]
fn the_parts_of_a_url() {
    let mut f = forth();
    for (word, want) in [
        ("URL-SCHEME", "https"),
        ("URL-USERINFO", "user:pw"),
        ("URL-HOST", "example.com"),
        ("URL-PATH", "/a/b.html"),
        ("URL-QUERY", "x=1&y=2"),
        ("URL-FRAGMENT", "top"),
    ] {
        assert_eq!(run(&mut f, &format!("{URL} {word} TYPE")), want, "{word}");
    }
    assert_eq!(run(&mut f, &format!("{URL} URL-PORT .")), "8080 ");
}

#[test]
fn parts_that_are_not_there_are_empty() {
    let mut f = forth();
    assert_eq!(run(&mut f, r#"S" /a/b" URL-HOST NIP ."#), "0 ");
    assert_eq!(run(&mut f, r#"S" /a/b" URL-SCHEME NIP ."#), "0 ");
    assert_eq!(run(&mut f, r#"S" /a/b" URL-PORT ."#), "-1 ");
    assert_eq!(run(&mut f, r#"S" http://example.com" URL-PORT ."#), "-1 ");
    assert_eq!(run(&mut f, r#"S" http://example.com" URL-PATH NIP ."#), "0 ");
    assert_eq!(run(&mut f, r#"S" http://example.com/a" URL-QUERY NIP ."#), "0 ");
    assert_eq!(run(&mut f, r#"S" " URL-HOST NIP ."#), "0 ");
}

#[test]
fn the_answer_points_into_the_text() {
    let mut f = forth();
    // The slice is in the given text, so its address is inside it.
    assert_eq!(run(&mut f, r#"S" http://h/p?q" OVER >R URL-QUERY DROP R> - ."#), "11 ");
}

#[test]
fn bad_authorities_throw() {
    let mut f = forth();
    for bad in ["http://host:80a/", "http://host:65536/", "http://[::1/", "http://host:1:2/"] {
        for word in ["URL-HOST", "URL-PORT", "URL-PATH"] {
            assert_eq!(code(&mut f, &format!("S\" {bad}\" {word}")), EXTRAS_ERROR, "{bad}");
        }
    }
}

#[test]
fn query_parameters() {
    let mut f = forth();
    f.interpret(SHOW).unwrap();
    f.interpret(": Q S\" a=1&b=x%20y&c&a=2\" ;").unwrap();
    assert_eq!(run(&mut f, r#"Q S" a" URL-PARAM . TYPE"#), "-1 1");
    assert_eq!(run(&mut f, r#"Q S" b" URL-PARAM . PERCENT> SHOW"#), "-1 x y");
    assert_eq!(run(&mut f, r#"Q S" c" URL-PARAM . NIP ."#), "-1 0 ");
    assert_eq!(run(&mut f, r#"Q S" d" URL-PARAM ."#), "0 ");
    assert_eq!(run(&mut f, &format!("{URL} URL-QUERY S\" y\" URL-PARAM . TYPE")), "-1 2");
}

#[test]
fn the_work_is_counted_and_the_stack_is_checked() {
    let mut f = Forth::sandboxed();
    f.install(&Extras);
    f.set_limits(Limits { steps: Some(1000), ..Limits::default() });
    f.interpret("ALSO URL").unwrap();
    f.interpret("CREATE T 100000 ALLOT").unwrap();
    let e = f.interpret("T 100000 URL-HOST").unwrap_err();
    assert_eq!(e.code, forthling::LIMIT_EXCEEDED, "{e}");
    let mut f = forth();
    assert_eq!(code(&mut f, "1 URL-HOST"), -4);
    assert_eq!(code(&mut f, "S\" a\" URL-PARAM"), -4);
}

#[test]
fn every_word_has_a_help_line_and_its_examples_work() {
    let mut f = Forth::new();
    f.install(&Extras);
    let problems = f.help_problems();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    let examples: Vec<_> = f.help_examples().into_iter().filter(|e| e.list == "URL").collect();
    assert!(examples.len() >= 9);
    for e in examples {
        let mut f = Forth::new();
        f.install(&Extras);
        f.interpret(&e.code).unwrap();
        let out = String::from_utf8_lossy(&f.take_output()).trim().to_string();
        assert_eq!(out, e.output, "{}: {}", e.name, e.code);
    }
}
