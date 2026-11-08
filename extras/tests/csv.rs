//! The CSV words, used from Forth.

use forthling::{Cell, Forth, Limits};
use forthling_extras::{EXTRAS_ERROR, Extras};

fn forth() -> Forth {
    let mut f = Forth::new();
    f.install(&Extras);
    f.interpret("ALSO CSV").unwrap();
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

/// Defines `FIELDS ( c-addr u -- )`: shows every field of the first record as
/// [field] and the flag of the last one, then what is left between braces.
const SHOW: &str = r#"
: FIELDS ( c-addr u -- )
  BEGIN  CSV-FIELD >R  ." [" TYPE ." ]"  R> 0= WHILE  REPEAT
  ." {" TYPE ." }" ;
"#;

#[test]
fn plain_fields() {
    let mut f = forth();
    f.interpret(SHOW).unwrap();
    assert_eq!(run(&mut f, r#"S" a,bc,d" FIELDS"#), "[a][bc][d]{}");
}

#[test]
fn a_record_ends_at_a_line_end() {
    let mut f = forth();
    f.interpret(SHOW).unwrap();
    assert_eq!(run(&mut f, r#"S\" a,b\nc,d\n" FIELDS"#), "[a][b]{c,d\n}");
    assert_eq!(run(&mut f, r#"S\" a,b\r\nc" FIELDS"#), "[a][b]{c}");
    assert_eq!(run(&mut f, r#"S\" a,b\rc" FIELDS"#), "[a][b]{c}");
}

#[test]
fn empty_fields() {
    let mut f = forth();
    f.interpret(SHOW).unwrap();
    assert_eq!(run(&mut f, r#"S" a,,c" FIELDS"#), "[a][][c]{}");
    // A separator at the end is followed by an empty field.
    assert_eq!(run(&mut f, r#"S" a," FIELDS"#), "[a][]{}");
    assert_eq!(run(&mut f, r#"S" ," FIELDS"#), "[][]{}");
    // An empty text is one empty field that ends the record.
    assert_eq!(run(&mut f, r#"S" " FIELDS"#), "[]{}");
    // An empty line is a record of one empty field.
    assert_eq!(run(&mut f, r#"S\" \nx" FIELDS"#), "[]{x}");
}

#[test]
fn quoted_fields() {
    let mut f = forth();
    f.interpret(SHOW).unwrap();
    // The separator and the line end belong to the field inside quotes.
    assert_eq!(run(&mut f, r#"S\" \"a,b\",\"c\nd\"\ne" FIELDS"#), "[a,b][c\nd]{e}");
    // A doubled quote stays doubled in CSV-FIELD.
    assert_eq!(run(&mut f, r#"S\" \"say \"\"hi\"\"\",x" FIELDS"#), "[say \"\"hi\"\"][x]{}");
    // A quoted empty field.
    assert_eq!(run(&mut f, r#"S\" \"\",x" FIELDS"#), "[][x]{}");
}

#[test]
fn csv_to_string_makes_doubled_quotes_single() {
    let mut f = forth();
    assert_eq!(
        run(&mut f, r#"S\" say \"\"hi\"\"" CSV>STRING 2DUP TYPE DROP FREE . "#),
        "say \"hi\"0 "
    );
    assert_eq!(run(&mut f, r#"S" plain" CSV>STRING 2DUP TYPE DROP FREE . "#), "plain0 ");
    assert_eq!(run(&mut f, r#"S" " CSV>STRING NIP ."#), "0 ");
}

#[test]
fn skip_a_record() {
    let mut f = forth();
    assert_eq!(run(&mut f, r#"S\" h1,h2\nx,y\n" CSV-SKIP TYPE"#), "x,y\n");
    assert_eq!(run(&mut f, r#"S\" \"a\nb\",c\nrest" CSV-SKIP TYPE"#), "rest");
    assert_eq!(run(&mut f, r#"S" only" CSV-SKIP NIP ."#), "0 ");
}

#[test]
fn another_separator() {
    let mut f = forth();
    f.interpret(SHOW).unwrap();
    f.interpret("59 CSV-SEPARATOR").unwrap();
    assert_eq!(run(&mut f, r#"S" 1,5;2,25" FIELDS"#), "[1,5][2,25]{}");
    f.interpret("9 CSV-SEPARATOR").unwrap();
    assert_eq!(run(&mut f, r#"S\" a\tb c\td" FIELDS"#), "[a][b c][d]{}");
}

#[test]
fn a_separator_that_cannot_be_one() {
    let mut f = forth();
    assert_eq!(code(&mut f, "34 CSV-SEPARATOR"), EXTRAS_ERROR);
    assert_eq!(code(&mut f, "10 CSV-SEPARATOR"), EXTRAS_ERROR);
    assert_eq!(code(&mut f, "13 CSV-SEPARATOR"), EXTRAS_ERROR);
    assert_eq!(code(&mut f, "-1 CSV-SEPARATOR"), EXTRAS_ERROR);
    assert_eq!(code(&mut f, "256 CSV-SEPARATOR"), EXTRAS_ERROR);
}

#[test]
fn quote_when_needed() {
    let mut f = forth();
    let quote = |f: &mut Forth, text: &str| {
        run(f, &format!(r#"S\" {text}" CSV-QUOTE 2DUP TYPE DROP FREE DROP"#))
    };
    assert_eq!(quote(&mut f, "plain"), "plain");
    assert_eq!(quote(&mut f, ""), "");
    assert_eq!(quote(&mut f, "a, b"), "\"a, b\"");
    assert_eq!(quote(&mut f, r#"12\" ruler"#), "\"12\"\" ruler\"");
    assert_eq!(quote(&mut f, r"two\nlines"), "\"two\nlines\"");
    // The separator decides.
    f.interpret("59 CSV-SEPARATOR").unwrap();
    assert_eq!(quote(&mut f, "a, b"), "a, b");
    assert_eq!(quote(&mut f, "a;b"), "\"a;b\"");
}

#[test]
fn what_is_written_is_read_back() {
    let mut f = forth();
    // Write with CSV-QUOTE, read with CSV-FIELD and CSV>STRING, show, free both.
    f.interpret(
        r": ROUND ( c-addr u -- )
              CSV-QUOTE 2DUP 2>R
              CSV-FIELD DROP CSV>STRING 2DUP TYPE DROP FREE DROP
              2DROP  2R> DROP FREE DROP ;",
    )
    .unwrap();
    let cases = [
        ("plain", "plain"),
        ("a,b", "a,b"),
        (r#"say \"hi\""#, "say \"hi\""),
        (r"line\none", "line\none"),
        ("", ""),
        (" spaced ", " spaced "),
    ];
    for (source, want) in cases {
        let got = run(&mut f, &format!(r#"S\" {source}" ROUND"#));
        assert_eq!(got, want, "{source}");
    }
}

#[test]
fn bad_text_throws() {
    let mut f = forth();
    // A quote that is never closed.
    assert_eq!(code(&mut f, r#"S\" \"open" CSV-FIELD"#), EXTRAS_ERROR);
    assert_eq!(code(&mut f, r#"S\" a,\"open" CSV-SKIP"#), EXTRAS_ERROR);
    // Text after the closing quote.
    assert_eq!(code(&mut f, r#"S\" \"a\"b,c" CSV-FIELD"#), EXTRAS_ERROR);
}

#[test]
fn a_wrong_address_is_an_error_not_a_crash() {
    let mut f = forth();
    assert_eq!(code(&mut f, "-1 5 CSV-FIELD"), -9);
    assert_eq!(code(&mut f, "CSV-FIELD"), -4);
}

#[test]
fn the_work_is_charged() {
    let mut f = Forth::sandboxed();
    f.install(&Extras);
    f.set_limits(Limits { steps: Some(1000), ..Limits::default() });
    f.interpret("ALSO CSV").unwrap();
    f.interpret("CREATE T 100000 ALLOT").unwrap();
    let e = f.interpret("T 100000 CSV-FIELD").unwrap_err();
    assert_eq!(e.code, forthling::LIMIT_EXCEEDED, "{e}");
}

#[test]
fn every_word_has_a_help_line_and_its_examples_work() {
    let mut f = Forth::new();
    f.install(&Extras);
    let problems = f.help_problems();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    let examples: Vec<_> = f.help_examples().into_iter().filter(|e| e.list == "CSV").collect();
    assert!(examples.len() >= 2);
    for e in examples {
        let mut f = Forth::new();
        f.install(&Extras);
        f.interpret(&e.code).unwrap();
        let out = String::from_utf8_lossy(&f.take_output()).trim().to_string();
        assert_eq!(out, e.output, "{}: {}", e.name, e.code);
    }
}

#[test]
fn the_examples_give_the_output_beside_them() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/examples");
    let mut seen = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "fs") {
            continue;
        }
        let mut f = Forth::new();
        f.install(&Extras);
        f.include(path.to_str().unwrap()).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let out = String::from_utf8_lossy(f.output()).into_owned();
        let expected = std::fs::read_to_string(path.with_extension("out")).unwrap();
        assert_eq!(out, expected, "{}", path.display());
        seen += 1;
    }
    assert!(seen >= 1);
}
