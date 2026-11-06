//! The COLLECTIONS words, used from Forth: vectors, dictionaries and SORT-CELLS.

use forthling::{Cell, Collections, Forth, Functional, Json, Limits, Strings};
use std::io::Cursor;

fn forth() -> Forth {
    let mut f = Forth::new();
    f.set_input(Box::new(Cursor::new(Vec::new())));
    f.install(&Collections);
    f.interpret("ALSO COLLECTIONS").unwrap();
    f
}

fn run(f: &mut Forth, text: &str) -> String {
    let start = f.output().len();
    f.interpret(text).unwrap_or_else(|e| panic!("{text}: {e}"));
    String::from_utf8_lossy(&f.output()[start..]).into_owned()
}

/// Runs text that has loops in it: they need a definition.
fn def(f: &mut Forth, text: &str) -> String {
    run(f, &format!(": T {text} ; T"))
}

fn code(f: &mut Forth, text: &str) -> Cell {
    f.interpret(text).expect_err(text).code
}

#[test]
fn vector_push_get_set_pop() {
    let mut f = forth();
    run(&mut f, "VEC-VARIABLE V");
    assert_eq!(run(&mut f, "V VEC# ."), "0 ");
    assert!(run(&mut f, "V VEC-DATA . .").starts_with("0 "));
    // more than the first capacity, so that the block moves
    def(&mut f, "100 0 DO I I * V VEC+! LOOP");
    assert_eq!(run(&mut f, "V VEC# . 0 V VEC@ . 7 V VEC@ . 99 V VEC@ ."), "100 0 49 9801 ");
    run(&mut f, "-1 7 V VEC!");
    assert_eq!(run(&mut f, "7 V VEC@ ."), "-1 ");
    assert_eq!(run(&mut f, "V VEC-POP . V VEC-POP . V VEC#  ."), "9801 9604 98 ");
    run(&mut f, "V VEC-CLEAR");
    assert_eq!(run(&mut f, "V VEC# ."), "0 ");
    run(&mut f, "5 V VEC+!");
    assert_eq!(run(&mut f, "0 V VEC@ ."), "5 ");
    run(&mut f, "V VEC-FREE");
    assert_eq!(run(&mut f, "V @ . V VEC# ."), "0 0 ");
    // free again, and use again
    run(&mut f, "V VEC-FREE 3 V VEC+! 0 V VEC@ . V VEC-FREE");
}

#[test]
fn vector_errors() {
    let mut f = forth();
    run(&mut f, "VEC-VARIABLE V");
    assert_eq!(code(&mut f, "V VEC-POP"), -24);
    assert_eq!(code(&mut f, "0 V VEC@"), -24);
    run(&mut f, "1 V VEC+! 2 V VEC+!");
    assert_eq!(code(&mut f, "2 V VEC@"), -24);
    assert_eq!(code(&mut f, "-1 V VEC@"), -24);
    assert_eq!(code(&mut f, "9 2 V VEC!"), -24);
}

#[test]
fn vector_data_works_with_the_cell_words() {
    let mut f = Forth::new();
    f.set_input(Box::new(Cursor::new(Vec::new())));
    f.install(&Collections);
    f.install(&Functional);
    f.interpret("ALSO COLLECTIONS ALSO FUNCTIONAL VEC-VARIABLE V").unwrap();
    def(&mut f, "5 0 DO I 1+ V VEC+! LOOP");
    assert_eq!(run(&mut f, "V VEC-DATA SUM-CELLS ."), "15 ");
    run(&mut f, "V VEC-DATA ' 1+ MAP-CELLS");
    assert_eq!(run(&mut f, "3 V VEC@ ."), "5 ");
}

