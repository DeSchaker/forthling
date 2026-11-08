//! The SCIENCE words, used from Forth. Reference values come from exact
//! rational arithmetic (Python `fractions`), or from the literature.

use forthling::{Cell, Forth, Limits};
use forthling_science::{SCIENCE_ERROR, Science};

fn forth() -> Forth {
    let mut f = Forth::new();
    f.install(&Science);
    f.interpret("ALSO SCIENCE").unwrap();
    f.interpret(DEFS).unwrap();
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

/// Runs the text and takes the floats it left.
fn floats(f: &mut Forth, text: &str) -> Vec<f64> {
    f.interpret(text).unwrap_or_else(|e| panic!("{text}: {e}"));
    let v = f.fstack().to_vec();
    f.interpret("FCLEAR").unwrap();
    v
}

fn near(got: f64, want: f64, rel: f64) {
    assert!((got - want).abs() <= rel * want.abs().max(1e-300), "{got} is not {want} within {rel}");
}

/// Words that read a vector or a matrix into the float stack.
const DEFS: &str = "
: FV@S  {: v :}  v FV-LEN 0 ?DO I v FV[] F@ LOOP ;
: MAT@S {: m :}  m MAT-ROWS 0 ?DO m MAT-COLS 0 ?DO J I m MAT[] F@ LOOP LOOP ;
: FCLEAR  BEGIN FDEPTH WHILE FDROP REPEAT ;
: ROUND  20 MAT-HILBERT DUP MAT-INV FREE THROW FREE THROW ;
: ROUNDS  0 DO ROUND LOOP ;
";

/// The 4 by 4 matrix and the right-hand side of the tests below.
const A4: &str = "4e -2e 1e 3e  3e 6e -4e 2e  2e 1e 8e -5e  -1e 3e 2e 7e 4 4 MAT-FROM-STACK";
const B4: &str = "10e -3e 7e 1e 4 FV-FROM-STACK";

#[test]
fn vectors() {
    let mut f = forth();
    assert_eq!(run(&mut f, "3 FV-NEW FV-LEN ."), "3 ");
    assert_eq!(run(&mut f, "0 FV-NEW FV-LEN ."), "0 ");
    f.interpret("1e 2e 3e 3 FV-FROM-STACK CONSTANT V").unwrap();
    assert_eq!(floats(&mut f, "V FV@S"), [1.0, 2.0, 3.0]);
    // Elements are written with F!, and FV-FILL sets all of them.
    f.interpret("9e 1 V FV[] F!").unwrap();
    assert_eq!(floats(&mut f, "V FV@S"), [1.0, 9.0, 3.0]);
    f.interpret("7e V FV-FILL").unwrap();
    assert_eq!(floats(&mut f, "V FV@S"), [7.0, 7.0, 7.0]);
}

#[test]
fn vector_arithmetic_makes_new_vectors() {
    let mut f = forth();
    f.interpret("1e 2e 3e 3 FV-FROM-STACK CONSTANT A  4e 5e 6e 3 FV-FROM-STACK CONSTANT B")
        .unwrap();
    assert_eq!(floats(&mut f, "A B FV-DOT"), [32.0]);
    assert_eq!(floats(&mut f, "A B FV+ FV@S"), [5.0, 7.0, 9.0]);
    assert_eq!(floats(&mut f, "B A FV- FV@S"), [3.0, 3.0, 3.0]);
    assert_eq!(floats(&mut f, "A 2e FV-SCALE FV@S"), [2.0, 4.0, 6.0]);
    assert_eq!(floats(&mut f, "3e 4e 2 FV-FROM-STACK FV-NORM"), [5.0]);
    // The arguments were not changed.
    assert_eq!(floats(&mut f, "A FV@S B FV@S"), [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
}

#[test]
fn matrices_are_stored_a_row_after_a_row() {
    let mut f = forth();
    f.interpret("1e 2e 3e 4e 5e 6e 2 3 MAT-FROM-STACK CONSTANT M").unwrap();
    assert_eq!(run(&mut f, "M MAT-ROWS . M MAT-COLS ."), "2 3 ");
    assert_eq!(floats(&mut f, "M MAT@S"), [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    assert_eq!(floats(&mut f, "1 2 M MAT[] F@"), [6.0]);
    f.interpret("0e 0 1 M MAT[] F!").unwrap();
    assert_eq!(floats(&mut f, "M MAT@S"), [1.0, 0.0, 3.0, 4.0, 5.0, 6.0]);
    assert_eq!(floats(&mut f, "M MAT-T MAT@S"), [1.0, 4.0, 0.0, 5.0, 3.0, 6.0]);
    assert_eq!(run(&mut f, "M MAT-T DUP MAT-ROWS . MAT-COLS ."), "3 2 ");
    assert_eq!(floats(&mut f, "2 2 MAT-NEW MAT@S"), [0.0; 4]);
    assert_eq!(floats(&mut f, "3 MAT-IDENTITY MAT@S").len(), 9);
}

#[test]
fn matrix_arithmetic() {
    let mut f = forth();
    f.interpret("1e 2e 3e 4e 2 2 MAT-FROM-STACK CONSTANT A").unwrap();
    f.interpret("0e 1e 1e 0e 2 2 MAT-FROM-STACK CONSTANT S").unwrap();
    assert_eq!(floats(&mut f, "A S MAT+ MAT@S"), [1.0, 3.0, 4.0, 4.0]);
    assert_eq!(floats(&mut f, "A S MAT- MAT@S"), [1.0, 1.0, 2.0, 4.0]);
    assert_eq!(floats(&mut f, "A 2e MAT-SCALE MAT@S"), [2.0, 4.0, 6.0, 8.0]);
    // Multiplying by the swap matrix swaps the columns.
    assert_eq!(floats(&mut f, "A S MAT* MAT@S"), [2.0, 1.0, 4.0, 3.0]);
    assert_eq!(floats(&mut f, "A 2 MAT-IDENTITY MAT* MAT@S"), [1.0, 2.0, 3.0, 4.0]);
    // A 2 by 3 times a 3 by 1 is a vector of 2.
    f.interpret("1e 2e 3e 4e 5e 6e 2 3 MAT-FROM-STACK CONSTANT R").unwrap();
    assert_eq!(floats(&mut f, "R 1e 0e -1e 3 FV-FROM-STACK MAT*V FV@S"), [-2.0, -2.0]);
    assert_eq!(floats(&mut f, "A MAT-NORM"), [7.0]);
}

#[test]
fn determinant_inverse_and_solve_match_the_exact_values() {
    let mut f = forth();
    f.interpret(&format!("{A4} CONSTANT A  {B4} CONSTANT B")).unwrap();
    near(floats(&mut f, "A MAT-DET")[0], 2800.0, 1e-13);
    // The inverse, row by row, from Fractions.
    let inv = [
        [
            0.157_142_857_142_857_14,
            0.078_571_428_571_428_57,
            0.035_714_285_714_285_71,
            -0.064_285_714_285_714_28,
        ],
        [
            -0.092_142_857_142_857_14,
            0.103_928_571_428_571_42,
            0.051_785_714_285_714_29,
            0.046_785_714_285_714_285,
        ],
        [
            0.009_285_714_285_714_286,
            -0.045_357_142_857_142_86,
            0.083_928_571_428_571_42,
            0.068_928_571_428_571_43,
        ],
        [
            0.059_285_714_285_714_28,
            -0.020_357_142_857_142_86,
            -0.041_071_428_571_428_57,
            0.093_928_571_428_571_43,
        ],
    ];
    let got = floats(&mut f, "A MAT-INV MAT@S");
    for (i, want) in inv.iter().flatten().enumerate() {
        near(got[i], *want, 1e-12);
    }
    let x = [
        1.521_428_571_428_571_4,
        -0.823_928_571_428_571_5,
        0.885_357_142_857_142_8,
        0.460_357_142_857_142_85,
    ];
    for (got, want) in floats(&mut f, "B A MAT-SOLVE FV@S").iter().zip(x) {
        near(*got, want, 1e-12);
    }
    // Gauss-Jordan with the right-hand side as a one-column matrix.
    f.interpret("10e -3e 7e 1e 4 1 MAT-FROM-STACK CONSTANT BM").unwrap();
    for (got, want) in floats(&mut f, "A BM MAT-GAUSS-JORDAN MAT@S").iter().zip(x) {
        near(*got, want, 1e-12);
    }
    // A times its inverse is the identity.
    let id = floats(&mut f, "A A MAT-INV MAT* MAT@S");
    for (k, v) in id.iter().enumerate() {
        near(*v + 1.0, if k % 5 == 0 { 2.0 } else { 1.0 }, 1e-12);
    }
}

#[test]
fn lu_factors_once_and_solves_many_times() {
    let mut f = forth();
    f.interpret(&format!("{A4} MAT-LU")).unwrap();
    // ( lu piv sign ): the sign of the permutation is 1 or -1.
    let sign = *f.stack().last().unwrap();
    assert!(sign == 1 || sign == -1, "{sign}");
    f.interpret("DROP CONSTANT PIV CONSTANT LU").unwrap();
    f.interpret(&format!("{A4} CONSTANT A")).unwrap();
    let x = [
        1.521_428_571_428_571_4,
        -0.823_928_571_428_571_5,
        0.885_357_142_857_142_8,
        0.460_357_142_857_142_85,
    ];
    let got = floats(&mut f, "10e -3e 7e 1e 4 FV-FROM-STACK LU PIV LU-SOLVE FV@S");
    for (g, w) in got.iter().zip(x) {
        near(*g, w, 1e-12);
    }
    // A times each solution is its right-hand side.
    for (text, b) in
        [("4e 2e 8e -5e", [4.0, 2.0, 8.0, -5.0]), ("0e 1e 0e 0e", [0.0, 1.0, 0.0, 0.0])]
    {
        let back = floats(&mut f, &format!("A {text} 4 FV-FROM-STACK LU PIV LU-SOLVE MAT*V FV@S"));
        for (g, w) in back.iter().zip(b) {
            near(*g + 1.0, w + 1.0, 1e-12);
        }
    }
}

#[test]
fn a_pivot_is_needed_when_the_first_one_is_zero() {
    let mut f = forth();
    // The swap matrix has determinant -1 and needs a row exchange.
    assert_eq!(floats(&mut f, "0e 1e 1e 0e 2 2 MAT-FROM-STACK MAT-DET"), [-1.0]);
    assert_eq!(
        floats(&mut f, "0e 1e 1e 0e 2 2 MAT-FROM-STACK MAT-INV MAT@S"),
        [0.0, 1.0, 1.0, 0.0]
    );
    assert_eq!(
        floats(&mut f, "3e 5e 2 FV-FROM-STACK 0e 1e 1e 0e 2 2 MAT-FROM-STACK MAT-SOLVE FV@S"),
        [5.0, 3.0]
    );
}

#[test]
fn the_hilbert_matrix() {
    let mut f = forth();
    near(floats(&mut f, "3 MAT-HILBERT MAT-DET")[0], 1.0 / 2160.0, 1e-13);
    // The inverse of the 3 by 3 Hilbert matrix has whole numbers.
    let inv = [9.0, -36.0, 30.0, -36.0, 192.0, -180.0, 30.0, -180.0, 180.0];
    for (got, want) in floats(&mut f, "3 MAT-HILBERT MAT-INV MAT@S").iter().zip(inv) {
        near(*got, want, 1e-10);
    }
    // The condition number in the infinity norm grows very fast.
    for (n, cond, rel) in
        [(3, 748.0, 1e-12), (4, 28375.0, 1e-10), (5, 943_656.0, 1e-8), (8, 33_872_791_095.0, 1e-3)]
    {
        near(floats(&mut f, &format!("{n} MAT-HILBERT MAT-COND"))[0], cond, rel);
    }
    // Solving with H(8) loses digits to the conditioning: the answer is only
    // roughly the vector of ones.
    f.interpret("8 MAT-HILBERT CONSTANT H  8 FV-NEW CONSTANT ONES  1e ONES FV-FILL").unwrap();
    let x = floats(&mut f, "H ONES MAT*V H MAT-SOLVE FV@S");
    let worst = x.iter().map(|v| (v - 1.0).abs()).fold(0.0, f64::max);
    assert!(worst < 1e-5, "{worst}");
}

#[test]
fn singular_matrices() {
    let mut f = forth();
    let rows = "1e 2e 2e 4e 2 2 MAT-FROM-STACK";
    assert_eq!(floats(&mut f, &format!("{rows} MAT-DET")), [0.0]);
    assert_eq!(code(&mut f, &format!("{rows} MAT-INV")), SCIENCE_ERROR);
    assert_eq!(code(&mut f, &format!("{rows} MAT-LU")), SCIENCE_ERROR);
    assert_eq!(code(&mut f, &format!("{rows} MAT-COND")), SCIENCE_ERROR);
    assert_eq!(code(&mut f, &format!("1e 2e 2 FV-FROM-STACK {rows} MAT-SOLVE")), SCIENCE_ERROR);
}

#[test]
fn bad_indexes_and_sizes() {
    let mut f = forth();
    f.interpret("3 FV-NEW CONSTANT V  2 3 MAT-NEW CONSTANT M").unwrap();
    assert_eq!(code(&mut f, "3 V FV[]"), -24);
    assert_eq!(code(&mut f, "-1 V FV[]"), -24);
    assert_eq!(code(&mut f, "2 0 M MAT[]"), -24);
    assert_eq!(code(&mut f, "0 3 M MAT[]"), -24);
    assert_eq!(code(&mut f, "0 -1 M MAT[]"), -24);
    assert_eq!(code(&mut f, "-1 FV-NEW"), -24);
    assert_eq!(code(&mut f, "-1 2 MAT-NEW"), -24);
    f.interpret("2 V FV[] DROP").unwrap();
}

#[test]
fn a_wrong_address_or_kind_is_an_error_and_never_other_memory() {
    let mut f = forth();
    f.interpret("3 FV-NEW CONSTANT V  2 2 MAT-NEW CONSTANT M  CREATE JUNK 100 ALLOT").unwrap();
    assert_eq!(code(&mut f, "M FV-LEN"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "V MAT-ROWS"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "JUNK FV-LEN"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "V M MAT*"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "99999999999 FV-LEN"), -9);
    assert_eq!(code(&mut f, "-8 FV-LEN"), -9);
    // A header cut off at the end of memory.
    assert_eq!(code(&mut f, "HERE 100000000 + FV-LEN"), -9);
}

#[test]
fn shapes_must_fit() {
    let mut f = forth();
    f.interpret("3 FV-NEW CONSTANT V3  2 FV-NEW CONSTANT V2").unwrap();
    f.interpret("2 3 MAT-NEW CONSTANT M23  2 3 MAT-NEW CONSTANT M23B  3 2 MAT-NEW CONSTANT M32")
        .unwrap();
    assert_eq!(code(&mut f, "V3 V2 FV+"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "V3 V2 FV-DOT"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "M23 M32 MAT+"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "M23 M23B MAT*"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "M23 V2 MAT*V"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "M23 MAT-DET"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "M23 MAT-INV"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "V2 3 MAT-IDENTITY MAT-SOLVE"), SCIENCE_ERROR);
    // M23 times M32 is fine.
    f.interpret("M23 M32 MAT* DROP").unwrap();
}

#[test]
fn an_array_larger_than_the_memory_is_an_error() {
    let mut f = forth();
    assert_eq!(code(&mut f, "100000000 FV-NEW"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "100000 100000 MAT-NEW"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "9223372036854775807 FV-NEW"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "100000 MAT-HILBERT"), SCIENCE_ERROR);
}

#[test]
fn arrays_can_be_freed_and_the_heap_reused() {
    let mut f = forth();
    // Each round allocates two matrices; without FREE the heap would fill.
    f.interpret("50 ROUNDS").unwrap();
}

#[test]
fn the_work_is_charged() {
    let mut f = Forth::sandboxed();
    f.install(&Science);
    f.set_limits(Limits { steps: Some(1000), ..Limits::default() });
    f.interpret("ALSO SCIENCE  30 MAT-HILBERT CONSTANT H").unwrap();
    let e = f.interpret("H MAT-INV").unwrap_err();
    assert_eq!(e.code, forthling::LIMIT_EXCEEDED, "{e}");
}

#[test]
fn the_environment_name_is_there() {
    let mut f = forth();
    assert_eq!(run(&mut f, r#"S" FORTHLING-SCIENCE" ENVIRONMENT? . ."#), "-1 1 ");
}

#[test]
fn every_word_has_a_help_line_and_its_examples_work() {
    let mut f = Forth::new();
    f.install(&Science);
    let problems = f.help_problems();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn the_example_gives_the_output_beside_it() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/linear_algebra.fs");
    let mut f = Forth::new();
    f.install(&Science);
    f.include(path).unwrap_or_else(|e| panic!("{e}"));
    let out = String::from_utf8_lossy(f.output()).into_owned();
    let expected = std::fs::read_to_string(path.replace(".fs", ".out")).unwrap();
    assert_eq!(out, expected);
}
