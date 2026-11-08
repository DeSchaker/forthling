//! The SCIENCE words, used from Forth. Reference values come from exact
//! rational arithmetic (Python `fractions`), or from the literature.

#![allow(
    clippy::unreadable_literal,
    clippy::float_cmp,
    reason = "reference values are copied as printed by Python; some results must be exact"
)]

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
fn the_examples_give_the_output_beside_them() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/examples");
    let mut count = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "fs") {
            continue;
        }
        let mut f = Forth::new();
        f.install(&Science);
        f.include(path.to_str().unwrap()).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let out = String::from_utf8_lossy(f.output()).into_owned();
        let expected = std::fs::read_to_string(path.with_extension("out")).unwrap();
        assert_eq!(out, expected, "{}", path.display());
        count += 1;
    }
    assert!(count >= 3, "found {count} examples");
}

/// One float from a one-argument word: `eval1(f, "GAMMA", 0.5)`.
fn eval1(f: &mut Forth, word: &str, x: f64) -> f64 {
    floats(f, &format!("{x:e} {word}"))[0]
}

fn eval2(f: &mut Forth, word: &str, a: f64, x: f64) -> f64 {
    floats(f, &format!("{a:e} {x:e} {word}"))[0]
}

#[test]
fn gamma_and_factorial() {
    let mut f = forth();
    // Reference values from Python's math module (an independent implementation).
    let gamma = [
        (0.1, 9.513507698668732),
        (0.5, 1.7724538509055159),
        (0.9, 1.068628702119319),
        (1.5, 0.886226925452758),
        (2.5, 1.3293403881791372),
        (3.7, 4.170651783796603),
        (7.25, 1155.3810139199893),
        (12.5, 136843365.46556583),
        (20.3, 2.97246107523557e+17),
        (50.5, 4.29046291235196e+63),
        (100.1, 1.47845449465148e+156),
        (170.5, 5.56209241456e+305),
        (-0.5, -3.544907701811032),
        (-1.5, 2.3632718012073544),
        (-2.7, -0.931082784838964),
        (-10.3, -5.26236323953561e-07),
    ];
    for (x, want) in gamma {
        near(eval1(&mut f, "GAMMA", x), want, 2e-13);
    }
    // At whole numbers gamma is a factorial, and exact where a float can be.
    assert_eq!(eval1(&mut f, "GAMMA", 1.0), 1.0);
    assert_eq!(eval1(&mut f, "GAMMA", 5.0), 24.0);
    assert_eq!(eval1(&mut f, "GAMMA", 21.0), 2_432_902_008_176_640_000.0);
    near(eval1(&mut f, "GAMMA", 171.0), 7.257415615307994e306, 1e-15);
    near(eval1(&mut f, "GAMMA", 1e-10), 1e10, 1e-9);
    assert_eq!(
        floats(&mut f, "0 FACTORIAL 1 FACTORIAL 5 FACTORIAL 20 FACTORIAL"),
        [1.0, 1.0, 120.0, 2_432_902_008_176_640_000.0]
    );
    near(floats(&mut f, "170 FACTORIAL")[0], 7.257415615307994e306, 1e-15);
    // Poles, results out of range, a negative factorial.
    assert_eq!(code(&mut f, "0e GAMMA"), -46);
    assert_eq!(code(&mut f, "-3e GAMMA"), -46);
    assert_eq!(code(&mut f, "172e GAMMA"), -43);
    assert_eq!(code(&mut f, "171 FACTORIAL"), -43);
    assert_eq!(code(&mut f, "-1 FACTORIAL"), -24);
}

#[test]
fn log_gamma() {
    let mut f = forth();
    let lgamma = [
        (0.1, 2.2527126517342055),
        (0.5, 0.5723649429247004),
        (0.9, 0.0663762397347431),
        (1.5, -0.12078223763524543),
        (2.5, 0.2846828704729196),
        (3.7, 1.4280723266653883),
        (7.25, 7.0521854507385395),
        (12.5, 18.73434751193645),
        (20.3, 40.23333683543724),
        (50.5, 146.5192554907206),
        (100.1, 359.5942717888569),
        (170.5, 704.0044277342047),
        (1000.5, 5908.674175848678),
        (1e6, 12815504.569147611),
        (-0.5, 1.265512123484645),
        (-1.5, 0.8600470153764812),
        (-2.7, -0.07140708531564521),
        (-10.3, -14.45751544002421),
    ];
    for (x, want) in lgamma {
        let got = eval1(&mut f, "LNGAMMA", x);
        assert!(
            (got - want).abs() <= 1e-13 * want.abs().max(1.0),
            "lngamma({x}) = {got}, not {want}"
        );
    }
    // Exactly zero at 1 and 2, and a pole at 0.
    assert_eq!(eval1(&mut f, "LNGAMMA", 1.0), 0.0);
    assert_eq!(eval1(&mut f, "LNGAMMA", 2.0), 0.0);
    assert_eq!(code(&mut f, "0e LNGAMMA"), -46);
}

#[test]
fn error_function() {
    let mut f = forth();
    let table = [
        (0.0, 0.0, 1.0),
        (0.1, 0.1124629160182849, 0.8875370839817152),
        (0.5, 0.5204998778130465, 0.4795001221869534),
        (1.0, 0.8427007929497149, 0.1572992070502851),
        (1.5, 0.9661051464753108, 0.033894853524689274),
        (2.0, 0.9953222650189527, 0.004677734981047265),
        (3.0, 0.9999779095030014, 2.20904969985854e-05),
        (4.5, 0.9999999998033839, 1.9661604415428876e-10),
        (6.0, 1.0, 2.1519736712498913e-17),
        (-0.7, -0.6778011938374183, 1.6778011938374182),
        (-2.0, -0.9953222650189527, 1.9953222650189528),
    ];
    for (x, erf, erfc) in table {
        let got = eval1(&mut f, "ERF", x);
        assert!((got - erf).abs() <= 1e-14, "erf({x}) = {got}, not {erf}");
        near(eval1(&mut f, "ERFC", x), erfc, 1e-12);
    }
    // The tail goes to 0 without an error.
    assert_eq!(eval1(&mut f, "ERFC", 30.0), 0.0);
    assert_eq!(eval1(&mut f, "ERF", 30.0), 1.0);
}