#[test]
fn dict_store_fetch_delete() {
    let mut f = forth();
    run(&mut f, "DICT-VARIABLE D");
    assert_eq!(run(&mut f, r#"D DICT# . S" a" D DICT@ ."#), "0 0 ");
    run(&mut f, r#"1 S" a" D DICT!  2 S" b" D DICT!  3 S" c" D DICT!"#);
    assert_eq!(run(&mut f, "D DICT# ."), "3 ");
    assert_eq!(run(&mut f, r#"S" a" D DICT@ . . S" c" D DICT@ . ."#), "-1 1 -1 3 ");
    assert_eq!(run(&mut f, r#"S" nope" D DICT@ ."#), "0 ");
    // a new value for an old key does not add a key
    run(&mut f, r#"20 S" b" D DICT!"#);
    assert_eq!(run(&mut f, r#"D DICT# . S" b" D DICT@ . ."#), "3 -1 20 ");
    // delete
    assert_eq!(run(&mut f, r#"S" b" D DICT-DELETE . S" b" D DICT-DELETE . D DICT# ."#), "-1 0 2 ");
    assert_eq!(run(&mut f, r#"S" b" D DICT@ . S" a" D DICT@ . ."#), "0 -1 1 ");
    run(&mut f, "D DICT-FREE");
    assert_eq!(run(&mut f, r#"D @ . D DICT# . S" a" D DICT@ . S" a" D DICT-DELETE ."#), "0 0 0 0 ");
    run(&mut f, r#"5 S" x" D DICT! D DICT-FREE"#);
}

#[test]
fn dict_empty_key_and_similar_keys() {
    let mut f = forth();
    run(&mut f, "DICT-VARIABLE D");
    run(&mut f, r#"1 S" " D DICT!  2 S" a" D DICT!  3 S" ab" D DICT!  4 S" abc" D DICT!"#);
    assert_eq!(
        run(&mut f, r#"S" " D DICT@ . . S" a" D DICT@ . . S" ab" D DICT@ . . S" abc" D DICT@ . ."#),
        "-1 1 -1 2 -1 3 -1 4 "
    );
    assert_eq!(run(&mut f, r#"S" abcd" D DICT@ ."#), "0 ");
    run(&mut f, "D DICT-FREE");
}

#[test]
fn dict_keeps_the_order_and_survives_growth_and_deletes() {
    let mut f = forth();
    run(&mut f, "DICT-VARIABLE D  : KEY ( n -- c-addr u ) S>D <# #S S\" k\" HOLDS #> ;");
    // 500 keys, so the tables move several times
    def(&mut f, "500 0 DO I I KEY D DICT! LOOP");
    assert_eq!(run(&mut f, "D DICT# ."), "500 ");
    assert_eq!(
        def(&mut f, "0 500 0 DO I KEY D DICT@ DROP + LOOP ."),
        format!("{} ", (0..500).sum::<i32>())
    );
    // delete the odd ones, add some new ones: the tables are rebuilt without the deleted
    def(&mut f, "500 1 DO I KEY D DICT-DELETE DROP 2 +LOOP");
    assert_eq!(run(&mut f, "D DICT# ."), "250 ");
    def(&mut f, "1000 500 DO I I KEY D DICT! LOOP");
    assert_eq!(run(&mut f, "D DICT# ."), "750 ");
    assert_eq!(run(&mut f, "3 KEY D DICT@ ."), "0 ");
    assert_eq!(run(&mut f, "4 KEY D DICT@ . ."), "-1 4 ");
    // the order: the old even keys, then the new ones
    run(&mut f, "VEC-VARIABLE V  : EACH ( c-addr u x -- ) NIP NIP V VEC+! ;");
    run(&mut f, "D ' EACH DICT-EACH");
    assert_eq!(
        run(&mut f, "V VEC# . 0 V VEC@ . 1 V VEC@ . 249 V VEC@ . 250 V VEC@ . 749 V VEC@ ."),
        "750 0 2 498 500 999 "
    );
    run(&mut f, "D DICT-FREE V VEC-FREE");
}

#[test]
fn dict_and_vector_use_the_heap_and_give_it_back() {
    let mut f = forth();
    run(&mut f, "DICT-VARIABLE D VEC-VARIABLE V UNUSED");
    run(&mut f, ": KEY ( n -- c-addr u ) S>D <# #S S\" k\" HOLDS #> ;");
    let cycle = ": T 100 0 DO I I KEY D DICT! I V VEC+! LOOP D DICT-FREE V VEC-FREE ; T";
    run(&mut f, cycle);
    let before = run(&mut f, "UNUSED .");
    run(&mut f, "T");
    // everything that was allocated is freed again: the heap can hold what it could before
    assert_eq!(run(&mut f, "UNUSED ."), before);
}

#[test]
fn dict_to_json() {
    let mut f = Forth::new();
    f.set_input(Box::new(Cursor::new(Vec::new())));
    f.install(&Collections);
    f.install(&Json);
    f.interpret("ALSO COLLECTIONS ALSO JSON DICT-VARIABLE D").unwrap();
    run(&mut f, r#"1 S" a" D DICT!  2 S" b" D DICT!  3 S" c" D DICT!"#);
    run(&mut f, ": ENTRY ( c-addr u x -- ) >R JSON-KEY R> JSON-NUMBER ;");
    assert_eq!(run(&mut f, "JSON{ D ' ENTRY DICT-EACH JSON}"), r#"{"a":1,"b":2,"c":3}"#);
}

#[test]
fn memory_comes_from_the_heap() {
    let mut f = Forth::with_memory(64 * 1024);
    f.set_input(Box::new(Cursor::new(Vec::new())));
    f.install(&Collections);
    f.interpret("ALSO COLLECTIONS VEC-VARIABLE V DICT-VARIABLE D").unwrap();
    // pushing without end stops with an error of ALLOCATE or RESIZE, not a crash
    let c = code(&mut f, ": T BEGIN 1 V VEC+! AGAIN ; T");
    assert!(c == -59 || c == -61, "{c}");
    run(&mut f, "V VEC-FREE");
    run(&mut f, ": KEY ( n -- c-addr u ) S>D <# #S S\" k\" HOLDS #> ;");
    let c = code(&mut f, ": U 100000 0 DO I I KEY D DICT! LOOP ; U");
    assert!(c == -59 || c == -61, "{c}");
    run(&mut f, "D DICT-FREE");
}

#[test]
fn names_stay_in_their_word_list() {
    let mut f = Forth::new();
    f.install(&Collections);
    assert_eq!(code(&mut f, "VEC-VARIABLE V"), -13);
    assert_eq!(run(&mut f, "GET-ORDER NIP ."), "1 ");
    run(&mut f, ": SQUARE DUP * ; 7 SQUARE .");
}

#[test]
fn install_ignores_the_limits_of_the_program() {
    let mut f = Forth::new();
    f.set_limits(Limits { steps: Some(10), space: Some(10), ..Limits::default() });
    f.install(&Collections);
    f.set_limits(Limits::default());
    run(&mut f, "ALSO COLLECTIONS VEC-VARIABLE V");
}

#[test]
fn sort_cells_sorts_in_place_and_is_stable() {
    let mut f = forth();
    run(&mut f, "CREATE A 9 , 3 , 7 , 1 , 8 , 2 , 5 ,");
    run(&mut f, "A 7 ' < SORT-CELLS");
    assert_eq!(def(&mut f, "7 0 DO A I CELLS + @ . LOOP"), "1 2 3 5 7 8 9 ");
    run(&mut f, "A 7 ' > SORT-CELLS");
    assert_eq!(def(&mut f, "7 0 DO A I CELLS + @ . LOOP"), "9 8 7 5 3 2 1 ");
    // 0 and 1 cells
    run(&mut f, "A 0 ' < SORT-CELLS A 1 ' < SORT-CELLS");
    assert_eq!(run(&mut f, "A @ ."), "9 ");
    // stable: sort by the tens only, 10 times a pair (key, order)
    run(&mut f, "CREATE P 31 , 22 , 35 , 21 , 33 , 24 , 12 , 36 ,");
    run(&mut f, ": TENS< ( a b -- flag ) 10 / SWAP 10 / SWAP < ;");
    run(&mut f, "P 8 ' TENS< SORT-CELLS");
    assert_eq!(def(&mut f, "8 0 DO P I CELLS + @ . LOOP"), "12 22 21 24 31 35 33 36 ");
}

#[test]
fn sort_cells_with_a_definition_and_a_thousand_cells() {
    let mut f = forth();
    run(&mut f, "VEC-VARIABLE V  : BEFORE ( a b -- flag ) < ;");
    // a scrambled sequence
    def(&mut f, "1000 0 DO I 7919 * 1009 MOD V VEC+! LOOP");
    run(&mut f, "V ' BEFORE VEC-SORT");
    assert_eq!(
        def(&mut f, "0 V VEC@ . 999 V VEC@ . 0 1000 1 DO I V VEC@ I 1- V VEC@ < IF 1+ THEN LOOP ."),
        "0 1008 0 "
    );
}

#[test]
fn sort_cells_errors() {
    let mut f = forth();
    run(&mut f, "CREATE A 2 , 1 , 3 ,");
    // a token with the wrong stack effect
    assert_eq!(code(&mut f, "A 3 ' 2DROP SORT-CELLS"), -24);
    // a token that throws leaves the cells as they were
    assert_eq!(code(&mut f, ": BAD 2DROP -77 THROW ; A 3 ' BAD SORT-CELLS"), -77);
    assert_eq!(run(&mut f, "A @ . A CELL+ @ . A 2 CELLS + @ ."), "2 1 3 ");
    // a bad block
    assert_eq!(code(&mut f, "A -1 ' < SORT-CELLS"), -24);
    assert_eq!(code(&mut f, "-100 5 ' < SORT-CELLS"), -9);
}

#[test]
fn environment_knows_the_extension() {
    let mut f = forth();
    assert_eq!(run(&mut f, r#"S" FORTHLING-COLLECTIONS" ENVIRONMENT? . ."#), "-1 1 ");
}

#[test]
fn string_arrays_sort() {
    let mut f = Forth::new();
    f.set_input(Box::new(Cursor::new(Vec::new())));
    f.install(&Strings);
    f.interpret("ALSO DYNAMIC-STRINGS $[]VARIABLE A").unwrap();
    run(
        &mut f,
        r#"S" pear" A $[]+!  S" apple" A $[]+!  S" " A $[]+!  S" fig" A $[]+!  S" Apple" A $[]+!  S" ap" A $[]+!"#,
    );
    run(&mut f, "A $[]SORT");
    run(&mut f, r#": BAR ( c-addr u -- ) TYPE ." |" ;"#);
    assert_eq!(run(&mut f, "A ' BAR $[]MAP"), "|Apple|ap|apple|fig|pear|");
    // sorting an empty array or a short one
    run(&mut f, "$[]VARIABLE E  E $[]SORT  S\" x\" E $[]+! E $[]SORT");
    assert_eq!(run(&mut f, "E ' BAR $[]MAP"), "x|");
}
