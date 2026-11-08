//! The ENCODING words, used from Forth.

use forthling::{Cell, Forth, Limits};
use forthling_extras::{EXTRAS_ERROR, Extras};

fn forth() -> Forth {
    let mut f = Forth::new();
    f.install(&Extras);
    f.interpret("ALSO ENCODING").unwrap();
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

#[test]
fn base64_both_ways() {
    let mut f = forth();
    f.interpret(SHOW).unwrap();
    assert_eq!(run(&mut f, r#"S" foob" >BASE64 SHOW"#), "Zm9vYg==");
    assert_eq!(run(&mut f, r#"S" Zm9vYg==" BASE64> SHOW"#), "foob");
    assert_eq!(run(&mut f, r#"S" " >BASE64 NIP . "#), "0 ");
    // Bytes that are not text: bytes 0 255 254.
    assert_eq!(
        run(&mut f, r#"S" AP/+" BASE64> DROP DUP C@ . DUP 1+ C@ . 2 + C@ . "#),
        "0 255 254 "
    );
}

#[test]
fn base64url_both_ways() {
    let mut f = forth();
    f.interpret(SHOW).unwrap();
    assert_eq!(run(&mut f, r#"S" -__-" BASE64URL> >HEX SHOW"#), "fbfffe");
    assert_eq!(run(&mut f, r#"S" fo" >BASE64URL SHOW"#), "Zm8");
    assert_eq!(run(&mut f, r#"S" Zm8=" BASE64URL> SHOW"#), "fo");
}

#[test]
fn hex_both_ways() {
    let mut f = forth();
    f.interpret(SHOW).unwrap();
    assert_eq!(run(&mut f, r#"S" Hi!" >HEX SHOW"#), "486921");
    assert_eq!(run(&mut f, r#"S" 486921" HEX> SHOW"#), "Hi!");
    assert_eq!(run(&mut f, r#"S" 4A4a" HEX> SHOW"#), "JJ");
}

#[test]
fn digest_and_checksum() {
    let mut f = forth();
    f.interpret(SHOW).unwrap();
    assert_eq!(
        run(&mut f, r#"S" abc" SHA-256 DUP . >HEX SHOW"#),
        "32 ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(run(&mut f, r#"S" 123456789" CRC-32 ."#), "3421780262 ");
    assert_eq!(run(&mut f, r#"S" " CRC-32 ."#), "0 ");
}

#[test]
fn bad_text_throws() {
    let mut f = forth();
    for text in [r#"S" Zg" BASE64>"#, r#"S" Zm9*" BASE64>"#, r#"S" abc" HEX>"#, r#"S" zz" HEX>"#] {
        assert_eq!(code(&mut f, text), EXTRAS_ERROR, "{text}");
    }
    assert_eq!(code(&mut f, r#"S" Z" BASE64URL>"#), EXTRAS_ERROR);
}

#[test]
fn a_wrong_address_is_an_error_not_a_crash() {
    let mut f = forth();
    assert_eq!(code(&mut f, "-1 5 SHA-256"), -9);
    assert_eq!(code(&mut f, "CRC-32"), -4);
}

#[test]
fn the_work_is_charged() {
    let mut f = Forth::sandboxed();
    f.install(&Extras);
    f.set_limits(Limits { steps: Some(1000), ..Limits::default() });
    f.interpret("ALSO ENCODING").unwrap();
    f.interpret("CREATE T 100000 ALLOT").unwrap();
    let e = f.interpret("T 100000 SHA-256").unwrap_err();
    assert_eq!(e.code, forthling::LIMIT_EXCEEDED, "{e}");
}

#[test]
fn every_word_has_a_help_line_and_its_examples_work() {
    let mut f = Forth::new();
    f.install(&Extras);
    let problems = f.help_problems();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    let examples: Vec<_> = f.help_examples().into_iter().filter(|e| e.list == "ENCODING").collect();
    assert!(examples.len() >= 6);
    for e in examples {
        let mut f = Forth::new();
        f.install(&Extras);
        f.interpret(&e.code).unwrap();
        let out = String::from_utf8_lossy(&f.take_output()).trim().to_string();
        assert_eq!(out, e.output, "{}: {}", e.name, e.code);
    }
}
