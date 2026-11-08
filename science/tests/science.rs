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
