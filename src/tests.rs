//! Unit tests; the Forth-2012 test suite runs from tests/suite.rs.

use super::*;
use std::{env, fs};

fn output(f: Forth) -> String {
    String::from_utf8(f.out).unwrap()
}

fn run(src: &str) -> String {
    let mut f = Forth::new();
    f.interpret(src).unwrap();
    output(f)
}

fn err(src: &str) -> Throw {
    Forth::new().interpret(src).unwrap_err()
}

fn temp_file(name: &str, content: &str) -> String {
    let path = env::temp_dir().join(format!("forth-test-{}-{name}", std::process::id()));
    fs::write(&path, content).unwrap();
    path.to_string_lossy().into_owned()
}

#[test]
fn arithmetic() {
    assert_eq!(run("2 3 + 4 * ."), "20 ");
    assert_eq!(run("-7 2 / . -7 2 MOD ."), "-4 1 ");
    assert_eq!(run("7 -2 / . 7 -2 MOD ."), "-4 -1 ");
    assert_eq!(run("10 3 /MOD . ."), "3 1 ");
    assert_eq!(run("1000000 3000000 1000 */ ."), "3000000000 ");
    assert_eq!(run("-7 2 1 */MOD . ."), "-14 0 ");
    assert_eq!(run("-8 2/ ."), "-4 ");
}

#[test]
fn double_cells() {
    assert_eq!(run("-5 S>D . ."), "-1 -5 ");
    assert_eq!(run("-1 -1 UM* . U."), "-2 1 ");
    assert_eq!(run("-1 2 M* . ."), "-1 -2 ");
    assert_eq!(run("0 1 4 UM/MOD . ."), "4611686018427387904 0 ");
    assert_eq!(run("-7 S>D 2 FM/MOD . . -7 S>D 2 SM/REM . ."), "-4 1 -3 -1 ");
    assert_eq!(run("7 S>D -2 FM/MOD . . 7 S>D -2 SM/REM . ."), "-4 -1 -3 1 ");
    assert_eq!(run("CREATE P 2 CELLS ALLOT 1 2 P 2! P @ . P 2@ . ."), "2 2 1 ");
    assert_eq!(err("0 1 1 UM/MOD").code, -11);
    assert_eq!(err("1 0 0 FM/MOD").code, -10);
    assert_eq!(err("0 -9223372036854775808 -1 SM/REM").code, -11);
}

