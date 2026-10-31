//! The JSON words, used from Forth.

use forthling::{Cell, Forth, Json, Limits};

fn forth() -> Forth {
    let mut f = Forth::new();
    f.install(&Json);
    f.interpret("ALSO JSON").unwrap();
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

/// Defines DOC as a word that gives this JSON text, kept in the dictionary.
fn doc(f: &mut Forth, json: &str) {
    let escaped = json.replace('\\', r"\\").replace('"', r#"\""#);
    run(f, &format!(r#": DOC S\" {escaped}" ;"#));
}

#[test]
fn validity_and_types() {
    let mut f = forth();
    let cases = [
        (r#"  {"a": [1, 2.5e-3, -0, true, null]}  "#, 5),
        ("[]", 4),
        (r#""x""#, 3),
        ("-12.5E+2", 2),
        ("false", 1),
        ("null", 0),
        ("01", -1),
        ("1.", -1),
        ("[1,]", -1),
        (r#"{"a" 1}"#, -1),
        (r#"{"a":1} x"#, -1),
        ("tru", -1),
        (r#""tab	in string""#, -1),
        (r#""\x""#, -1),
        ("", -1),
    ];
    for (json, kind) in cases {
        doc(&mut f, json);
        assert_eq!(run(&mut f, "DOC JSON-TYPE ."), format!("{kind} "), "{json}");
        let valid = if kind == -1 { "0 " } else { "-1 " };
        assert_eq!(run(&mut f, "DOC JSON-VALID? ."), valid, "{json}");
    }
    // Deep nesting is refused, not a crash. The text is longer than a line,
    // so it goes on the stack from Rust.
    for (depth, valid) in [(600, "0 "), (500, "-1 ")] {
        let deep = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
        f.push_bytes(deep.as_bytes()).unwrap();
        assert_eq!(run(&mut f, "JSON-VALID? ."), valid, "{depth}");
    }
}

#[test]
fn looking_things_up() {
    let mut f = forth();
    doc(&mut f, r#"{"name": "Ada", "langs": ["en", "fr"], "a.b": {"c": 7}, "name": "Ada L."}"#);
    // The pieces are the text itself.
    assert_eq!(run(&mut f, r#"DOC S" langs" JSON-GET . TYPE"#), r#"-1 ["en", "fr"]"#);
    // A key that occurs twice: the last one, as JavaScript does.
    assert_eq!(run(&mut f, r#"DOC S" name" JSON-GET DROP TYPE"#), r#""Ada L.""#);
    assert_eq!(run(&mut f, r#"DOC S" nobody" JSON-GET . . ."#), "0 0 0 ");
    assert_eq!(run(&mut f, "DOC JSON-LENGTH . DOC 2 JSON-NTH DROP TYPE"), r#"4 {"c": 7}"#);
    assert_eq!(run(&mut f, "DOC 2 JSON-NAME . 2DUP TYPE DROP FREE ."), "-1 a.b0 ");
    assert_eq!(run(&mut f, "DOC 9 JSON-NAME . . . DOC -1 JSON-NTH . . ."), "0 0 0 0 0 0 ");
    assert_eq!(run(&mut f, r#"DOC S" langs[1]" JSON@ DROP TYPE"#), r#""fr""#);
    assert_eq!(run(&mut f, r#"DOC S\" [\"a.b\"].c" JSON@ DROP TYPE"#), "7");
    assert_eq!(run(&mut f, r#"DOC S" .langs" JSON@ DROP JSON-LENGTH ."#), "2 ");
    assert_eq!(run(&mut f, r#"DOC S" " JSON@ DROP JSON-TYPE ."#), "5 ");
    assert_eq!(
        run(&mut f, r#"DOC S" langs[2]" JSON@ . 2DROP DOC S" langs.x" JSON@ . 2DROP"#),
        "0 0 "
    );
    assert_eq!(
        run(&mut f, r#"DOC S" name[0]" JSON@ . 2DROP DOC S" langs[x]" JSON@ . 2DROP"#),
        "0 0 "
    );
    // Scalars have no elements.
    assert_eq!(run(&mut f, r#"S" 5" JSON-LENGTH . S" 5" 0 JSON-NTH . 2DROP"#), "-1 0 ");
    // Escaped keys are compared without their escapes.
    doc(&mut f, r#"{"café": 1}"#);
    assert_eq!(run(&mut f, r#"DOC S" café" JSON-GET DROP TYPE"#), "1");
}

#[test]
fn values() {
    let mut f = forth();
    let number = |f: &mut Forth, json: &str| run(f, &format!(r#"S" {json}" JSON>NUMBER . ."#));
    assert_eq!(number(&mut f, "-42"), "-1 -42 ");
    assert_eq!(number(&mut f, "9223372036854775807"), "-1 9223372036854775807 ");
    assert_eq!(number(&mut f, "9223372036854775808"), "0 0 "); // too big for a cell
    assert_eq!(number(&mut f, "1.5"), "0 0 ");
    assert_eq!(number(&mut f, "true"), "0 0 ");
    assert_eq!(run(&mut f, r#"S" 1.5e2" JSON>FLOAT . F. S" 7" JSON>FLOAT . F."#), "-1 150. -1 7. ");
    assert_eq!(run(&mut f, r#"S" null" JSON>FLOAT . DEPTH ."#), "0 0 ");
    assert_eq!(
        run(&mut f, r#"S" true" JSON>FLAG . . S" false" JSON>FLAG . . S" 1" JSON>FLAG . ."#),
        "-1 -1 -1 0 0 0 "
    );
    let string = |f: &mut Forth, json: &str| {
        doc(f, json);
        run(f, "DOC JSON>STRING . 2DUP TYPE DROP FREE DROP")
    };
    assert_eq!(string(&mut f, r#""a\"b\\c\/d\n\tx""#), "-1 a\"b\\c/d\n\tx");
    assert_eq!(string(&mut f, r#""é€😀""#), "-1 é€😀");
    assert_eq!(string(&mut f, r#""\ud83d alone""#), "-1 \u{FFFD} alone");
    assert_eq!(run(&mut f, r#"S" 12" JSON>STRING . . ."#), "0 0 0 ");
}

#[test]
fn writing() {
    let mut f = forth();
    let out = run(
        &mut f,
        r#"JSON{ S" name" JSON-KEY S" Ada" JSON-STRING
             S" age" JSON-KEY 36 JSON-NUMBER
             S" tags" JSON-KEY JSON[ S" math" JSON-STRING 1 JSON-FLAG 0 JSON-FLAG JSON-NULL JSON]
             S" empty" JSON-KEY JSON{ JSON} JSON}"#,
    );
    assert_eq!(out, r#"{"name":"Ada","age":36,"tags":["math",true,false,null],"empty":{}}"#);
    assert_eq!(run(&mut f, "HEX FF JSON-NUMBER DECIMAL"), "255"); // always decimal
    let floats =
        "JSON[ 1.5E0 JSON-FLOAT 1E300 JSON-FLOAT 1E-7 JSON-FLOAT 0E0 0E0 F/ JSON-FLOAT JSON]";
    assert_eq!(run(&mut f, floats), "[1.5,1e300,1e-7,null]");
    let text = r#"S\" say \"hi\"\\\n\a" JSON-STRING"#;
    assert_eq!(run(&mut f, text), r#""say \"hi\"\\\n\u0007""#);
    assert_eq!(
        run(&mut f, r#"JSON[ S\" {\"a\": [1, 2]}" JSON-RAW 3 JSON-NUMBER JSON]"#),
        r#"[{"a": [1, 2]},3]"#
    );
    assert_eq!(code(&mut f, r#"S" {oops" JSON-RAW"#), -24);
    // Bytes that are no UTF-8 become U+FFFD.
    assert_eq!(run(&mut f, "HERE 255 C, 1 JSON-STRING"), "\"\u{FFFD}\"");
}

#[test]
fn writing_to_a_string() {
    let mut f = forth();
    let out = run(
        &mut f,
        r#"<JSON JSON{ S" n" JSON-KEY 5 JSON-NUMBER JSON} JSON> 2DUP TYPE SPACE JSON-TYPE ."#,
    );
    assert_eq!(out, r#"{"n":5} 5 "#);
    // Nested: the inner string goes into the outer one as a JSON string.
    let nested = "<JSON JSON[ 1 JSON-NUMBER <JSON JSON{ JSON} JSON> JSON-STRING JSON] JSON> TYPE";
    assert_eq!(run(&mut f, nested), r#"[1,"{}"]"#);
    assert_eq!(run(&mut f, "<JSON JSON> . DROP"), "0 ");
    // A string as big as memory does not fit.
    let mut small = Forth::with_memory(64 << 10);
    small.install(&Json);
    small.interpret("ALSO JSON").unwrap();
    let big = ": BIG <JSON JSON[ 20000 0 DO 1234 JSON-NUMBER LOOP JSON] JSON> ; BIG";
    assert_eq!(code(&mut small, big), -59);
}

#[test]
fn mistakes_in_writing() {
    let mut f = forth();
    assert_eq!(code(&mut f, "JSON{ 1 JSON-NUMBER"), -22); // a value needs its key
    assert_eq!(code(&mut f, r#"JSON[ S" k" JSON-KEY"#), -22);
    assert_eq!(code(&mut f, r#"JSON{ S" k" JSON-KEY S" k" JSON-KEY"#), -22);
    assert_eq!(code(&mut f, r#"JSON{ S" k" JSON-KEY JSON}"#), -22); // the key has no value
    assert_eq!(code(&mut f, "JSON[ JSON}"), -22);
    assert_eq!(code(&mut f, "JSON]"), -22);
    assert_eq!(code(&mut f, "JSON>"), -22);
    assert_eq!(code(&mut f, "<JSON JSON[ JSON>"), -22);
    // After an error, what was half written is forgotten.
    assert_eq!(code(&mut f, "<JSON JSON[ 1 JSON-NUMBER NOSUCHWORD"), -13);
    assert_eq!(run(&mut f, "JSON[ JSON]"), "[]");
    assert_eq!(code(&mut f, "JSON>"), -22);
}

#[test]
fn json_and_the_sandbox() {
    let mut f = Forth::sandboxed();
    f.install(&Json);
    assert_eq!(run(&mut f, r#"S" FORTHLING-JSON" ENVIRONMENT? . ."#), "-1 1 ");
    f.set_limits(Limits { output: Some(5), ..Limits::default() });
    assert_eq!(code(&mut f, r#"ALSO JSON S" a long string" JSON-STRING"#), -258);
    // Without ALSO JSON, a standard program does not see the words.
    let mut plain = Forth::new();
    plain.install(&Json);
    assert_eq!(code(&mut plain, "JSON-TYPE"), -13);
}

#[test]
fn json_within_bounds() {
    // Nested <JSON strings count together.
    let mut f = Forth::with_memory(64 << 10);
    f.install(&Json);
    f.interpret("ALSO JSON").unwrap();
    let nested = ": FILL-IT JSON[ 3000 0 DO 1234567 JSON-NUMBER LOOP JSON] ; \
                  : NEST <JSON FILL-IT <JSON FILL-IT <JSON FILL-IT ; NEST";
    assert_eq!(code(&mut f, nested), -59);
    // Writing nests 512 deep at most, as reading does.
    let mut f = forth();
    assert_eq!(code(&mut f, ": DEEP 600 0 DO JSON[ LOOP ; DEEP"), -22);
    assert_eq!(code(&mut f, ": DEEP 600 0 DO <JSON LOOP ; DEEP"), -22);
    // Reading a long text costs steps in proportion.
    let mut f = forth();
    f.set_limits(Limits { steps: Some(1000), ..Limits::default() });
    let long = format!("[{}0]", "0,".repeat(100_000));
    f.push_bytes(long.as_bytes()).unwrap();
    assert_eq!(code(&mut f, "JSON-VALID?"), -258);
    // The JSON strings count for the space limit.
    let mut f = forth();
    f.set_limits(Limits { space: Some(10_000), ..Limits::default() });
    let big = ": BIG <JSON JSON[ 5000 0 DO 1 JSON-NUMBER LOOP JSON] JSON> ; BIG";
    assert_eq!(code(&mut f, big), -258);
}