#[test]
fn incomplete_gamma() {
    let mut f = forth();
    // P(1, x) = 1 - e^-x.
    for x in [0.01, 0.5, 1.0, 2.0, 5.0, 40.0] {
        near(eval2(&mut f, "GAMMA-P", 1.0, x), 1.0 - (-x).exp(), 1e-13);
        near(eval2(&mut f, "GAMMA-Q", 1.0, x), (-x).exp(), 1e-13);
    }
    // Q(n, x) = e^-x (1 + x + ... + x^(n-1)/(n-1)!) for a whole n, in the
    // series (x < n + 1) and in the fraction (x >= n + 1).
    for n in [1_u32, 3, 8] {
        for x in [0.3, 2.5, 7.0, 30.0] {
            let mut term = 1.0;
            let mut sum = 0.0;
            for k in 0..n {
                sum += term;
                term *= x / (f64::from(k) + 1.0);
            }
            near(eval2(&mut f, "GAMMA-Q", f64::from(n), x), (-x).exp() * sum, 1e-12);
        }
    }
    // The error function is P(1/2, x^2); the two sides add up to one.
    near(eval2(&mut f, "GAMMA-P", 0.5, 4.0), 0.9953222650189527, 1e-14);
    let (p, q) = (eval2(&mut f, "GAMMA-P", 7.3, 9.1), eval2(&mut f, "GAMMA-Q", 7.3, 9.1));
    assert!((p + q - 1.0).abs() < 1e-14);
    assert_eq!(eval2(&mut f, "GAMMA-P", 2.0, 0.0), 0.0);
    assert_eq!(code(&mut f, "0e 1e GAMMA-P"), -46);
    assert_eq!(code(&mut f, "1e -1e GAMMA-Q"), -46);
}

#[test]
fn beta_functions() {
    let mut f = forth();
    near(eval2(&mut f, "BETA", 2.0, 3.0), 1.0 / 12.0, 1e-13);
    near(eval2(&mut f, "BETA", 0.5, 0.5), std::f64::consts::PI, 1e-13);
    let inc =
        |f: &mut Forth, a: f64, b: f64, x: f64| floats(f, &format!("{a:e} {b:e} {x:e} BETA-I"))[0];
    // For whole a and b: the sum of a binomial tail.
    let binomial = |a: i32, b: i32, x: f64| -> f64 {
        let n = a + b - 1;
        let mut total = 0.0;
        for j in a..=n {
            let mut c = 1.0;
            for i in 0..j {
                c *= f64::from(n - i) / f64::from(i + 1);
            }
            total += c * x.powi(j) * (1.0 - x).powi(n - j);
        }
        total
    };
    assert_eq!(inc(&mut f, 2.0, 3.0, 0.5), 0.6875);
    for (a, b) in [(1, 1), (1, 4), (3, 2), (5, 9), (12, 7)] {
        for x in [0.05, 0.3, 0.5, 0.77, 0.99] {
            near(inc(&mut f, f64::from(a), f64::from(b), x), binomial(a, b, x), 1e-12);
        }
    }
    // I(x; a, b) = 1 - I(1 - x; b, a), and the ends.
    let (u, v) = (inc(&mut f, 2.5, 4.5, 0.3), inc(&mut f, 4.5, 2.5, 0.7));
    assert!((u + v - 1.0).abs() < 1e-14);
    assert_eq!(inc(&mut f, 2.0, 3.0, 0.0), 0.0);
    assert_eq!(inc(&mut f, 2.0, 3.0, 1.0), 1.0);
    near(inc(&mut f, 1.0, 1.0, 0.37), 0.37, 1e-14);
    assert_eq!(code(&mut f, "2e 3e 1.5e BETA-I"), -46);
    assert_eq!(code(&mut f, "0e 3e 0.5e BETA-I"), -46);
    assert_eq!(code(&mut f, "-1e 3e BETA"), -46);
}

#[test]
fn logistic() {
    let mut f = forth();
    assert_eq!(eval1(&mut f, "LOGISTIC", 0.0), 0.5);
    near(eval1(&mut f, "LOGISTIC", 2.0), 0.8807970779778823, 1e-15);
    near(eval1(&mut f, "LOGISTIC", -2.0), 0.11920292202211755, 1e-15);
    // No overflow for a large argument of either sign.
    assert_eq!(eval1(&mut f, "LOGISTIC", 1000.0), 1.0);
    assert_eq!(eval1(&mut f, "LOGISTIC", -1000.0), 0.0);
    assert_eq!(eval1(&mut f, "LOGISTIC-D", 0.0), 0.25);
    near(eval1(&mut f, "LOGISTIC-D", 2.0), 0.8807970779778823 * 0.11920292202211755, 1e-14);
    assert_eq!(eval1(&mut f, "LOGISTIC-D", 1000.0), 0.0);
    assert_eq!(eval1(&mut f, "LOGISTIC-D", -2.0), eval1(&mut f, "LOGISTIC-D", 2.0));
}

