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
    // a shift by a cell or more shifts every bit out
    assert_eq!(run("1 63 LSHIFT 0< . 1 64 LSHIFT . -1 64 RSHIFT . -1 63 RSHIFT ."), "-1 0 0 1 ");
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
fn cell_and_index_conversions_saturate() {
    assert_eq!(to_cell(0), 0);
    assert_eq!(to_cell(1 << 40), 1 << 40);
    assert_eq!(to_cell(usize::MAX), Cell::MAX);
    assert_eq!(to_index(0), 0);
    assert_eq!(to_index(Cell::MAX), usize::try_from(Cell::MAX).unwrap_or(usize::MAX));
    // a negative cell is never a valid index: it stays outside every memory
    assert_eq!(to_index(-1), usize::MAX);
    assert_eq!(to_index(Cell::MIN), usize::MAX);
}

#[test]
fn extreme_cells_as_addresses_and_lengths_are_refused() {
    let max = Cell::MAX;
    assert_eq!(err(&format!("{max} @")).code, -9);
    assert_eq!(err(&format!("{} @", Cell::MIN)).code, -9);
    assert_eq!(err(&format!("0 {max} TYPE")).code, -9);
    assert_eq!(err(&format!("0 {max} 0 FILL")).code, -9);
    assert_eq!(err(&format!("{max} 1 TYPE")).code, -9);
    assert_eq!(err(&format!("0 8 {max} MOVE")).code, -9);
    // a negative length counts as zero where the standard says so
    assert_eq!(run(&format!("0 {} TYPE", Cell::MIN)), "");
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
    assert_eq!(f.stack, [] as [i64; 0]);
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
fn quotations() {
    assert_eq!(run(": T [: 6 7 * ;] EXECUTE ; T ."), "42 ");
    assert_eq!(run(": T [: DUP * ;] ; 7 T EXECUTE ."), "49 ");
    // control flow inside the quotation, and in the outer definition around it
    assert_eq!(run(": T 3 0 DO [: 1 5 0 DO I + LOOP ;] EXECUTE . LOOP ; T"), "11 11 11 ");
    // nested
    assert_eq!(run(": T [: [: 10 + ;] EXECUTE ;] ; 5 T EXECUTE ."), "15 ");
    // a quotation with one primitive is that primitive
    assert_eq!(run(": T [: + ;] ; T ' + = ."), "-1 ");
    assert_eq!(run(": T [: + DUP ;] ; T ' + = ."), "0 ");
    // it does not use up the outer definition's space more than needed
    assert_eq!(run(": T [: + ;] DROP ; T"), "");
    // the outer definition is intact after the quotation
    assert_eq!(run(": T 1 [: 2 ;] EXECUTE + [: 3 ;] EXECUTE + ; T ."), "6 ");
    // RECURSE inside a quotation calls the quotation
    assert_eq!(run(": T [: DUP IF 1- RECURSE THEN ;] EXECUTE ; 5 T ."), "0 ");
    // errors
    // outside a definition it leaves the xt, as :NONAME does
    assert_eq!(run("[: 6 7 * ;] EXECUTE ."), "42 ");
    assert_eq!(run("3 [: 2 * ;] EXECUTE [: 1+ ;] EXECUTE ."), "7 ");
    assert_eq!(run("[: + ;] ' + = ."), "-1 ");
    assert_eq!(run("[: [: 5 ;] EXECUTE 1+ ;] EXECUTE ."), "6 ");
    assert_eq!(err(";]").code, -22);
    assert_eq!(err("[: 1 ; ").code, -22);
    assert_eq!(err(": T ;] ;").code, -22);
    assert_eq!(err(": T [: 1 ; ;").code, -22);
    assert_eq!(err(": T [: IF ;] ;").code, -22);
    assert_eq!(err(": T [: 1 ;").code, -22);
    // an error inside a quotation drops it and the definition around it
    let mut f = Forth::new();
    assert!(f.interpret(": T [: 1 BOGUS ;] ;").is_err());
    assert_eq!(f.interpret(": U 5 ; U .").map(|()| f.output().to_vec()).unwrap(), b"5 ");
    // CATCH puts the compiler back
    let src = r#": F S" : Y [: 1 BOGUS ;] ;" EVALUATE ; ' F CATCH . : U 5 ; U ."#;
    assert_eq!(run(src), "-13 5 ");
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
    assert_eq!(err("' IF EXECUTE").code, -14);
    assert_eq!(err("LOOP").code, -14);
    assert_eq!(run(": T 3 0 DO I . LOOP ; T"), "0 1 2 ");
    assert_eq!(run("SEE THEN"), "THEN is a compile-only primitive IMMEDIATE\n");
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
    let lines = "VARIABLE N  : AGAIN? N @ 3 < IF 2OVER 2OVER RESTORE-INPUT . ELSE 2DROP 2DROP THEN ;\n\
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
    // UNESCAPE stops at the end of its buffer: PAD, an ALLOCATEd block
    assert_eq!(err(r#"S" %%" PAD 253 + UNESCAPE"#).code, -9);
    assert_eq!(run(r#"S" %%" PAD 252 + UNESCAPE . DROP"#), "4 ");
    assert_eq!(err(r#"8 ALLOCATE DROP S" %%%%%" ROT UNESCAPE"#).code, -9);
    assert_eq!(run(r#"S" a%" HERE UNESCAPE TYPE"#), "a%%");
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
fn xkey_reads_whole_xchars() {
    let src = "XKEY? . XKEY . XKEY . EKEY . XKEY . KEY . XKEY? .";
    assert_eq!(run_with_input(src, "é€😀A\n"), "-1 233 8364 128512 65 10 0 ");
    // A byte that does not start a complete xchar comes on its own.
    let mut f = Forth::new();
    f.input = Box::new(io::Cursor::new(b"\xC3A\xA9".to_vec()));
    f.interpret("XKEY . XKEY . XKEY .").unwrap();
    assert_eq!(output(f), "195 65 169 ");
    assert_eq!(run("'é' . '€' ."), "233 8364 ");
    assert_eq!(err("'ab'").code, -13);
}

#[test]
fn floats() {
    // literals need an E and a decimal BASE; 1.5 is not a float, 15. a double
    assert_eq!(run("1.5E0 2E F* F. -1.25e-1 F. 1E3 FDEPTH ."), "3. -0.125 1 ");
    assert_eq!(err("1.5").code, -13);
    assert_eq!(err("HEX 1.5E0").code, -13);
    assert_eq!(run("HEX 1E DECIMAL . FDEPTH ."), "30 0 ");
    assert_eq!(run(": X 2.5E0 FDUP F* ; X F. 7 S>F 2E F/ F>S ."), "6.25 3 ");
    assert_eq!(run("1E 0E F/ F. -1E 0E F/ F. 0E 0E F/ F. -0E F."), "inf -inf nan -0. ");
    // FVALUE and TO, interpreted and compiled
    let src = "1.5E0 FVALUE V V F. 2E TO V V F. : SET TO V ; 3E SET V F.";
    assert_eq!(run(src), "1.5 2. 3. ");
    let src = "BEGIN-STRUCTURE P CFIELD: P.TAG FFIELD: P.X SFFIELD: P.Y END-STRUCTURE
               P . 0 P.X . 0 P.Y . CREATE Q P ALLOT 0.5E0 Q P.Y SF! Q P.Y SF@ F.";
    assert_eq!(run(src), "20 8 16 0.5 ");
    assert_eq!(
        run("3 SET-PRECISION PRECISION . 2E 3E F/ FDUP F. FS. 0 SET-PRECISION PRECISION ."),
        "3 0.667 6.67E-1 1 "
    );
    assert_eq!(run("1E FSINCOS 1E FCOS 0E F~ . 1E FSIN 0E F~ ."), "-1 -1 ");
    // the float stack has its own errors, and CATCH restores its depth
    assert_eq!(err("FDROP").code, -45);
    assert_eq!(err(": X 0 BEGIN 1E 1+ AGAIN ; X").code, -44);
    assert_eq!(run("1E ' FDROP CATCH . ' F@ CATCH . FDEPTH ."), "0 -4 0 ");
    assert_eq!(err("1E50 F>D").code, -11);
    let out = run("2.5E0 FVALUE W : Y 1.5E0 TO W ; SEE W SEE Y");
    assert!(out.contains("2.5E0 FVALUE W") && out.contains("1.5E0\n"), "{out}");
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
fn errors_keep_their_context() {
    let context = |src: &str| err(src).context().cloned().unwrap();
    let c = context(": STEP 0 / ;\n: MAIN 1 2 STEP ;\n5   MAIN 7");
    assert_eq!(c.words, ["MAIN", "STEP", "/"]);
    // `/` has taken its two arguments; 5 1 are left.
    assert_eq!(c.stack, [5, 1]);
    assert_eq!((c.source.as_str(), c.line, c.column, c.token.as_str()), ("input", 3, 5, "MAIN"));
    // An underflow takes nothing, so the stack is as it was.
    let c = context("1.5e0 4 +");
    assert_eq!((c.stack, c.fstack, c.words), (vec![4], vec![1.5], vec!["+".to_string()]));
    // An undefined word runs nothing, also while compiling.
    let c = context("1 2\n  : X  DUP FOO ;");
    assert_eq!((c.line, c.column, c.token.as_str()), (2, 12, "FOO"));
    assert!(c.words.is_empty() && c.stack == [1, 2]);
    // The column counts characters, not bytes.
    assert_eq!(context("S\" é\" 2DROP FOO").column, 13);
    // A line too long reads nothing.
    let c = context(&"1 ".repeat(1000));
    assert_eq!((c.line, c.column, c.token.as_str()), (1, 0, ""));
    // EVALUATE: the position is in its string; the words go on from the caller.
    let c = context(": RUN S\" 1 2\" EVALUATE S\" 3 0 MOD\" EVALUATE ;\nRUN");
    assert_eq!((c.source.as_str(), c.line, c.column, c.token.as_str()), ("evaluate", 1, 5, "MOD"));
    assert_eq!(c.words, ["RUN", "MOD"]);
    assert_eq!(context(":NONAME 0 0 / ; EXECUTE").words, [":NONAME", "/"]);
    // Lines count from the start of each call of interpret.
    let mut f = Forth::new();
    f.interpret("1\n2\n3 DROP DROP DROP").unwrap();
    let e = f.interpret("\n BOGUS").unwrap_err();
    assert_eq!(e.context().map(|c| (c.line, c.column)), Some((2, 2)));
    // An error caught by CATCH has none; one from a file has its path.
    assert!(f.interpret("' DROP CATCH").is_ok());
    let path = temp_file("context.fs", "1 2\n3 0 /\n");
    let c = context(&format!("INCLUDE {path}"));
    assert_eq!((c.source, c.line, c.column, c.stack), (path, 2, 5, vec![1, 2]));
}

#[test]
fn steps_used_and_max_depth() {
    let mut f = Forth::new();
    f.interpret(": F 1 2 3 DROP DROP DROP ; F 7").unwrap();
    assert_eq!(f.max_depth(), 3);
    let steps = f.steps_used();
    assert!(steps > 0, "{steps}");
    f.interpret("F F 7").unwrap();
    assert!(f.steps_used() > steps);
    f.set_limits(Limits { steps: Some(1000), ..Limits::default() });
    assert_eq!(f.interpret(": L BEGIN AGAIN ; L").unwrap_err().code, LIMIT_EXCEEDED);
    assert_eq!(f.steps_used(), 1000);
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
    assert_eq!(f.stack, [] as [i64; 0]);
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

#[test]
fn memory_size_is_configurable() {
    let mut small = Forth::with_memory(MIN_MEMORY);
    let mut large = Forth::with_memory(4 << 20);
    small.interpret("UNUSED . HERE .").unwrap();
    large.interpret("UNUSED . HERE .").unwrap();
    let (small, large) = (output(small), output(large));
    let unused = |s: &str| s.split(' ').next().unwrap().parse::<usize>().unwrap();
    assert_eq!(unused(&large) - unused(&small), (4 << 20) - MIN_MEMORY);
    assert_eq!(err("1 ALLOCATE DROP 2000000 ALLOT").code, -8);
}

#[test]
fn locals() {
    assert_eq!(run(": T {: A B | C -- D :} B A + TO C C A B ; 1 2 T .S"), "<3> 3 1 2 ");
    assert_eq!(run(": T LOCALS| A B | A B ; 1 2 T .S"), "<2> 2 1 ");
    // locals are gone after ; and a local can share a word's name
    assert_eq!(run(": T {: DUP :} DUP DUP ; 5 T .S"), "<2> 5 5 ");
    assert_eq!(err(": T {: X :} X ; X").code, -13);
    // each call has its own locals, also after EXIT, an error or a THROW
    let src = ": F {: N :} N 0= IF 1 EXIT THEN N 1- RECURSE N * ; 10 F .";
    assert_eq!(run(src), "3628800 ");
    let src = ": BAD {: X :} X THROW ; : T {: A :} 7 ['] BAD CATCH . A ; 9 T .";
    assert_eq!(run(src), "7 9 ");
    assert_eq!(run("S\" #LOCALS\" ENVIRONMENT? . ."), "-1 256 ");
    // {: reads on to the next line; here there is none
    let mut f = Forth::new();
    f.set_input(Box::new(io::Cursor::new(Vec::new())));
    assert_eq!(f.interpret(": T {: A").unwrap_err().code, -22);
    assert_eq!(err(": T {: A | B | C :} ;").code, -22);
}

#[test]
fn word_lists() {
    let src = "WORDLIST CONSTANT W  W SET-CURRENT : X 1 ; FORTH-WORDLIST SET-CURRENT : X 2 ;";
    assert_eq!(run(&format!("{src} X . GET-ORDER W SWAP 1+ SET-ORDER X . PREVIOUS X .")), "2 1 2 ");
    assert_eq!(run(&format!("{src} S\" X\" W SEARCH-WORDLIST . EXECUTE .")), "-1 1 ");
    assert_eq!(err(": P PREVIOUS PREVIOUS ; ONLY P").code, -50);
    assert_eq!(err("17 SET-ORDER").code, -49);
    assert_eq!(err("99 SET-CURRENT").code, -24);
    // an empty search order finds only the words that set a new one
    assert_eq!(err("0 SET-ORDER DUP").code, -13);
    assert_eq!(run("0 SET-ORDER ONLY 1 2 + ."), "3 ");
    assert_eq!(run("0 SET-ORDER FORTH-WORDLIST 1 SET-ORDER GET-ORDER . ."), "1 1 ");
    // a marker puts the search order and the compilation word list back
    let src = "MARKER M  WORDLIST DUP SET-CURRENT GET-ORDER 1+ SET-ORDER  : Y ;  M";
    assert_eq!(run(&format!("{src} GET-ORDER . . GET-CURRENT .")), "1 1 1 ");
    assert_eq!(err("WORDLIST SET-CURRENT : Z ; ONLY Z").code, -13);
}

#[test]
fn an_undefined_word_hints_at_its_word_list() {
    let hint = |src: &str| err(src).hint().map(str::to_string);
    let mut f = Forth::new();
    f.install(&Json);
    let e = f.interpret("JSON-LENGTH").unwrap_err();
    assert_eq!((e.code, e.detail()), (-13, Some("JSON-LENGTH")));
    let text = "it is in the word list JSON; ALSO JSON finds it";
    assert_eq!(e.hint(), Some(text));
    assert_eq!(e.to_string(), format!("error -13: undefined word: JSON-LENGTH ({text})"));
    // also for ', and in lower case
    assert_eq!(f.interpret("' json@").unwrap_err().hint(), Some(text));
    // no hint for a word that is nowhere, or in a word list without a name
    assert_eq!(hint("NOSUCHWORD"), None);
    assert_eq!(hint("WORDLIST SET-CURRENT : Z ; FORTH-WORDLIST SET-CURRENT Z"), None);
    // with an empty search order, ALSO cannot help
    let only = "it is in the word list FORTH; ONLY FORTH finds it";
    assert_eq!(hint("0 SET-ORDER DUP").as_deref(), Some(only));
}

#[test]
fn name_tokens() {
    let src = "WORDLIST CONSTANT W  W SET-CURRENT : A 1 ; : B 2 ; IMMEDIATE \
               FORTH-WORDLIST SET-CURRENT : SHOW NAME>STRING TYPE SPACE TRUE ;";
    assert_eq!(run(&format!("{src} ' SHOW W TRAVERSE-WORDLIST")), "B A ");
    // the xt may run a marker, which removes the words still to come, and
    // here the xt itself too: no panic, but -24 for the next call
    let gone = "MARKER M1 : A1 ; : TW DROP M1 TRUE ; ' TW FORTH-WORDLIST TRAVERSE-WORDLIST";
    assert_eq!(err(gone).code, -24);
    let src = format!("{src} S\" A\" W SEARCH-WORDLIST DROP");
    assert_eq!(run(&format!("{src} NAME>INTERPRET EXECUTE .")), "1 ");
    assert_eq!(run(&format!("{src} NAME>COMPILE ' COMPILE, = . DROP")), "-1 ");
}

#[test]
fn forget() {
    let src = ": A 1 ; HERE : B 2 ; CREATE C 10 ALLOT : A 3 ;";
    assert_eq!(run(&format!("{src} FORGET B HERE = . A .")), "-1 1 ");
    assert_eq!(err(&format!("{src} FORGET B C")).code, -13);
    // FORGET looks in the compilation word list only
    let src = "WORDLIST CONSTANT W  GET-ORDER W SWAP 1+ SET-ORDER  : D 4 ;";
    assert_eq!(err(&format!("{src} W SET-CURRENT FORGET D")).code, -13);
    assert_eq!(err("FORGET DUP").code, -15);
}

/// Runs src with a block file of its own in the temp directory.
fn run_blocks(name: &str, src: &str) -> Result<String, Throw> {
    let path = env::temp_dir().join(format!("forthling-{}-{name}.fb", std::process::id()));
    let mut f = Forth::new();
    f.set_block_file(&path).unwrap();
    let result = f.interpret(src);
    fs::remove_file(&path).ok();
    result.map(|()| output(f))
}

#[test]
fn blocks() {
    let write = ": W ( blk addr u -- ) ROT BUFFER DUP 1024 BL FILL SWAP MOVE UPDATE ; ";
    let src = format!("{write} 3 S\" 1 2 + BLK @\" W FLUSH 3 LOAD . . BLK @ .");
    assert_eq!(run_blocks("load", &src).unwrap(), "3 3 0 ");
    // \ skips to the end of a 64-character line; REFILL moves to the next block
    let src = format!(r#"{write} 1 S" 7 \ 8" W 1 BLOCK 64 + 1 CHARS 57 FILL UPDATE 1 LOAD .S"#);
    assert_eq!(run_blocks("comment", &src).unwrap(), "<2> 7 9 ");
    let src = format!("{write} 1 S\" 1 REFILL 2\" W 2 S\" 3\" W 1 LOAD .S");
    assert_eq!(run_blocks("refill", &src).unwrap(), "<3> 1 -1 3 ");
    // a block beyond the end of the file is blank; EMPTY-BUFFERS drops changes
    let src = "5 BLOCK C@ . 5 BLOCK 65 SWAP C! UPDATE EMPTY-BUFFERS 5 BLOCK C@ .";
    assert_eq!(run_blocks("empty", src).unwrap(), "32 32 ");
    let out =
        run_blocks("list", ": T 2 BLOCK 1024 BL FILL S\" hi\" 2 BLOCK SWAP MOVE ; T 2 LIST SCR ?");
    assert!(out.as_ref().unwrap().starts_with("\nScreen 2\n 0 hi\n 1\n"), "{out:?}");
    assert!(out.unwrap().ends_with("15\n2 "));
    assert_eq!(run_blocks("zero", "0 BLOCK").unwrap_err().code, -35);
    assert_eq!(run_blocks("thru", "1 BUFFER 1024 BL FILL UPDATE 1 1 THRU").unwrap(), "");
}

#[test]
fn gforth_loops() {
    // +DO and U+DO skip a loop whose start is not below the limit
    assert_eq!(run(": T 10 0 +DO I . LOOP ; T"), "0 1 2 3 4 5 6 7 8 9 ");
    assert_eq!(run(": T 0 10 +DO I . LOOP ; T .S"), "<0> ");
    assert_eq!(run(": T 4 1 +DO I . 2 +LOOP ; T"), "1 3 ");
    assert_eq!(run(": T -1 1 +DO I . LOOP ; T .S"), "<0> ");
    assert_eq!(run(": T 1 -1 U+DO I . LOOP ; T .S"), "<0> ");
    // -DO counts down and leaves the limit out; the manual's examples
    assert_eq!(run(": T -2 0 -DO I . 1 -LOOP ; T"), "0 -1 ");
    assert_eq!(run(": T -1 0 -DO I . 1 -LOOP ; T"), "0 ");
    assert_eq!(run(": T 0 0 -DO I . 1 -LOOP ; T .S"), "<0> ");
    assert_eq!(run(": T 0 -1 -DO I . 1 -LOOP ; T .S"), "<0> ");
    assert_eq!(run(": T 0 6 U-DO I . 2 -LOOP ; T"), "6 4 2 ");
    assert_eq!(run(": T 6 0 U-DO I . 2 -LOOP ; T .S"), "<0> ");
    // FOR runs n+1 times, the index from n down to 0
    assert_eq!(run(": T 3 FOR I . NEXT ; T"), "3 2 1 0 ");
    assert_eq!(run(": T 0 FOR I . NEXT ; T"), "0 ");
    assert_eq!(run(": T -1 FOR I . NEXT ; T .S"), "<0> ");
    assert_eq!(run(": T 2 FOR 1 FOR I J . . NEXT NEXT ; T"), "2 1 2 0 1 1 1 0 0 1 0 0 ");
    // LEAVE, ?LEAVE and ?EXIT work in them
    assert_eq!(run(": T 9 FOR I . I 7 = ?LEAVE NEXT ; T"), "9 8 7 ");
    assert_eq!(run(": T 10 0 +DO I 3 = ?EXIT I . LOOP ; T"), "0 1 2 ");
    assert_eq!(run(": T 5 0 +DO I 3 = IF LEAVE THEN I . LOOP ; T"), "0 1 2 ");
    // they are compile-only, and a loop needs its own closing word
    assert_eq!(err("+DO").code, -14);
    assert_eq!(err("-LOOP").code, -14);
    assert_eq!(err(": T 1 0 +DO ;").code, -22);
    // LEAVE and UNLOOP leave a loop that ends with -LOOP
    assert_eq!(run(": T 0 9 -DO I . I 6 = IF LEAVE THEN 1 -LOOP ; T"), "9 8 7 6 ");
    assert_eq!(run(": T 0 9 -DO I . I 7 = IF UNLOOP EXIT THEN 1 -LOOP ; T"), "9 8 7 ");
    // the loop leaves nothing behind
    assert_eq!(run(": T 3 FOR NEXT 7 ; T .S"), "<1> 7 ");
    assert_eq!(run(": T 3 0 -DO I . 2 -LOOP ; T"), "");
}

#[test]
fn gforth_stack_and_numbers() {
    assert_eq!(run("1 2 3 4 2NIP .S"), "<2> 3 4 ");
    assert_eq!(run("1 2 3 4 2TUCK .S"), "<6> 3 4 1 2 3 4 ");
    assert_eq!(run("1 2 3 THIRD .S"), "<4> 1 2 3 1 ");
    assert_eq!(run("1 2 3 4 FOURTH .S"), "<5> 1 2 3 4 1 ");
    assert_eq!(run("5 1 2 UNDER+ .S"), "<2> 7 1 ");
    assert_eq!(run("1 2 <= . 2 2 <= . 3 2 <= . 1 2 >= . 2 2 >= ."), "-1 -1 0 0 -1 ");
    assert_eq!(run("0 0<= . -1 0<= . 1 0<= . 0 0>= . -1 0>= ."), "-1 -1 0 -1 0 ");
    assert_eq!(run("-1 1 U<= . 1 -1 U<= . 1 1 U>= . 1 -1 U>= ."), "0 -1 -1 0 ");
    assert_eq!(
        run("1. 1. D<> . 1. 2. D<> . 2. 1. D> . 1. 2. D> . 0. D0<> . 1. D0<> ."),
        "0 -1 -1 0 0 -1 "
    );
    assert_eq!(run("-1 3 UMAX . 3 -1 UMIN . -1 3 UMIN . 3 -1 UMAX ."), "-1 3 3 -1 ");
    assert_eq!(run("-7 2 U/MOD . . 7 2 UMOD ."), "9223372036854775804 1 1 ");
    // bits
    assert_eq!(run("$8000000000000000 1 ROL . 1 1 ROR 0< . 1 64 ROL . 1 0 ROR ."), "1 -1 1 1 ");
    assert_eq!(run("-16 2 ARSHIFT . -1 70 ARSHIFT . 16 70 ARSHIFT . 16 2 ARSHIFT ."), "-4 -1 0 4 ");
    assert_eq!(
        run("8 CTZ . 1 CTZ . 0 CTZ . 1000 LOG2 . 1 LOG2 . 0 LOG2 . -1 LOG2 ."),
        "3 0 64 9 0 -1 63 "
    );
    // return stack
    assert_eq!(run(": T 1 2 >R >R 0 RPICK . 1 RPICK . 2RDROP 3 >R RDROP ; T .S"), "1 2 <0> ");
    assert_eq!(err(": T RDROP ; T").code, -6);
    assert_eq!(err(": T 1 >R 2RDROP ; T").code, -6);
    assert_eq!(err(": T 1 >R 1 RPICK ; T").code, -6);
    // small helpers
    assert_eq!(run("VARIABLE V V ON V @ . V OFF V @ ."), "-1 0 ");
    assert_eq!(run("VARIABLE V ' 1+ V ! : T 1 V PERFORM ; T ."), "2 ");
    assert_eq!(run("NOOP 1 .S"), "<1> 1 ");
    assert_eq!(run(": T 1 IF 5 ENDIF . ; T"), "5 ");
    assert_eq!(run("1 \\G a comment\n2 .S"), "<2> 1 2 ");
    assert_eq!(run("UVALUE U U . 7 TO U U ."), "0 7 ");
    assert_eq!(run("' DUP ALIAS DUPE 5 DUPE .S"), "<2> 5 5 ");
    assert_eq!(run(": FOO 1 ; IMMEDIATE ' FOO ALIAS BAR : T BAR ; T ."), "1 ");
    assert_eq!(err("' IF ALIAS BAR 1 BAR").code, -14);
    assert_eq!(err("1000000 ALIAS BAR").code, -24);
    assert_eq!(
        run("VOCABULARY V ALSO V DEFINITIONS : X 42 . ; PREVIOUS DEFINITIONS ALSO V X PREVIOUS"),
        "42 "
    );
    // output
    assert_eq!(run("12345678901234. UD. 5. 4 UD.R"), "12345678901234    5");
    assert_eq!(
        run("HEX 255 DEC. BASE @ . DECIMAL 255 HEX. BASE @ . -1 HEX. "),
        "597 10 $FF 10 $FFFFFFFFFFFFFFFF "
    );
}

#[test]
fn gforth_strings() {
    let cmp = |op: &str, a: &str, b: &str| run(&format!("S\" {a}\" S\" {b}\" {op} ."));
    assert_eq!(cmp("STR=", "abc", "abd"), "0 ");
    assert_eq!(cmp("STR=", "abc", "abc"), "-1 ");
    assert_eq!(cmp("STR<", "abc", "abd"), "-1 ");
    assert_eq!(cmp("STR<", "b", "abd"), "0 ");
    assert_eq!(cmp("STRING-PREFIX?", "hello", "he"), "-1 ");
    assert_eq!(cmp("STRING-PREFIX?", "hello", "lo"), "0 ");
    assert_eq!(cmp("STRING-PREFIX?", "he", "hello"), "0 ");
    assert_eq!(cmp("STRING-PREFIX?", "hello", ""), "-1 ");
    assert_eq!(cmp("STRING-SUFFIX?", "hello", "lo"), "-1 ");
    assert_eq!(cmp("STRING-SUFFIX?", "hello", "he"), "0 ");
    assert_eq!(cmp("STRING-SUFFIX?", "lo", "hello"), "0 ");
    assert_eq!(cmp("STRING-SUFFIX?", "hello", ""), "-1 ");
    assert_eq!(run("S\" hello\" 2 SAFE/STRING TYPE"), "llo");
    assert_eq!(run("S\" hello\" 9 SAFE/STRING NIP ."), "0 ");
    assert_eq!(run("S\" hello\" -3 SAFE/STRING TYPE"), "hello");
    assert_eq!(
        run("97 TOUPPER . 65 TOUPPER . 122 TOUPPER . 123 TOUPPER . 48 TOUPPER ."),
        "65 65 90 123 48 "
    );
    assert_eq!(run("S\" a,b,c\" 44 SCAN TYPE"), ",b,c");
    assert_eq!(run("S\" abc\" 44 SCAN NIP ."), "0 ");
    assert_eq!(run("S\" ,,x\" 44 SKIP TYPE"), "x");
    assert_eq!(run("S\" ,,\" 44 SKIP NIP ."), "0 ");
    assert_eq!(run("S\" a,b,c\" 44 SCAN-BACK TYPE"), "a,b,");
    assert_eq!(run("S\" abc\" 44 SCAN-BACK NIP ."), "0 ");
    assert_eq!(run("S\" ,a\" 44 SCAN-BACK NIP ."), "1 ");
    assert_eq!(run("10 14 BOUNDS .S"), "<2> 24 10 ");
    assert_eq!(run("100 CELL- ."), "92 ");
    let buf = "CREATE B 20 ALLOT ";
    assert_eq!(run(&format!("{buf} S\" abcdef\" B PLACE B COUNT TYPE B C@ .")), "abcdef6 ");
    assert_eq!(run(&format!("{buf} S\" 123\" B SWAP MOVE S\" ab\" B 3 INSERT B 5 TYPE")), "ab123");
    let ab123 = format!("{buf} S\" ab123\" B SWAP MOVE ");
    assert_eq!(run(&format!("{ab123} B 5 2 DELETE B 5 TYPE")), "123  ");
    assert_eq!(run(&format!("{ab123} B 5 9 DELETE B 5 TYPE")), "     ");
    assert_eq!(run(&format!("{ab123} B 5 -1 DELETE B 5 TYPE")), "ab123");
    // S+ and APPEND use the heap
    assert_eq!(run("S\" abc\" S\" def\" S+ 2DUP TYPE SWAP FREE THROW ."), "abcdef6 ");
    assert_eq!(
        run("S\" abc\" S\" def\" S+ S\" XY\" APPEND 2DUP TYPE SWAP FREE THROW ."),
        "abcdefXY8 "
    );
    assert_eq!(run("0 0 0 0 S+ NIP . S\" x\" 0 0 S+ TYPE"), "0 x");
}

#[test]
fn gforth_memory_floats_and_conditionals() {
    let buf = "CREATE B 8 ALLOT ";
    assert_eq!(
        run(&format!("{buf} -1 B ! 0 B W! B @ . $1234 B W! B W@ . B C@ . B 1+ C@ .")),
        "-65536 4660 52 18 "
    );
    assert_eq!(
        run(&format!("{buf} -1 B ! $DEADBEEF B L! B L@ . B @ . B 4 + L@ .")),
        "3735928559 -559038737 4294967295 "
    );
    assert_eq!(run(&format!("{buf} -1 B ! -2 B W! B W@ .")), "65534 ");
    assert_eq!(err("L@").code, -4);
    assert_eq!(err("-1 L@").code, -9);
    assert_eq!(run("HERE $11223344 L, HERE SWAP - . HERE 1 W, HERE SWAP - ."), "4 2 ");
    assert_eq!(
        run(
            "1E 2E F= . 2E 2E F= . 2E 1E F> . 1E 2E F> . 2E 2E F>= . 1E 2E F>= . 0E F0<= . 1E F0<= . -1E F0<= ."
        ),
        "0 -1 -1 0 -1 0 -1 0 -1 "
    );
    // a NaN is not equal to, greater than or less than anything
    assert_eq!(
        run("0E 0E F/ FDUP F= . 1E 0E 0E F/ F> . 0E 0E F/ 0E F>= . 0E 0E F/ F0<= ."),
        "0 0 0 0 "
    );
    assert_eq!(run("1E 2E 3E FNIP F. F. FDEPTH ."), "3. 1. 0 ");
    assert_eq!(run("1E 2E FTUCK F. F. F. FDEPTH ."), "2. 1. 2. 0 ");
    assert_eq!(run("1E 2E 3E F-ROT F. F. F."), "2. 1. 3. ");
    assert_eq!(run("3E F2* F. 3E F2/ F. -3E F2/ F."), "6. 1.5 -1.5 ");
    assert_eq!(run("PI F. FDEPTH ."), "3.14159265358979 0 ");
    assert_eq!(run("1E 2E F.S FDROP FDROP F.S"), "<2> 1. 2. <0> ");
    // conditional compilation
    assert_eq!(run("[IFDEF] DUP 1 [ELSE] 2 [ENDIF] ."), "1 ");
    assert_eq!(run("[IFDEF] NOSUCH 1 [ELSE] 2 [ENDIF] ."), "2 ");
    assert_eq!(run("[IFUNDEF] NOSUCH 1 [ELSE] 2 [THEN] ."), "1 ");
    assert_eq!(run("[IFUNDEF] DUP 1 [ELSE] 2 [ENDIF] ."), "2 ");
    assert_eq!(run("[IFDEF] NOSUCH [IFDEF] DUP 1 [ELSE] 2 [ENDIF] 3 [ELSE] 4 [ENDIF] ."), "4 ");
    assert_eq!(run("0 [IF] [IFDEF] DUP 1 [ENDIF] [ELSE] 2 [THEN] ."), "2 ");
    assert_eq!(run("[IFDEF] NOSUCH\n 1\n[ELSE]\n 2\n[ENDIF] ."), "2 ");
    // time and random numbers
    assert_eq!(run("UTIME 1000000000000000. D< 0= ."), "-1 ");
    assert_eq!(run("CPUTIME 2DROP D0< ."), "0 ");
    assert_eq!(run("7 SEED! 100 RANDOM 7 SEED! 100 RANDOM = . 0 RANDOM . 1 RANDOM ."), "-1 0 0 ");
    assert_eq!(run("100 RANDOM 100 U< ."), "-1 ");
    assert_eq!(run("1 SEED! RND 1 SEED! RND = . RND RND = ."), "-1 0 ");
}

#[test]
fn gforth_slurp() {
    let slurp = |name: &str, content: &str, tail: &str| {
        let path = temp_file(name, content).replace('\\', "/");
        run(&format!("S\" {path}\" {tail}"))
    };
    assert_eq!(
        slurp("slurp.txt", "hello\nworld\n", "SLURP-FILE 2DUP TYPE SWAP FREE THROW ."),
        "hello\nworld\n12 "
    );
    assert_eq!(
        slurp(
            "slurp2.txt",
            "abc",
            "R/O OPEN-FILE THROW DUP >R SLURP-FID R> CLOSE-FILE THROW NIP ."
        ),
        "3 "
    );
    assert_eq!(slurp("empty.txt", "", "SLURP-FILE NIP ."), "0 ");
    assert_ne!(err("S\" nonexistent-file.txt\" SLURP-FILE").code, 0);
}