#[test]
fn pictured_output() {
    assert_eq!(run("<# 66 HOLD 65 HOLD 0 0 #> TYPE"), "AB");
    assert_eq!(run("<# 1 0 # # #> TYPE"), "01");
    assert_eq!(run("<# -1 SIGN 0 SIGN 255 0 #S #> TYPE"), "255-");
    assert_eq!(run("255 HEX <# 0 #S -1 SIGN #> TYPE"), "-FF");
    assert_eq!(run(r#"<# 0 0 #S S" x=" HOLDS #> TYPE"#), "x=0");
    assert_eq!(run("-1 -1 2 BASE ! <# #S #> NIP DECIMAL ."), "128 ");
    assert_eq!(
        run("0 . -42 . -1 U. -9223372036854775808 ."),
        "0 -42 18446744073709551615 -9223372036854775808 "
    );
    assert_eq!(run("12 1 BASE ! . DECIMAL"), "12 ");
    assert_eq!(err(": T <# 300 0 DO 65 HOLD LOOP ; T").code, -17);
}

#[test]
fn stack_ops() {
    assert_eq!(run("1 2 3 ROT .S"), "<3> 2 3 1 ");
    assert_eq!(run("1 2 3 4 2SWAP .S"), "<4> 3 4 1 2 ");
    assert_eq!(run("1 2 3 4 2OVER .S"), "<6> 1 2 3 4 1 2 ");
    assert_eq!(run("5 0 ?DUP .S"), "<2> 5 0 ");
}

#[test]
fn conditionals() {
    let src = ": SIGN DUP 0< IF DROP -1 ELSE 0> IF 1 ELSE 0 THEN THEN ; \
               -5 SIGN . 0 SIGN . 9 SIGN .";
    assert_eq!(run(src), "-1 0 1 ");
}

#[test]
fn recursion() {
    assert_eq!(run(": FAC DUP 1 > IF DUP 1- RECURSE * THEN ; 10 FAC ."), "3628800 ");
    assert_eq!(err(": FOREVER RECURSE ; FOREVER").code, -5);
}

#[test]
fn loops() {
    assert_eq!(run(": T 5 0 DO I . LOOP ; T"), "0 1 2 3 4 ");
    assert_eq!(run(": T 0 10 DO I . -3 +LOOP ; T"), "10 7 4 1 ");
    assert_eq!(run(": T 3 0 DO 2 0 DO J 10 * I + . LOOP LOOP ; T"), "0 1 10 11 20 21 ");
    assert_eq!(run(": T 10 0 DO I 3 = IF LEAVE THEN I . LOOP ; T"), "0 1 2 ");
    assert_eq!(run(": T 0 0 ?DO 1 . LOOP ; T"), "");
    assert_eq!(run(": T 3 BEGIN DUP . 1- DUP 0= UNTIL DROP ; T"), "3 2 1 ");
    assert_eq!(run(": T 0 BEGIN DUP 3 < WHILE DUP . 1+ REPEAT DROP ; T"), "0 1 2 ");
    let two_whiles = ": T BEGIN DUP 2 > WHILE DUP 5 < WHILE 1+ REPEAT 123 ELSE 345 THEN ; \
                      1 T . . 3 T . .";
    assert_eq!(run(two_whiles), "345 1 123 5 ");
    assert_eq!(run(": T 10 0 DO I 2 = IF UNLOOP EXIT THEN I . LOOP ; T 7 ."), "0 1 7 ");
}

#[test]
fn variables_and_constants() {
    assert_eq!(run("VARIABLE X 5 X ! 3 X +! X @ ."), "8 ");
    assert_eq!(run("42 CONSTANT ANSWER ANSWER ."), "42 ");
    assert_eq!(run("CREATE BUF 3 CELLS ALLOT 9 BUF 2 CELLS + ! BUF 2 CELLS + @ ."), "9 ");
}

#[test]
fn byte_addressed_memory() {
    assert_eq!(run("1 CELLS . 1 CHARS . 0 CELL+ . 0 CHAR+ ."), "8 1 8 1 ");
    // cells are little-endian bytes
    assert_eq!(run("VARIABLE V 258 V ! V C@ . V 1+ C@ ."), "2 1 ");
    assert_eq!(run("VARIABLE V -1 V ! 0 V C! V @ ."), "-256 ");
    // CREATE aligns its data field after an odd C,
    assert_eq!(run("1 C, CREATE X X 8 MOD ."), "0 ");
    assert_eq!(run("0 ALIGNED . 1 ALIGNED . 8 ALIGNED . 9 ALIGNED ."), "0 8 8 16 ");
    assert_eq!(run("1 C, ALIGN HERE 8 MOD ."), "0 ");
    assert_eq!(run("CREATE B 8 ALLOT B 8 65 FILL B 3 66 FILL B 0 67 FILL B 8 TYPE"), "BBBAAAAA");
    let moves = r#"CREATE B 8 ALLOT S" abcdef" B SWAP MOVE
                   B B 2 + 3 MOVE B 6 TYPE  B 2 + B 3 MOVE B 6 TYPE  B B 0 MOVE"#;
    assert_eq!(run(moves), "ababcfabcbcf");
    assert_eq!(run("-5 0 0 FILL 0 -1 0 MOVE"), "");
    assert_eq!(err("-5 8 0 FILL").code, -9);
    assert_eq!(err("0 -5 8 MOVE").code, -9);
    assert_eq!(err("-1 @").code, -9);
    assert_eq!(err("1048570 @").code, -9);
}

#[test]
fn create_does() {
    assert_eq!(run(": KONST CREATE , DOES> @ ; 7 KONST SEVEN SEVEN SEVEN + ."), "14 ");
    let counter = ": COUNTER CREATE 0 , DOES> 1 OVER +! @ ; COUNTER TICK TICK . TICK . TICK .";
    assert_eq!(run(counter), "1 2 3 ");
    assert_eq!(run("CREATE X 5 , ' X >BODY X = . ' X >BODY @ ."), "-1 5 ");
    assert_eq!(run(": K CREATE , DOES> @ ; 3 K Y ' Y >BODY @ ."), "3 ");
    assert_eq!(err("' DUP >BODY").code, -31);
}

#[test]
fn strings_and_chars() {
    assert_eq!(run(r#": HI ." Hello, Forth!" CR ; HI"#), "Hello, Forth!\n");
    assert_eq!(run(r#"S" abc" TYPE"#), "abc");
    assert_eq!(run(r#": T S" xyz" TYPE ; T"#), "xyz");
    assert_eq!(run("CHAR A EMIT 'B' EMIT"), "AB");
    assert_eq!(run(": T [CHAR] Z EMIT ; T"), "Z");
    assert_eq!(run("1 ( a comment ) 2 \\ rest of line\n + ."), "3 ");
}

#[test]
fn transient_strings_leave_here_alone() {
    assert_eq!(run(r#"HERE S" abc" 2DROP HERE = ."#), "-1 ");
    // two S" strings are usable at the same time
    assert_eq!(run(r#"S" one" S" two" 2SWAP TYPE TYPE"#), "onetwo");
}

#[test]
fn number_bases() {
    assert_eq!(run("HEX FF DECIMAL ."), "255 ");
    assert_eq!(run("255 HEX . DECIMAL"), "FF ");
    assert_eq!(run("$10 #10 %10 + + ."), "28 ");
    assert_eq!(run("-1 U."), "18446744073709551615 ");
    assert_eq!(run("18446744073709551615 -9223372036854775808 . ."), "-9223372036854775808 -1 ");
    assert_eq!(err("18446744073709551616").code, -13);
    assert_eq!(err("12A").code, -13);
}

#[test]
fn to_number_stops_at_the_first_non_digit() {
    assert_eq!(run(r#"0 0 S" 123xy" >NUMBER TYPE . ."#), "xy0 123 ");
    assert_eq!(run(r#"1 0 S" 23" >NUMBER . DROP . ."#), "0 0 123 ");
    assert_eq!(run(r#"0 0 S" ff" HEX >NUMBER DECIMAL . DROP . ."#), "0 0 255 ");
    assert_eq!(run(r#"0 0 S" -5" >NUMBER . DROP . ."#), "2 0 0 ");
    let max_ud = r#"0 0 S" 340282366920938463463374607431768211455" >NUMBER . DROP U. U."#;
    assert_eq!(run(max_ud), "0 18446744073709551615 18446744073709551615 ");
}

#[test]
fn input_buffer_and_parsing() {
    // SOURCE is the current line, >IN the offset just past the parsed word
    assert_eq!(run("SOURCE TYPE"), "SOURCE TYPE");
    assert_eq!(run(">IN @ ."), "6 ");
    // moving >IN skips or re-reads input
    assert_eq!(run("1 >IN +! x2 ."), "2 ");
    assert_eq!(run(": SKIP-REST SOURCE >IN ! DROP ; 1 . SKIP-REST 2 ."), "1 ");
    let reread = "VARIABLE N 0 N !\nN @ 1+ DUP N ! 3 < 0= 8 AND >IN +! 0 >IN ! N @ .";
    assert_eq!(run(reread), "3 ");
    // WORD and COUNT
    assert_eq!(run(": MSG 41 WORD COUNT ; MSG hi there) TYPE"), "hi there");
    assert_eq!(run("BL WORD   spaced COUNT TYPE"), "spaced");
    assert_eq!(run("BL WORD xyz C@ ."), "3 ");
}

#[test]
fn find_and_state() {
    assert_eq!(run("BL WORD DUP FIND NIP ."), "-1 ");
    assert_eq!(run("BL WORD IF FIND NIP ."), "1 ");
    assert_eq!(run("BL WORD NOSUCHWORD FIND . COUNT TYPE"), "0 NOSUCHWORD");
    assert_eq!(run("BL WORD dup FIND SWAP ' DUP = . ."), "-1 -1 ");
    assert_eq!(run("STATE @ . : T [ STATE @ ] LITERAL ; T ."), "0 0 ");
    assert_eq!(run(": NOW STATE @ ; IMMEDIATE : T NOW LITERAL ; T ."), "-1 ");
}

#[test]
fn include_nests_and_restores_input() {
    let inner = temp_file("inner.fs", "10 20\n+ \\ a comment\n");
    let outer = temp_file("outer.fs", &format!("1 INCLUDE {inner} 2 .S\n"));
    assert_eq!(run(&format!("INCLUDE {outer} 3 .S")), "<3> 1 30 2 <4> 1 30 2 3 ");
}

fn run_with_input(src: &str, input: &str) -> String {
    let mut f = Forth::new();
    f.input = Box::new(io::Cursor::new(input.as_bytes().to_vec()));
    f.interpret(src).unwrap();
    output(f)
}

#[test]
fn accept_and_key_read_the_terminal() {
    let accept = "CREATE B 10 ALLOT : T B 10 ACCEPT B SWAP TYPE [CHAR] | EMIT ; T T T";
    assert_eq!(run_with_input(accept, "hello\r\nforthling rocks\n"), "hello|forthling ||");
    assert_eq!(run_with_input("KEY . KEY . KEY .", "ab\n"), "97 98 10 ");
    assert_eq!(run_with_input("CREATE B 4 ALLOT B 4 ACCEPT . KEY .", "x\ny"), "1 121 ");
    let mut f = Forth::new();
    f.input = Box::new(io::Cursor::new(Vec::new()));
    assert_eq!(f.interpret("KEY").unwrap_err().code, -39);
}

#[test]
fn abort_and_quit() {
    let mut f = Forth::new();
    let e = f.interpret("1 2 ABORT 3").unwrap_err();
    assert_eq!((e.code, f.stack.len()), (-1, 0));
    f.interpret(r#": CHECK  DUP 0< ABORT" negative!" ;  5 CHECK"#).unwrap();
    assert_eq!(f.stack, [5]);
    let e = f.interpret("-5 CHECK").unwrap_err();
    assert_eq!(e.code, -2);
    assert_eq!(e.to_string(), "error -2: aborted: negative!");
    assert!(f.stack.is_empty());
    assert_eq!(f.interpret(r#"1 ABORT" direct""#).unwrap_err().code, -2);
    f.interpret(r#"0 ABORT" not shown""#).unwrap();
    // QUIT empties the return stack and leaves compilation, but keeps the data stack.
    let e = f.interpret(": Q 7 8 >R QUIT ; 1 2 Q 3").unwrap_err();
    assert_eq!(e.code, QUIT);
    assert_eq!((f.stack.as_slice(), f.rstack.len()), ([1, 2, 7].as_slice(), 0));
    f.interpret(": HALF [ QUIT").unwrap_err();
    assert!(!f.compiling() && f.find("HALF").is_none());
    assert_eq!(output(f), "");
}

#[test]
fn core_ext_basics() {
    assert_eq!(run("1 2 3 4 3 ROLL .S"), "<4> 2 3 4 1 ");
    assert_eq!(run("5 0<> . 0 0<> . -1 1 U> . 1 -1 U> ."), "-1 0 -1 0 ");
    assert_eq!(run("2 1 3 WITHIN . 3 1 3 WITHIN . -1 -5 5 WITHIN ."), "-1 0 -1 ");
    assert_eq!(run(": T 1 2 2>R 2R@ 2R> ; T .S"), "<4> 1 2 1 2 ");
    assert_eq!(run("42 5 .R -42 5 .R 42 1 .R 7 3 U.R"), "   42  -4242  7");
    assert_eq!(run("PAD 3 65 FILL PAD 1 ERASE PAD C@ . PAD 1+ C@ ."), "0 65 ");
    assert_eq!(run("UNUSED HERE 100 ALLOT DROP UNUSED - ."), "100 ");
}

#[test]
fn noname_and_compiling_words() {
    assert_eq!(run(":NONAME 6 7 * ; EXECUTE ."), "42 ");
    assert_eq!(run(":NONAME DUP IF 1- RECURSE THEN ; 5 SWAP EXECUTE ."), "0 ");
    assert_eq!(run(":NONAME 1 ; DROP CREATE E 0 C, E FIND NIP ."), "0 ");
    assert_eq!(run(": Q ['] DUP COMPILE, ; IMMEDIATE : T Q * ; 7 T ."), "49 ");
    assert_eq!(run(": MYIF [COMPILE] IF ; IMMEDIATE : T MYIF 1 THEN ; -1 T ."), "1 ");
    assert_eq!(run(r#": T C" abc" ; T COUNT TYPE T C@ ."#), "abc3 ");
    assert_eq!(err(r#"C" abc""#).code, -14);
    assert_eq!(err("5 COMPILE,").code, -14);
}

#[test]
fn escaped_strings() {
    let codes = |lit: &str| run(&format!(": T {lit} 0 ?DO DUP I + C@ . LOOP DROP ; T"));
    assert_eq!(codes(r#"S\" a\tb\"\\\x41\x4a\m""#), "97 9 98 34 92 65 74 13 10 ");
    assert_eq!(codes(r#"S\" \z\q\xF""#), "0 34 15 ");
    assert_eq!(run(r#"S\" x\"y" TYPE"#), "x\"y");
}

#[test]
fn parse_refill_and_input_state() {
    assert_eq!(run("CHAR | PARSE abc| TYPE PARSE-NAME   word TYPE"), "abcword");
    assert_eq!(run(": T PARSE-NAME NIP . ; T"), "0 ");
    assert_eq!(run(r#"SOURCE-ID . S" SOURCE-ID ." EVALUATE S" REFILL ." EVALUATE"#), "0 -1 0 ");
    // REFILL in a file moves to the next line; RESTORE-INPUT can go back.
    // REFILL continues in the next line; the rest of the current one is skipped.
    let file = temp_file("refill.fs", "SOURCE-ID 0> . REFILL 99 .\n. 2 .\n");
    assert_eq!(run(&format!("INCLUDE {file}")), "-1 -1 2 ");
    let lines =
        "VARIABLE N  : AGAIN? N @ 3 < IF 2OVER 2OVER RESTORE-INPUT . ELSE 2DROP 2DROP THEN ;\n\
                 SAVE-INPUT\n\
                 1 N +! AGAIN?\n\
                 N @ .\n";
    let file = temp_file("restore.fs", lines);
    assert_eq!(run(&format!("INCLUDE {file}")), "0 0 3 ");
    // Another source cannot be restored: the flag is true.
    assert_eq!(run(r#"SAVE-INPUT S" RESTORE-INPUT ." EVALUATE"#), "-1 ");
    assert_eq!(run_with_input("REFILL", "1 2 + . SOURCE TYPE\n"), "3 1 2 + . SOURCE TYPE");
}

#[test]
fn case_of_endof_endcase() {
    let src = ": T CASE 1 OF 10 ENDOF 2 OF 20 ENDOF DUP 100 * SWAP ENDCASE ; \
               1 T . 2 T . 3 T . : E CASE ENDCASE ; 5 E DEPTH .";
    assert_eq!(run(src), "10 20 300 0 ");
    let nested = ": N CASE 1 OF CASE 2 OF 12 ENDOF 0 SWAP ENDCASE ENDOF 9 SWAP ENDCASE ; \
                  2 1 N . 3 1 N . 1 7 N . DEPTH .";
    assert_eq!(run(nested), "12 0 9 1 ");
    assert_eq!(err(": BAD CASE 1 OF ENDCASE ;").code, -22);
    assert_eq!(err(": BAD IF ENDCASE ;").code, -22);
    assert_eq!(err(": BAD CASE 1 OF THEN ;").code, -22);
    assert_eq!(run("2 CELLS BUFFER: B 7 B CELL+ ! B CELL+ @ . B 8 MOD ."), "7 0 ");
}

#[test]
fn conditional_interpretation() {
    assert_eq!(run("-1 [IF] 1 [ELSE] 2 [THEN] . 0 [IF] 1 [ELSE] 2 [THEN] ."), "1 2 ");
    assert_eq!(run("0 [IF] 1 [if] 2 [ELSE] 3 [THEN] 4 [ELSE] 5 [THEN] ."), "5 ");
    assert_eq!(run("[ELSE] 1 . [THEN] 2 . [THEN] 3 ."), "2 3 ");
    assert_eq!(run(": T [ 0 ] [IF] 1 [ELSE] 2 [THEN] ; T ."), "2 ");
    // skipping goes on across lines read from the terminal
    assert_eq!(run_with_input("0 [IF] 1 .", "2 .\n[ELSE] 4 . [THEN] 5 .\n"), "4 5 ");
    assert_eq!(run("[DEFINED] DUP . [UNDEFINED] DUP . [DEFINED] NOSUCHWORD ."), "-1 0 0 ");
}

#[test]
fn string_words() {
    assert_eq!(
        run(r#"S" abc" S" abd" COMPARE . S" ab" S" ab" COMPARE . S" b" S" ab" COMPARE ."#),
        "-1 0 1 "
    );
    assert_eq!(run(r#"S" hello world" S" wor" SEARCH . TYPE"#), "-1 world");
    assert_eq!(run(r#"S" hello" S" xyz" SEARCH . TYPE"#), "0 hello");
    assert_eq!(run(r#"S" ab  " -TRAILING TYPE ." |" S" abcde" 2 /STRING TYPE"#), "ab|cde");
    // an overlapping CMOVE repeats characters, CMOVE> and MOVE do not
    let src = r#"S" abcd" PAD SWAP MOVE PAD PAD 1+ 3 CMOVE PAD 4 TYPE ."  "
                 S" abcd" PAD SWAP MOVE PAD PAD 1+ 3 CMOVE> PAD 4 TYPE"#;
    assert_eq!(run(src), "aaaa aabc");
    assert_eq!(run(r#": T [ S" lit" ] SLITERAL ; T TYPE"#), "lit");
    let src = r#"S" Bob" S" who" REPLACES S" hi %WHO%, 100%% %x%" PAD 40 SUBSTITUTE . TYPE"#;
    assert_eq!(run(src), "1 hi Bob, 100% %x%");
    assert_eq!(
        run(r#"S" a%b" PAD UNESCAPE TYPE S" toolong" PAD 3 SUBSTITUTE . . DROP"#),
        "a%%b-78 0 "
    );
    assert_eq!(err(r#"S" x" S" a%b" REPLACES"#).code, -79);
    assert_eq!(run("1 2 2CONSTANT C 2VARIABLE V C V 2! V 2@ . ."), "2 1 ");
}

#[test]
fn structures() {
    let src = "BEGIN-STRUCTURE POINT FIELD: P.X FIELD: P.Y CFIELD: P.TAG END-STRUCTURE
               POINT . 0 P.Y . CREATE P POINT ALLOT 3 P P.X ! 4 P P.Y ! P P.X @ P P.Y @ + .";
    assert_eq!(run(src), "17 8 7 ");
    assert_eq!(run("BEGIN-STRUCTURE S 1 +FIELD A FIELD: B END-STRUCTURE 0 B . S ."), "8 16 ");
}

#[test]
fn facility_words() {
    assert_eq!(run("5 2 AT-XY PAGE"), "\x1b[3;6H\x1b[2J\x1b[H");
    assert_eq!(run("0 MS 65 EKEY>CHAR . . K-UP EKEY>FKEY . K-UP = . EMIT? ."), "-1 65 -1 -1 -1 ");
    assert_eq!(run_with_input("KEY? . KEY . KEY? .", "A"), "-1 65 0 ");
    let date = run("TIME&DATE . . . . . .");
    let v: Vec<i64> = date.split_whitespace().map(|s| s.parse().unwrap()).collect();
    assert!(v[0] >= 2024 && (1..=12).contains(&v[1]) && (1..=31).contains(&v[2]), "{date}");
    assert!(v[3] < 24 && v[4] < 60 && v[5] < 60, "{date}");
}

#[test]
fn double_numbers() {
    assert_eq!(run("1. . . -2. . . : T 3. ; T D. HEX FF. DECIMAL D."), "0 1 -1 -2 3 255 ");
    assert_eq!(run("1. 2. D+ D. 5. 7. D- D. -3. DABS D. 1. DNEGATE D."), "3 -2 3 -1 ");
    assert_eq!(run("-1 -1 D2/ D. 1. D2* D. 12. 6 D.R"), "-1 2     12");
    assert_eq!(run("1. 2. D< . 1. 1. D= . -1. 1. DU< . -1. D0< . 0. D0= ."), "-1 -1 0 -1 -1 ");
    assert_eq!(run("1. 2. DMAX D. 1. 2. DMIN D. 7. D>S . 5. 3 M+ D."), "2 1 7 8 ");
    assert_eq!(run("5. -7 11 M*/ D. -1. 1 2 M*/ D."), "-4 -1 ");
    assert_eq!(err("1. 1 0 M*/").code, -10);
    let two_value = "1 2 2VALUE V V . . 3 4 TO V V . . : T 5 6 TO V ; T V . .";
    assert_eq!(run(two_value), "2 1 4 3 6 5 ");
    assert_eq!(
        run("1 2 3 4 5 6 2ROT . . . . . . : T [ 1 2 ] 2LITERAL ; T . ."),
        "2 1 6 5 4 3 2 1 "
    );
}

#[test]
fn programming_tools() {
    assert_eq!(run(": T AHEAD 1 THEN 2 ; T . DEPTH ."), "2 0 ");
    // WHILE as IF 1 CS-ROLL
    let src = ": W POSTPONE IF 1 CS-ROLL ; IMMEDIATE \
               : T 0 BEGIN DUP 3 < W 1+ REPEAT ; T .";
    assert_eq!(run(src), "3 ");
    assert_eq!(err(": T [ 5 CS-PICK ] ;").code, -22);
    assert_eq!(run(": T N>R DEPTH NR> ; 7 8 9 2 T . . . . ."), "2 9 8 1 7 ");
    assert_eq!(err("1 2 5 N>R").code, -4);
    // B is immediate too: compiling C runs it, leaving a 1
    assert_eq!(run(": A 1 ; IMMEDIATE SYNONYM B A : C B ; B . C DEPTH ."), "1 1 ");
    assert_eq!(run("VARIABLE V 42 V ! V ?"), "42 ");
    let dump = run("HERE 65 C, 0 C, HERE OVER - DUMP");
    assert_eq!(dump[8..], format!("  41 00{}  A.\n", " ".repeat(42)));
}

#[test]
fn values_defers_and_markers() {
    assert_eq!(run("5 VALUE V V . 7 TO V V . : T 9 TO V ; T V ."), "5 7 9 ");
    let defer = "DEFER D ' 1+ IS D 5 D . : T ['] 2* IS D ; T 5 D . ACTION-OF D ' 2* = .";
    assert_eq!(run(defer), "6 10 -1 ");
    assert_eq!(run("DEFER D ' NEGATE ' D DEFER! 3 D . ' D DEFER@ ' NEGATE = ."), "-3 -1 ");
    assert_eq!(run("DEFER D : A ACTION-OF D ; ' DUP IS D A ' DUP = ."), "-1 ");
    assert_eq!(err("DEFER D D").code, -21);
    assert_eq!(err("DEFER D ' D IS D D").code, -5);
    assert_eq!(err("5 CONSTANT C 6 TO C").code, -32);
    assert_eq!(err("0 VALUE V ' V DEFER@").code, -32);
    assert_eq!(run("MARKER M : A 1 ; MARKER N : A 2 ; A . N A . M"), "2 1 ");
    assert_eq!(err("MARKER M : A 1 ; M A").code, -13);
    assert_eq!(err("MARKER M M M").code, -13);
    assert_eq!(run("HERE MARKER M 100 ALLOT M HERE = ."), "-1 ");
    assert_eq!(run("1 VALUE X MARKER M 2 TO X M X ."), "2 ");
}

#[test]
fn catch_and_throw() {
    assert_eq!(run(": T 9 ; 1 2 ' T CATCH .S"), "<4> 1 2 9 0 ");
    assert_eq!(run(": T 7 8 99 THROW ; 1 2 ' T CATCH .S"), "<3> 1 2 99 ");
    assert_eq!(run(": T 8 0 THROW ; ' T CATCH .S"), "<2> 8 0 ");
    // The depth is restored even when the stack was emptied, and the
    // return stack, loops and input sources are unwound.
    assert_eq!(run(": T 2DROP 2DROP 5 THROW ; 1 2 3 4 ' T CATCH . DEPTH ."), "5 4 ");
    let unwind = ": T 3 0 DO 1 >R I 1 = IF S\" 6 THROW\" EVALUATE THEN LOOP ; \
                  ' T CATCH . 7 .";
    assert_eq!(run(unwind), "6 7 ");
    assert_eq!(run(r#": T 1 ABORT" not shown" ; ' T CATCH ."#), "-2 ");
    assert_eq!(run(r#"S" 1 2 NO-SUCH-WORD" ' EVALUATE CATCH . DEPTH ."#), "-13 2 ");
    // A definition started inside CATCH is dropped, and interpreting resumes.
    let mut f = Forth::new();
    f.interpret(r#": T S" : HALF 1 NO-SUCH-WORD ;" EVALUATE ; ' T CATCH . 5 ."#).unwrap();
    assert!(!f.compiling() && f.find("HALF").is_none());
    assert_eq!(output(f), "-13 5 ");
    assert_eq!(err("' QUIT CATCH").code, QUIT);
    assert_eq!(err("3 THROW").code, 3);
    assert_eq!(run("-1 ' EXECUTE CATCH ."), "-24 ");
}

#[test]
fn catch_nesting_is_limited() {
    let deep = "DEFER R' : R ?DUP IF 1- ['] R' CATCH THROW THEN ; ' R IS R' ";
    let big_stack = std::thread::Builder::new().stack_size(64 << 20);
    let codes = big_stack
        .spawn(move || {
            let ok = Forth::new().interpret(&format!("{deep} {MAX_CATCHES} R")).is_ok();
            let e = Forth::new().interpret(&format!("{deep} {} R", MAX_CATCHES + 1));
            (ok, e.unwrap_err().code)
        })
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(codes, (true, -5));
}

#[test]
fn stack_limits() {
    assert_eq!(err(": T BEGIN 1 AGAIN ; T").code, -3);
    assert_eq!(err(": T BEGIN 1 >R AGAIN ; T").code, -5);
}

#[test]
fn environment_queries() {
    let q = |s: &str| run(&format!(r#"S" {s}" ENVIRONMENT? .S"#));
    assert_eq!(q("ADDRESS-UNIT-BITS"), "<2> 8 -1 ");
    assert_eq!(q("floored"), "<2> -1 -1 ");
    assert_eq!(q("/HOLD"), "<2> 256 -1 ");
    assert_eq!(q("MAX-N"), "<2> 9223372036854775807 -1 ");
    assert_eq!(q("MAX-U"), "<2> -1 -1 ");
    assert_eq!(q("MAX-D"), "<3> -1 9223372036854775807 -1 ");
    assert_eq!(q("MAX-UD"), "<3> -1 -1 -1 ");
    assert_eq!(q("STACK-CELLS"), "<2> 65536 -1 ");
    assert_eq!(q("NO-SUCH-THING"), "<1> 0 ");
}

#[test]
fn evaluate_is_a_nested_source() {
    assert_eq!(run(r#"S" 1 2 + ." EVALUATE 4 ."#), "3 4 ");
    assert_eq!(run(r#": E S" 6 7 *" EVALUATE ; E ."#), "42 ");
    assert_eq!(run(r#"S" : SQ DUP * ;" EVALUATE 5 SQ ."#), "25 ");
    assert_eq!(run(r#"S" CHAR A EMIT SOURCE TYPE" EVALUATE"#), "ACHAR A EMIT SOURCE TYPE");
    let nested = r#": INNER S" 2 3 +" EVALUATE ; S" INNER 10 *" EVALUATE ."#;
    assert_eq!(run(nested), "50 ");
    assert_eq!(run(r#": F S" 9 DUP" ; : G F EVALUATE ; IMMEDIATE : H G + ; H ."#), "18 ");
    assert_eq!(err(r#"S" 1 FOO" EVALUATE"#).code, -13);
}

#[test]
fn include_reports_error_location() {
    let path = temp_file("bad.fs", "1 2\n: OK ;\n3 FOO\n");
    let e = err(&format!("INCLUDE {path}"));
    assert_eq!(e.code, -13);
    assert!(e.to_string().ends_with(":3: error -13: undefined word: FOO"), "{e}");
    assert_eq!(err("INCLUDE /no/such/file.fs").code, -38);
}

#[test]
fn file_access() {
    let path = temp_file("io.txt", "");
    let src = format!(
        r#"VARIABLE FID  CREATE BUF 20 ALLOT
        S" {path}" W/O CREATE-FILE . FID !
        S" ab" FID @ WRITE-LINE .  S" cdef" FID @ WRITE-FILE .  FID @ CLOSE-FILE .
        S" {path}" R/O BIN OPEN-FILE . FID !  FID @ FILE-SIZE . . .
        BUF 20 FID @ READ-LINE . . .  BUF 2 TYPE
        BUF 20 FID @ READ-LINE . . .  BUF 4 TYPE
        BUF 20 FID @ READ-LINE . . .
        FID @ CLOSE-FILE .  FID @ CLOSE-FILE .
        S" {path}" DELETE-FILE .  S" {path}" R/O OPEN-FILE . ."#
    );
    assert_eq!(run(&src), "0 0 0 0 0 0 0 7 0 -1 2 ab0 -1 4 cdef0 0 0 0 -62 0 -69 0 ");
}

#[test]
fn included_and_required() {
    let inc = temp_file("inc.fs", "( a comment\n over two lines ) 1+\nSOURCE-ID 0> .\n");
    let src = format!(
        r#"0 S" {inc}" INCLUDED  S" {inc}" REQUIRED  REQUIRE {inc}
        S" {inc}" R/O OPEN-FILE DROP INCLUDE-FILE ."#
    );
    assert_eq!(run(&src), "-1 -1 2 ");
    // At the terminal a comment ends with the line.
    assert_eq!(run("( no end\n1 ."), "1 ");
}

#[test]
fn byte_order_mark_is_ignored() {
    assert_eq!(run("\u{feff}1 2 + ."), "3 ");
}

#[test]
fn metaprogramming() {
    assert_eq!(run(": SQ DUP * ; 4 ' SQ EXECUTE ."), "16 ");
    assert_eq!(run(": ENDIF POSTPONE THEN ; IMMEDIATE : T 1 IF 2 . ENDIF ; T"), "2 ");
    assert_eq!(run(": T [ 6 7 * ] LITERAL ; T ."), "42 ");
    assert_eq!(run(": A 1 ; : A A 1 + ; A ."), "2 ");
}

#[test]
fn errors_are_throw_codes_and_reset_state() {
    let mut f = Forth::new();
    let e = f.interpret("1 2 FOO").unwrap_err();
    assert_eq!(e.code, -13);
    assert_eq!(e.to_string(), "error -13: undefined word: FOO");
    assert!(f.stack.is_empty());
    assert_eq!(f.interpret("DROP").unwrap_err().code, -4);
    assert_eq!(f.interpret(": BAD IF ;").unwrap_err().code, -22);
    assert!(f.find("BAD").is_none());
    assert!(!f.compiling());
    assert_eq!(f.interpret("1 0 /").unwrap_err().code, -10);
    assert_eq!(f.interpret("IF").unwrap_err().code, -14);
    assert_eq!(f.interpret("R>").unwrap_err().code, -6);
    assert_eq!(f.interpret("I").unwrap_err().code, -26);
    assert_eq!(f.interpret(":").unwrap_err().code, -16);
    f.interpret("1 2 + .").unwrap();
    assert_eq!(output(f), "3 ");
}

#[test]
fn heap_shares_free_space_with_data_space() {
    assert_eq!(run("UNUSED 100 ALLOCATE . DROP UNUSED - ."), "0 104 ");
    assert_eq!(run("100 ALLOCATE DROP DUP FREE . FREE ."), "0 -60 ");
    assert_eq!(run("1 ALLOCATE DROP 7 OVER ! 1000 RESIZE . @ ."), "0 7 ");
    assert_eq!(run("-1 ALLOCATE . .  5 -1 RESIZE . ."), "-59 0 -61 5 ");
    // a full data space leaves no room for the heap, and the other way round
    assert_eq!(run("UNUSED ALLOT 8 ALLOCATE . DROP"), "-59 ");
    assert_eq!(err("ALIGN UNUSED 8 - ALLOCATE 2DROP 16 ALLOT").code, -8);
}