/// Words that the tests of integration and differential equations share.
const CALCULUS: &str = "
: SQ  FDUP F* ;
: CUBE  FDUP FDUP F* F* ;
: X9  FDUP FDUP F* FDUP F* FDUP F* F* ;
FVARIABLE XV
: INNER  XV F@ F* ;
: OUTER  XV F!  0e 1e ['] INNER 1e-10 INTEGRATE FDROP ;
: DECAY ( y dy -- ) ( F: t -- )  FDROP SWAP 0 SWAP FV[] F@ FNEGATE 0 SWAP FV[] F! ;
: OSC ( y dy -- ) ( F: t -- )  FDROP
   OVER 1 SWAP FV[] F@  DUP 0 SWAP FV[] F!
   OVER 0 SWAP FV[] F@ FNEGATE  DUP 1 SWAP FV[] F!  2DROP ;
: BLOW ( y dy -- ) ( F: t -- )  FDROP SWAP 0 SWAP FV[] F@ FDUP F* 0 SWAP FV[] F! ;
";

fn calculus() -> Forth {
    let mut f = forth();
    f.interpret(CALCULUS).unwrap();
    f
}

#[test]
fn trapezoid_and_simpson() {
    let mut f = calculus();
    // Simpson is exact for a cubic; the trapezoid rule is not.
    assert_eq!(floats(&mut f, "0e 1e ' CUBE 2 SIMPSON"), [0.25]);
    assert_eq!(floats(&mut f, "0e 1e ' SQ 3 SIMPSON"), floats(&mut f, "0e 1e ' SQ 4 SIMPSON"));
    near(floats(&mut f, "0e 1e ' SQ 10 SIMPSON")[0], 1.0 / 3.0, 1e-15);
    // Ten panels of the trapezoid rule for x^2: 1/3 + h^2 / 6.
    near(floats(&mut f, "0e 1e ' SQ 10 TRAPEZOID")[0], 1.0 / 3.0 + 0.01 / 6.0, 1e-14);
    // The ends in either order.
    near(floats(&mut f, "1e 0e ' SQ 10 SIMPSON")[0], -1.0 / 3.0, 1e-15);
    assert_eq!(code(&mut f, "0e 1e ' SQ 0 SIMPSON"), -24);
    assert_eq!(code(&mut f, "0e 1e ' SQ -3 TRAPEZOID"), -24);
}

#[test]
fn romberg_and_trapezoid_with_tolerance() {
    let mut f = calculus();
    let r = floats(&mut f, "0e 3.141592653589793e ' FSIN 1e-10 ROMBERG");
    near(r[0], 2.0, 1e-10);
    assert!(r[1] >= 0.0 && r[1] < 1e-9, "{r:?}");
    let r = floats(&mut f, "0e 1e ' FEXP 1e-12 ROMBERG");
    near(r[0], std::f64::consts::E - 1.0, 1e-12);
    let r = floats(&mut f, "0e 3.141592653589793e ' FSIN 1e-6 TRAPEZOID-TOL");
    near(r[0], 2.0, 1e-5);
    assert!(r[1] < 1e-5, "{r:?}");
    // A tolerance that cannot be met: -264 after 20 halvings.
    assert_eq!(code(&mut f, "0e 1e ' SQ -1e ROMBERG"), SCIENCE_ERROR);
    f.interpret("FCLEAR").unwrap();
}

#[test]
fn adaptive_gauss_legendre() {
    let mut f = calculus();
    // Five points are exact up to degree 9.
    let r = floats(&mut f, "0e 1e ' X9 1e-12 INTEGRATE");
    near(r[0], 0.1, 1e-14);
    let r = floats(&mut f, "0e 1e ' FSQRT 1e-8 INTEGRATE");
    near(r[0], 2.0 / 3.0, 1e-8);
    assert!(r[1] <= 1e-8, "{r:?}");
    // 1 / (1 + x^2) gives the arctangent.
    let r =
        floats(&mut f, "-1e 1e : LORENTZ FDUP F* 1e F+ 1e FSWAP F/ ; ' LORENTZ 1e-12 INTEGRATE");
    near(r[0], std::f64::consts::FRAC_PI_2, 1e-12);
    // A reversed interval changes the sign; an empty one is zero.
    let r = floats(&mut f, "1e 0e ' X9 1e-12 INTEGRATE");
    near(r[0], -0.1, 1e-14);
    assert_eq!(floats(&mut f, "2e 2e ' X9 1e-12 INTEGRATE"), [0.0, 0.0]);
    // A peak that needs many intervals.
    let r = floats(&mut f, "0e 1e : PEAK 0.3e F- SQ 1e-4 F+ 1e-4 FSWAP F/ ; ' PEAK 1e-8 INTEGRATE");
    // The integral of 1e-4 / ((x - 0.3)^2 + 1e-4) over [0, 1] is 0.01 (atan(70) + atan(30)).
    near(r[0], 0.01 * (70.0f64.atan() + 30.0f64.atan()), 1e-8);
    assert!(r[1] < 1e-7, "{r:?}");
    // Too many intervals: -264.
    assert_eq!(code(&mut f, "0e 1e : WILD 1000e F* FSIN ; ' WILD 1e-14 INTEGRATE"), SCIENCE_ERROR);
    f.interpret("FCLEAR").unwrap();
}

#[test]
fn integration_can_be_nested() {
    // The integral of x * y over the unit square is 1/4: the function that the
    // outer integral calls integrates again.
    let mut f = calculus();
    let r = floats(&mut f, "0e 1e ' OUTER 1e-9 INTEGRATE");
    near(r[0], 0.25, 1e-9);
}

#[test]
fn integration_throws_pass_through() {
    let mut f = calculus();
    // A function that throws leaves the error as it is.
    assert_eq!(code(&mut f, "0e 1e : BAD FDROP -77 THROW ; ' BAD 1e-9 INTEGRATE"), -77);
}

#[test]
fn runge_kutta() {
    let mut f = calculus();
    f.interpret("1e 1 FV-FROM-STACK VALUE Y").unwrap();
    f.interpret("Y ' DECAY 10 0e 1e RK4").unwrap();
    near(floats(&mut f, "0 Y FV[] F@")[0], (-1.0f64).exp(), 1e-6);
    // The error falls with the fourth power of the step.
    let err = |f: &mut Forth, steps: u32| {
        f.interpret("1e 1 FV-FROM-STACK TO Y").unwrap();
        f.interpret(&format!("Y ' DECAY {steps} 0e 1e RK4")).unwrap();
        (floats(f, "0 Y FV[] F@")[0] - (-1.0f64).exp()).abs()
    };
    let (e1, e2) = (err(&mut f, 10), err(&mut f, 20));
    assert!(e1 / e2 > 12.0 && e1 / e2 < 20.0, "{e1} {e2}");
    // A system: the oscillator, one period.
    f.interpret("1e 0e 2 FV-FROM-STACK TO Y").unwrap();
    f.interpret("Y ' OSC 200 0e 6.283185307179586e RK4").unwrap();
    let y = floats(&mut f, "0 Y FV[] F@ 1 Y FV[] F@");
    near(y[0], 1.0, 1e-8);
    assert!(y[1].abs() < 1e-7, "{y:?}");
    assert_eq!(code(&mut f, "Y ' OSC 0 0e 1e RK4"), -24);
}

#[test]
fn adaptive_runge_kutta() {
    let mut f = calculus();
    f.interpret("1e 1 FV-FROM-STACK VALUE Y").unwrap();
    f.interpret("Y ' DECAY 0e 1e 1e-9 ODE-RKF").unwrap();
    near(floats(&mut f, "0 Y FV[] F@")[0], (-1.0f64).exp(), 1e-8);
    // The oscillator over a period, and backwards in time.
    f.interpret("1e 0e 2 FV-FROM-STACK TO Y").unwrap();
    f.interpret("Y ' OSC 0e 6.283185307179586e 1e-10 ODE-RKF").unwrap();
    let y = floats(&mut f, "0 Y FV[] F@ 1 Y FV[] F@");
    near(y[0], 1.0, 1e-8);
    assert!(y[1].abs() < 1e-8, "{y:?}");
    f.interpret("1e 0e 2 FV-FROM-STACK TO Y").unwrap();
    f.interpret("Y ' OSC 0e -3e 1e-9 ODE-RKF").unwrap();
    let y = floats(&mut f, "0 Y FV[] F@ 1 Y FV[] F@");
    near(y[0], (-3.0f64).cos(), 1e-7);
    near(y[1], 3.0f64.sin(), 1e-7);
    // An interval of no length leaves y as it was.
    f.interpret("5e 1 FV-FROM-STACK TO Y").unwrap();
    f.interpret("Y ' DECAY 2e 2e 1e-9 ODE-RKF").unwrap();
    assert_eq!(floats(&mut f, "0 Y FV[] F@"), [5.0]);
    // y' = y^2 with y(0) = 1 blows up at t = 1: the step gets too small and the
    // word ends with -264, not in a loop.
    f.interpret("1e 1 FV-FROM-STACK TO Y").unwrap();
    assert_eq!(code(&mut f, "Y ' BLOW 0e 2e 1e-9 ODE-RKF"), SCIENCE_ERROR);
}

/// Functions for the root finders and the interpolation.
const ROOTS: &str = "
: SQ2 ( F: x -- y ) FDUP F* 2e F- ;
: DSQ2 ( F: x -- y ) 2e F* ;
: COSMX ( F: x -- y ) FDUP FCOS FSWAP F- ;
: DCOSMX ( F: x -- y ) FSIN FNEGATE 1e F- ;
: FLAT ( F: x -- y ) FDROP 1e ;
: STEEP ( F: x -- y ) 6 0 DO FDUP F* LOOP 1e F- ;
: STEEPER ( F: x -- y ) 10 0 DO FDUP F* LOOP 1e F- ;
: NAN ( F: x -- y ) FDROP 0e 0e F/ ;
: CUBEM ( F: x -- y ) FDUP FDUP F* F* 1e F- ;
: ZEROD ( F: x -- y ) FDROP 0e ;
FVARIABLE SEEN
: COUNTING ( F: x -- y ) 1e SEEN F@ F+ SEEN F! FDUP F* 2e F- ;
";

fn roots() -> Forth {
    let mut f = forth();
    f.interpret(ROOTS).unwrap();
    f
}

fn assert_all_near(got: &[f64], want: &[f64], rel: f64) {
    assert_eq!(got.len(), want.len(), "{got:?} and {want:?}");
    for (&g, &w) in got.iter().zip(want) {
        near(g, w, rel);
    }
}

#[test]
fn orthogonal_polynomials() {
    let mut f = roots();
    // Chebyshev: T_n(cos t) = cos(n t) and U_n(cos t) = sin((n+1) t) / sin t.
    let x = 0.7f64;
    let t = x.acos();
    for n in [0, 1, 2, 5, 12, 40] {
        let r = floats(&mut f, &format!("{n} {x:e} CHEBYSHEV-T"));
        assert!((r[0] - (f64::from(n) * t).cos()).abs() < 1e-12, "T{n}: {r:?}");
        let r = floats(&mut f, &format!("{n} {x:e} CHEBYSHEV-U"));
        assert!((r[0] - (f64::from(n + 1) * t).sin() / t.sin()).abs() < 1e-11, "U{n}: {r:?}");
    }
    // Legendre, Hermite (the physicists' form) and Laguerre against their closed forms.
    let x = 0.3f64;
    let want = [
        ("LEGENDRE", 4, (35.0 * x.powi(4) - 30.0 * x * x + 3.0) / 8.0),
        ("LEGENDRE", 5, (63.0 * x.powi(5) - 70.0 * x.powi(3) + 15.0 * x) / 8.0),
        ("HERMITE", 4, 16.0 * x.powi(4) - 48.0 * x * x + 12.0),
        ("HERMITE", 0, 1.0),
        ("LAGUERRE", 3, (-x.powi(3) + 9.0 * x * x - 18.0 * x + 6.0) / 6.0),
        ("LAGUERRE", 1, 1.0 - x),
    ];
    for (word, n, v) in want {
        let r = floats(&mut f, &format!("{n} {x:e} {word}"));
        assert!((r[0] - v).abs() < 1e-13, "{word} {n}: {r:?} not {v}");
    }
    // P_n(1) = 1 for every n.
    near(floats(&mut f, "200 1e LEGENDRE")[0], 1.0, 1e-12);
    assert_eq!(code(&mut f, "-1 0.5e LEGENDRE"), -24);
    assert_eq!(code(&mut f, "3 CHEBYSHEV-T"), -45);
}

#[test]
fn polynomials() {
    let mut f = roots();
    // p = 1 + 2x + 3x^2 + 4x^3.
    f.interpret("1e 2e 3e 4e 4 FV-FROM-STACK CONSTANT P").unwrap();
    assert_eq!(floats(&mut f, "2e P POLY-EVAL"), [49.0]);
    assert_eq!(floats(&mut f, "2e P POLY-EVAL-D"), [49.0, 62.0]);
    assert_eq!(floats(&mut f, "3e 5e 1 FV-FROM-STACK POLY-EVAL"), [5.0]);
    assert_eq!(floats(&mut f, "P POLY-DERIV FV@S"), [2.0, 6.0, 12.0]);
    assert_eq!(floats(&mut f, "1e 1 FV-FROM-STACK POLY-DERIV FV-LEN S>F"), [0.0]);
    // x^3 - 6 x^2 + 11 x - 6 = (x-1)(x-2)(x-3): deflate by the root 1.
    f.interpret("-6e 11e -6e 1e 4 FV-FROM-STACK CONSTANT R").unwrap();
    assert_eq!(floats(&mut f, "1e R POLY-DEFLATE FV@S"), [6.0, -5.0, 1.0]);
    // Laguerre finds each of the three roots from a start near it.
    for (start, root) in [(0.0, 1.0), (2.4, 2.0), (10.0, 3.0)] {
        let r = floats(&mut f, &format!("{start:e} 1e-14 R POLY-LAGUERRE"));
        near(r[0], root, 1e-12);
    }
    // All roots by deflation: find one, divide it out, find the next.
    f.interpret(
        ": ALL-ROOTS ( c -- ) ( F: -- r1 r2 r3 )
            {: c :}
            3 0 DO
               0.2e 1e-14 c POLY-LAGUERRE FDUP
               c POLY-DEFLATE TO c
            LOOP ;",
    )
    .unwrap();
    // (the roots are not printed in order, so only the sum is checked: 1 + 2 + 3)
    let sum = floats(&mut f, "R ALL-ROOTS F+ F+");
    near(sum[0], 6.0, 1e-9);
    // x^2 + 1 has no real root: -264, not a loop. A wrong address, an empty vector.
    f.interpret("1e 0e 1e 3 FV-FROM-STACK CONSTANT Q").unwrap();
    assert_eq!(code(&mut f, "0.5e 1e-12 Q POLY-LAGUERRE"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "1e 0 FV-NEW POLY-EVAL"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "0.5e 1e-12 -1 POLY-LAGUERRE"), -9);
}

#[test]
fn quadratic_and_cubic_roots() {
    let mut f = roots();
    let solve = |f: &mut Forth, word: &str, coefficients: &str| {
        f.interpret(&format!("{coefficients} {word}")).unwrap();
        let n = f.pop().unwrap();
        let v = f.fstack().to_vec();
        f.interpret("FCLEAR").unwrap();
        (n, v)
    };
    let q = |f: &mut Forth, a: &str| solve(f, "QUADRATIC", a);
    assert_eq!(q(&mut f, "1e -3e 2e"), (2, vec![1.0, 2.0]));
    assert_eq!(q(&mut f, "1e 2e 1e"), (2, vec![-1.0, -1.0]));
    assert_eq!(q(&mut f, "1e 0e 1e"), (0, vec![]));
    assert_eq!(q(&mut f, "0e 2e -6e"), (1, vec![3.0]));
    assert_eq!(q(&mut f, "0e 0e 5e"), (0, vec![]));
    assert_eq!(q(&mut f, "1e 0e 0e"), (2, vec![0.0, 0.0]));
    assert_eq!(q(&mut f, "-2e 6e -4e"), (2, vec![1.0, 2.0]));
    // No digits are lost to cancellation: the roots of x^2 + 1e8 x + 1.
    let (n, v) = q(&mut f, "1e 1e8 1e");
    assert_eq!(n, 2);
    near(v[0], -1e8 + 1e-8, 1e-15);
    near(v[1], -1e-8 - 1e-24, 1e-15);
}

#[test]
fn cubic_roots() {
    let mut f = roots();
    let mut c = |a: &str| {
        f.interpret(&format!("{a} CUBIC")).unwrap();
        let n = f.pop().unwrap();
        let v = f.fstack().to_vec();
        f.interpret("FCLEAR").unwrap();
        (n, v)
    };
    let (n, v) = c("1e -6e 11e -6e");
    assert_eq!(n, 3);
    assert_all_near(&v, &[1.0, 2.0, 3.0], 1e-14);
    let (n, v) = c("2e 0e 0e -16e"); // 2 x^3 = 16
    assert_eq!(n, 1);
    assert_all_near(&v, &[2.0], 1e-14);
    let (n, v) = c("1e 0e 1e 0e"); // x^3 + x
    assert_eq!(n, 1);
    assert!(v[0].abs() < 1e-14);
    let (n, v) = c("1e -3e 3e -1e"); // (x-1)^3
    assert_eq!(n, 3);
    assert_all_near(&v, &[1.0, 1.0, 1.0], 1e-5);
    assert_eq!(c("1e 0e 0e 0e"), (3, vec![0.0; 3])); // x^3
    let (n, v) = c("-1e 6e -11e 6e"); // the sign of a does not matter
    assert_eq!(n, 3);
    assert_all_near(&v, &[1.0, 2.0, 3.0], 1e-14);
    // Roots far apart: (x - 0.001)(x - 1)(x + 1000).
    let (n, v) = c("1e 998.999e -1000.999e 1e0");
    assert_eq!(n, 3);
    assert_all_near(&v, &[-1000.0, 0.001, 1.0], 1e-9);
    // a = 0 is a quadratic.
    assert_eq!(c("0e 1e -3e 2e"), (2, vec![1.0, 2.0]));
}

#[test]
fn interpolation() {
    let mut f = roots();
    // The points of y = x^3 - 2x + 1 at 0, 1, 2, 4: a cubic is reproduced exactly.
    f.interpret(
        "0e 1e 2e 4e 4 FV-FROM-STACK CONSTANT XS  1e 0e 5e 57e 4 FV-FROM-STACK CONSTANT YS",
    )
    .unwrap();
    for x in [-1.0f64, 0.5, 3.0, 3.7] {
        let want = x * x * x - 2.0 * x + 1.0;
        near(floats(&mut f, &format!("{x:e} XS YS LAGRANGE"))[0], want, 1e-13);
        let r = floats(&mut f, &format!("{x:e} XS YS AITKEN"));
        near(r[0], want, 1e-13);
        assert!(r[1].is_finite(), "{r:?}");
    }
    // Newton's divided differences: 1, -1, 3, 1 for this cubic.
    assert_all_near(&floats(&mut f, "XS YS NEWTON-DD FV@S"), &[1.0, -1.0, 3.0, 1.0], 1e-14);
    f.interpret("XS YS NEWTON-DD CONSTANT DD").unwrap();
    near(floats(&mut f, "3.7e XS DD NEWTON-EVAL")[0], 3.7f64.powi(3) - 7.4 + 1.0, 1e-13);
    // At a node the value is the value.
    assert_eq!(floats(&mut f, "2e XS YS LAGRANGE"), [5.0]);
    // A single point is a constant.
    assert_eq!(floats(&mut f, "9e 3e 1 FV-FROM-STACK 8e 1 FV-FROM-STACK AITKEN"), [8.0, 0.0]);
    // sin on 9 points: the error estimate is of the size of the true error.
    f.interpret(
        "9 FV-NEW CONSTANT SX  9 FV-NEW CONSTANT SY
         : FILL-SIN  9 0 DO I S>F 0.25e F* FDUP I SX FV[] F! FSIN I SY FV[] F! LOOP ;
         FILL-SIN",
    )
    .unwrap();
    let r = floats(&mut f, "0.9e SX SY AITKEN");
    let true_error = (r[0] - 0.9f64.sin()).abs();
    assert!(true_error < 1e-8, "{r:?}");
    assert!(r[1] >= true_error * 0.1 && r[1] < 1e-5, "{r:?}");
    // Rational interpolation of 1/(1+x^2) at the nodes -2 .. 2 is exact: the function is rational.
    f.interpret(
        "-2e -1e 0e 1e 2e 5 FV-FROM-STACK CONSTANT RX
         0.2e 0.5e 1e 0.5e 0.2e 5 FV-FROM-STACK CONSTANT RY",
    )
    .unwrap();
    for x in [-1.5f64, 0.3, 0.9, 3.0] {
        let r = floats(&mut f, &format!("{x:e} RX RY RATIONAL-INTERP"));
        near(r[0], 1.0 / (1.0 + x * x), 1e-12);
    }
    assert_eq!(floats(&mut f, "1e RX RY RATIONAL-INTERP"), [0.5, 0.0]);
    // Equal nodes, and vectors of different lengths.
    f.interpret("1e 1e 2e 3 FV-FROM-STACK CONSTANT BADX  1e 2e 3e 3 FV-FROM-STACK CONSTANT BADY")
        .unwrap();
    assert_eq!(code(&mut f, "0.5e BADX BADY LAGRANGE"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "BADX BADY NEWTON-DD"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "0.5e BADX BADY AITKEN"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "0.5e XS RY LAGRANGE"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "0.5e 0 FV-NEW 0 FV-NEW LAGRANGE"), SCIENCE_ERROR);
}

#[test]
fn root_finders() {
    let mut f = roots();
    let sqrt2 = std::f64::consts::SQRT_2;
    near(floats(&mut f, "0e 2e 1e-14 ' SQ2 BISECT")[0], sqrt2, 1e-13);
    near(floats(&mut f, "2e 0e 1e-14 ' SQ2 BISECT")[0], sqrt2, 1e-13);
    near(floats(&mut f, "0e 2e 1e-14 ' SQ2 REGULA-FALSI")[0], sqrt2, 1e-13);
    near(floats(&mut f, "2e 0e 1e-14 ' SQ2 REGULA-FALSI")[0], sqrt2, 1e-13);
    near(floats(&mut f, "1e 1e-14 ' SQ2 ' DSQ2 NEWTON")[0], sqrt2, 1e-14);
    near(floats(&mut f, "-3e 1e-14 ' SQ2 ' DSQ2 NEWTON")[0], -sqrt2, 1e-14);
    // The fixed point of cos, the Dottie number.
    let dottie = 0.739_085_133_215_160_6;
    for word in ["0e 1e 1e-14 ' COSMX BISECT", "0e 1e 1e-14 ' COSMX REGULA-FALSI"] {
        near(floats(&mut f, word)[0], dottie, 1e-13);
    }
    near(floats(&mut f, "1e 1e-14 ' COSMX ' DCOSMX NEWTON")[0], dottie, 1e-14);
    // A root at an end is found at once.
    assert_eq!(floats(&mut f, "1e 3e 1e-9 ' CUBEM BISECT"), [1.0]);
    assert_eq!(floats(&mut f, "-3e 1e 1e-9 ' CUBEM REGULA-FALSI"), [1.0]);
    // A steep function (x^64 = 1): the Illinois rule still gets there; x^1024 = 1 needs
    // hundreds of steps, and the word says so instead of answering too early.
    near(floats(&mut f, "0e 1.5e 1e-12 ' STEEP REGULA-FALSI")[0], 1.0, 1e-9);
    assert_eq!(code(&mut f, "0e 1.5e 1e-12 ' STEEPER REGULA-FALSI"), SCIENCE_ERROR);
    near(floats(&mut f, "0e 1.5e 1e-12 ' STEEPER BISECT")[0], 1.0, 1e-9);
    // No sign change, a NaN, a derivative of 0.
    assert_eq!(code(&mut f, "0e 1e 1e-9 ' FLAT BISECT"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "0e 1e 1e-9 ' FLAT REGULA-FALSI"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "0e 1e 1e-9 ' NAN BISECT"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "0e 1e 1e-9 ' NAN REGULA-FALSI"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "0e 1e-9 ' SQ2 ' ZEROD NEWTON"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "0e 1e-9 ' NAN ' DSQ2 NEWTON"), SCIENCE_ERROR);
    // A tolerance of 0 ends too: bisection when the midpoint reaches an end.
    near(floats(&mut f, "0e 2e 0e ' SQ2 BISECT")[0], sqrt2, 1e-15);
    // The function is called a modest number of times.
    f.interpret("0e SEEN F!").unwrap();
    f.interpret("0e 2e 1e-14 ' COUNTING REGULA-FALSI FDROP").unwrap();
    let calls = floats(&mut f, "SEEN F@")[0];
    assert!(calls < 20.0, "{calls} calls");
    // Nothing stays on the heap: many rounds fit in the default memory.
    f.interpret(
        ": ROOTS-ROUND  0e 2e 1e-14 ['] SQ2 BISECT FDROP  0e 2e 1e-14 ['] SQ2 REGULA-FALSI FDROP
            1e 1e-14 ['] SQ2 ['] DSQ2 NEWTON FDROP ;
         : MANY  3000 0 DO ROOTS-ROUND LOOP ;
         MANY",
    )
    .unwrap();
}

/// Two floats from the float stack: a complex number is the real part, then the imaginary part.
fn complex(f: &mut Forth, text: &str) -> (f64, f64) {
    let v = floats(f, text);
    assert_eq!(v.len(), 2, "{text}: {v:?}");
    (v[0], v[1])
}

fn assert_complex(got: (f64, f64), want: (f64, f64), tol: f64) {
    assert!(
        (got.0 - want.0).abs() <= tol && (got.1 - want.1).abs() <= tol,
        "{got:?} is not {want:?}"
    );
}

fn assert_all_near_abs(got: &[f64], want: &[f64], tol: f64) {
    assert_eq!(got.len(), want.len(), "{got:?} and {want:?}");
    for (g, w) in got.iter().zip(want) {
        assert!((g - w).abs() <= tol, "{got:?} is not {want:?}");
    }
}

#[test]
fn complex_arithmetic() {
    let mut f = forth();
    assert_eq!(complex(&mut f, "1e 2e 3e 4e Z+"), (4.0, 6.0));
    assert_eq!(complex(&mut f, "1e 2e 3e 4e Z-"), (-2.0, -2.0));
    assert_eq!(complex(&mut f, "1e 2e 3e 4e Z*"), (-5.0, 10.0));
    assert_complex(complex(&mut f, "1e 2e 3e 4e Z/"), (0.44, 0.08), 1e-15);
    assert_eq!(complex(&mut f, "1e 2e ZNEGATE"), (-1.0, -2.0));
    assert_eq!(complex(&mut f, "1e 2e ZCONJG"), (1.0, -2.0));
    assert_eq!(floats(&mut f, "3e 4e ZABS"), [5.0]);
    near(floats(&mut f, "-1e 0e ZARG")[0], std::f64::consts::PI, 1e-15);
    // Division does not overflow where the plain formula would.
    assert_complex(complex(&mut f, "1e300 1e300 2e300 2e300 Z/"), (0.5, 0.0), 1e-15);
    // i * i = -1.
    assert_eq!(complex(&mut f, "0e 1e 0e 1e Z*"), (-1.0, 0.0));
}

#[test]
fn complex_functions_match_the_values_from_python() {
    let mut f = forth();
    assert_complex(complex(&mut f, "1e 2e ZSIN"), (3.165778513216168, 1.959601041421606), 1e-14);
    assert_complex(complex(&mut f, "1e 2e ZCOS"), (2.0327230070196656, -3.0518977991517997), 1e-14);
    assert_complex(
        complex(&mut f, "1e 2e ZSINH"),
        (-0.4890562590412937, 1.4031192506220405),
        1e-14,
    );
    assert_complex(complex(&mut f, "1e 2e ZCOSH"), (-0.64214812471552, 1.0686074213827783), 1e-14);
    assert_complex(complex(&mut f, "-1e 1e ZLN"), (0.34657359027997264, 2.356194490192345), 1e-14);
    assert_complex(complex(&mut f, "-3e -4e ZSQRT"), (1.0, -2.0), 1e-14);
    assert_complex(complex(&mut f, "-4e 0e ZSQRT"), (0.0, 2.0), 1e-15);
    assert_complex(complex(&mut f, "0e 0e ZSQRT"), (0.0, 0.0), 0.0);
    // Euler: e^(i pi) = -1; the polar form is the inverse of ZABS and ZARG.
    assert_complex(complex(&mut f, "0e 3.141592653589793e ZEXP"), (-1.0, 0.0), 1e-15);
    assert_complex(
        complex(&mut f, "2e 0.5e POLAR>Z"),
        (2.0 * 0.5f64.cos(), 2.0 * 0.5f64.sin()),
        1e-15,
    );
    // (1+2i)^(0.5+1i) and the whole powers.
    assert_complex(
        complex(&mut f, "1e 2e 0.5e 1e Z**"),
        (0.1042330549168707, 0.48309593926990946),
        1e-14,
    );
    assert_eq!(complex(&mut f, "2e 3e 5 Z^N"), (122.0, -597.0));
    assert_complex(complex(&mut f, "1e 1e -2 Z^N"), (0.0, -0.5), 1e-16);
    assert_eq!(complex(&mut f, "2e 3e 0 Z^N"), (1.0, 0.0));
    assert_eq!(code(&mut f, "0e 0e 0e 1e Z**"), SCIENCE_ERROR);
    assert_eq!(complex(&mut f, "0e 0e 2e 0e Z**"), (0.0, 0.0));
    assert_eq!(code(&mut f, "FCLEAR 1e Z+"), -45);
}

/// The vector pair of a complex signal.
const SIGNAL: &str = "
1e 2e 3e 4e 0.5e -1e 2e 7e 8 FV-FROM-STACK CONSTANT RE
0e 1e -1e 2e 0.25e 3e 1e 0e 8 FV-FROM-STACK CONSTANT IM
";

#[test]
fn fft_matches_the_definition_of_the_transform() {
    let mut f = forth();
    f.interpret(SIGNAL).unwrap();
    // The values come from the sum over e^(-2 pi i k n / N) in Python.
    let re =
        [18.5, 2.742640687119, -1.5, -1.742640687119, -5.5, -5.742640687119, -5.5, 6.742640687119];
    let im = [
        6.25,
        -4.078427124746,
        10.25,
        3.578427124746,
        -5.75,
        1.578427124746,
        -9.75,
        -2.078427124746,
    ];
    // DFT makes new vectors, FFT changes the vectors in place, so it gets copies.
    f.interpret("RE IM DFT CONSTANT DIM CONSTANT DRE").unwrap();
    assert_all_near_abs(&floats(&mut f, "DRE FV@S"), &re, 1e-11);
    assert_all_near_abs(&floats(&mut f, "DIM FV@S"), &im, 1e-11);
    f.interpret("RE 1e FV-SCALE CONSTANT R2  IM 1e FV-SCALE CONSTANT I2  R2 I2 FFT").unwrap();
    assert_all_near_abs(&floats(&mut f, "R2 FV@S"), &re, 1e-11);
    assert_all_near_abs(&floats(&mut f, "I2 FV@S"), &im, 1e-11);
}

#[test]
fn fft_and_its_inverse_give_the_signal_back() {
    let mut f = forth();
    f.interpret(SIGNAL).unwrap();
    f.interpret("RE 1e FV-SCALE CONSTANT R2  IM 1e FV-SCALE CONSTANT I2").unwrap();
    f.interpret("R2 I2 FFT  R2 I2 IFFT").unwrap();
    let want_re = floats(&mut f, "RE FV@S");
    let want_im = floats(&mut f, "IM FV@S");
    assert_all_near_abs(&floats(&mut f, "R2 FV@S"), &want_re, 1e-14);
    assert_all_near_abs(&floats(&mut f, "I2 FV@S"), &want_im, 1e-14);
    // Not a power of 2, or two lengths, or empty: -264. DFT takes any length but 0.
    assert_eq!(code(&mut f, "6 FV-NEW 6 FV-NEW FFT"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "8 FV-NEW 4 FV-NEW FFT"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "0 FV-NEW 0 FV-NEW FFT"), SCIENCE_ERROR);
    assert_eq!(code(&mut f, "0 FV-NEW 0 FV-NEW DFT"), SCIENCE_ERROR);
    f.interpret("1e 2e 3e 5e 7e 5 FV-FROM-STACK 5 FV-NEW DFT IDFT SWAP CONSTANT BACK").unwrap();
    assert_all_near_abs(&floats(&mut f, "BACK FV@S"), &[1.0, 2.0, 3.0, 5.0, 7.0], 1e-13);
    // One point: the transform is the identity.
    f.interpret("4e 1 FV-FROM-STACK 1 FV-NEW 2DUP FFT").unwrap();
    assert_eq!(floats(&mut f, "DROP FV@S"), [4.0]);
}

#[test]
fn fft_finds_a_frequency() {
    let mut f = forth();
    // 64 samples of cos(2 pi 5 n / 64): a peak of height 32 at 5 and at 59, nothing else.
    f.interpret(
        ": WAVE ( -- re im ) 64 FV-NEW 64 FV-NEW  {: re im :}
            64 0 DO I S>F 5e F* 3.141592653589793e F* 32e F/ FCOS I re FV[] F! LOOP  re im ;",
    )
    .unwrap();
    f.interpret("WAVE 2DUP FFT ZV-ABS CONSTANT SPEC").unwrap();
    let spec = floats(&mut f, "SPEC FV@S");
    for (k, v) in spec.iter().enumerate() {
        let want = if k == 5 || k == 59 { 32.0 } else { 0.0 };
        assert!((v - want).abs() < 1e-12, "bin {k}: {v}");
    }
}

#[test]
fn the_work_of_a_transform_is_charged() {
    let mut f = forth();
    // 2^14 points: about 10^6 operations, charged as work, within the default limit.
    f.interpret("16384 FV-NEW 16384 FV-NEW FFT").unwrap();
    let mut g = Forth::sandboxed();
    g.install(&Science);
    g.set_limits(Limits { steps: Some(10_000), ..Limits::default() });
    g.interpret("ALSO SCIENCE 16384 FV-NEW 16384 FV-NEW").unwrap();
    let e = g.interpret("FFT").unwrap_err();
    assert_eq!(e.code, forthling::LIMIT_EXCEEDED, "{e}");
}

#[test]
fn complex_roots_of_the_quadratic_and_the_cubic() {
    let mut f = forth();
    // x^2 - 3x + 2: both roots real, ascending.
    assert_eq!(floats(&mut f, "1e -3e 2e ZQUADRATIC"), [1.0, 0.0, 2.0, 0.0]);
    // x^2 + 1: -i and i, real part exactly 0.
    assert_eq!(floats(&mut f, "1e 0e 1e ZQUADRATIC"), [0.0, -1.0, 0.0, 1.0]);
    // 2x^2 + 2x + 5: -1/2 +- 3/2 i, also with the leading coefficient negative.
    assert_all_near_abs(&floats(&mut f, "2e 2e 5e ZQUADRATIC"), &[-0.5, -1.5, -0.5, 1.5], 1e-15);
    assert_all_near_abs(&floats(&mut f, "-2e -2e -5e ZQUADRATIC"), &[-0.5, -1.5, -0.5, 1.5], 1e-15);
    assert_eq!(code(&mut f, "0e 1e 1e ZQUADRATIC"), SCIENCE_ERROR);
    // x^3 - 1: 1 and -1/2 +- sqrt(3)/2 i.
    let s = 3f64.sqrt() / 2.0;
    assert_all_near_abs(
        &floats(&mut f, "1e 0e 0e -1e ZCUBIC"),
        &[-0.5, -s, -0.5, s, 1.0, 0.0],
        1e-15,
    );
    // Three real roots: x^3 - 6x^2 + 11x - 6.
    assert_all_near_abs(
        &floats(&mut f, "1e -6e 11e -6e ZCUBIC"),
        &[1.0, 0.0, 2.0, 0.0, 3.0, 0.0],
        1e-13,
    );
    // 2x^3 - 3x^2 + 4x - 5: every root is a root of the polynomial.
    f.interpret("-5e 4e -3e 2e 4 FV-FROM-STACK CONSTANT CR  4 FV-NEW CONSTANT CI").unwrap();
    let r = floats(&mut f, "2e -3e 4e -5e ZCUBIC");
    assert_eq!(r.len(), 6);
    for z in r.chunks(2) {
        let p = complex(&mut f, &format!("{:e} {:e} CR CI ZPOLY-EVAL", z[0], z[1]));
        assert_complex(p, (0.0, 0.0), 1e-13);
    }
    assert_eq!(code(&mut f, "0e 1e 1e 1e ZCUBIC"), SCIENCE_ERROR);
}

#[test]
fn all_the_roots_of_a_polynomial() {
    let mut f = forth();
    // x^5 + 1: the roots exp(i pi (2k+1) / 5), sorted by real and then imaginary part.
    f.interpret("1e 0e 0e 0e 0e 1e 6 FV-FROM-STACK POLY-ROOTS CONSTANT IMS CONSTANT RES").unwrap();
    let re = floats(&mut f, "RES FV@S");
    let im = floats(&mut f, "IMS FV@S");
    let mut want: Vec<(f64, f64)> = (0..5)
        .map(|k| {
            let t = std::f64::consts::PI * f64::from(2 * k + 1) / 5.0;
            (t.cos(), t.sin())
        })
        .collect();
    want.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    assert_eq!(re.len(), 5);
    for i in 0..5 {
        assert_complex((re[i], im[i]), want[i], 1e-13);
    }
    // x^2 + 1: -i and i, found from the real start.
    f.interpret("1e 0e 1e 3 FV-FROM-STACK POLY-ROOTS CONSTANT IMS CONSTANT RES").unwrap();
    assert_all_near_abs(&floats(&mut f, "RES FV@S"), &[0.0, 0.0], 1e-15);
    assert_all_near_abs(&floats(&mut f, "IMS FV@S"), &[-1.0, 1.0], 1e-15);
    // (x-1)(x-2)(x-3)(x-4) has four real roots, and the imaginary parts are 0.
    f.interpret("24e -50e 35e -10e 1e 5 FV-FROM-STACK POLY-ROOTS CONSTANT IMS CONSTANT RES")
        .unwrap();
    assert_all_near_abs(&floats(&mut f, "RES FV@S"), &[1.0, 2.0, 3.0, 4.0], 1e-10);
    assert_all_near_abs(&floats(&mut f, "IMS FV@S"), &[0.0; 4], 1e-10);
    // A multiple root loses digits, as it must: (x-1)^3.
    f.interpret("-1e 3e -3e 1e 4 FV-FROM-STACK POLY-ROOTS CONSTANT IMS CONSTANT RES").unwrap();
    assert_all_near_abs(&floats(&mut f, "RES FV@S"), &[1.0, 1.0, 1.0], 1e-4);
    // A constant has no roots; the zero polynomial is an error.
    f.interpret("5e 1 FV-FROM-STACK POLY-ROOTS CONSTANT IMS CONSTANT RES").unwrap();
    assert_eq!(floats(&mut f, "RES FV-LEN S>F"), [0.0]);
    assert_eq!(code(&mut f, "0e 0e 2 FV-FROM-STACK POLY-ROOTS"), SCIENCE_ERROR);
}

#[test]
fn complex_coefficients() {
    let mut f = forth();
    // (x - i)(x - 2) = x^2 - (2 + i) x + 2i: real parts (0, -2, 1), imaginary parts (2, -1, 0).
    f.interpret("0e -2e 1e 3 FV-FROM-STACK CONSTANT CR  2e -1e 0e 3 FV-FROM-STACK CONSTANT CI")
        .unwrap();
    f.interpret("CR CI ZPOLY-ROOTS CONSTANT IMS CONSTANT RES").unwrap();
    assert_all_near_abs(&floats(&mut f, "RES FV@S"), &[0.0, 2.0], 1e-14);
    assert_all_near_abs(&floats(&mut f, "IMS FV@S"), &[1.0, 0.0], 1e-14);
    // One root near a start; the value at 1+i is (1+i)^2 - (2+i)(1+i) + 2i = -1 + i.
    assert_complex(complex(&mut f, "0.1e 0.9e 1e-14 CR CI ZPOLY-LAGUERRE"), (0.0, 1.0), 1e-13);
    assert_complex(complex(&mut f, "1e 1e CR CI ZPOLY-EVAL"), (-1.0, 1.0), 1e-14);
    // Too few coefficients, or two lengths.
    assert_eq!(
        code(&mut f, "0e 0e 1e-9 1e 1 FV-FROM-STACK 0e 1 FV-FROM-STACK ZPOLY-LAGUERRE"),
        SCIENCE_ERROR
    );
    assert_eq!(code(&mut f, "0e 0e 1e-9 2 FV-NEW 3 FV-NEW ZPOLY-LAGUERRE"), SCIENCE_ERROR);
}

#[test]
fn polynomial_roots_are_charged() {
    let mut g = Forth::sandboxed();
    g.install(&Science);
    g.set_limits(Limits { steps: Some(1000), ..Limits::default() });
    g.interpret("ALSO SCIENCE 200 FV-NEW CONSTANT BIG").unwrap();
    let e = g.interpret("BIG POLY-ROOTS").unwrap_err();
    assert_eq!(e.code, forthling::LIMIT_EXCEEDED, "{e}");
}
