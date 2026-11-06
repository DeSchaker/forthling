//! The FIXED words, used from Forth.

use forthling::{Cell, Fixed, Forth};
use std::io::Cursor;

fn forth() -> Forth {
    let mut f = Forth::new();
    f.set_input(Box::new(Cursor::new(Vec::new())));
    f.install(&Fixed);
    f.interpret("ALSO FIXED").unwrap();
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

#[test]
fn decimal_arithmetic_is_exact() {
    let mut f = forth();
    run(&mut f, "2 FX-DECIMALS");
    assert_eq!(run(&mut f, "0.10fx 0.20fx FX+ FX."), "0.30 ");
    assert_eq!(run(&mut f, "0.10fx 0.20fx FX+ 0.30fx = ."), "-1 ");
    assert_eq!(run(&mut f, "19.99fx 3 S>FX FX* FX."), "59.97 ");
    assert_eq!(run(&mut f, "10.00fx 3.00fx FX/ FX."), "3.33 ");
    assert_eq!(run(&mut f, "-1.50fx FXABS FX. 1.5fx FXNEGATE FX."), "1.50 -1.50 ");
    assert_eq!(run(&mut f, "FX-SCALE ."), "100 ");
}

#[test]
fn rounding_modes() {
    let mut f = forth();
    run(&mut f, "2 FX-DECIMALS");
    // 0.05 * 0.50 = 0.025 and 0.05 * -0.50 = -0.025: a tie on the second decimal
    assert_eq!(run(&mut f, "0.05fx 0.50fx FX* FX. 0.05fx -0.50fx FX* FX."), "0.03 -0.03 ");
    run(&mut f, "FX-FLOOR");
    assert_eq!(run(&mut f, "0.05fx 0.50fx FX* FX. 0.05fx -0.50fx FX* FX."), "0.02 -0.03 ");
    run(&mut f, "FX-TRUNC");
    assert_eq!(run(&mut f, "0.05fx 0.50fx FX* FX. 0.05fx -0.50fx FX* FX."), "0.02 -0.02 ");
    run(&mut f, "FX-NEAREST");
    assert_eq!(run(&mut f, "2.50fx FX>S . -2.50fx FX>S . 2.49fx FX>S ."), "3 -3 2 ");
    run(&mut f, "FX-FLOOR");
    assert_eq!(run(&mut f, "2.50fx FX>S . -2.50fx FX>S ."), "2 -3 ");
}

#[test]
fn a_decimal_literal_must_fit_exactly() {
    let mut f = forth();
    run(&mut f, "2 FX-DECIMALS");
    assert_eq!(
        run(&mut f, "1.500fx FX. 1.5fx FX. 3fx FX. .25fx FX. +7.1fx FX."),
        "1.50 1.50 3.00 0.25 7.10 "
    );
    assert_eq!(code(&mut f, "0.001fx"), -24);
    assert_eq!(code(&mut f, "1.005fx"), -24);
    // an error inside a definition
    assert_eq!(code(&mut f, ": X 0.001fx ;"), -24);
}

#[test]
fn nine_decimals_at_the_start() {
    let mut f = forth();
    assert_eq!(run(&mut f, "FX-SCALE ."), "1000000000 ");
    assert_eq!(run(&mut f, "0.1fx 0.2fx FX+ 0.3fx = . 0.1fx 10 S>FX FX* 1fx = ."), "-1 -1 ");
    assert_eq!(run(&mut f, "0.1fx 0.2fx FX+ FX."), "0.300000000 ");
    assert_eq!(code(&mut f, "0.0000000001fx"), -24);
}

#[test]
fn binary_scales() {
    let mut f = forth();
    run(&mut f, "16 FX-BITS");
    assert_eq!(run(&mut f, "FX-SCALE . 1.5fx . -0.25fx ."), "65536 98304 -16384 ");
    assert_eq!(run(&mut f, "1.5fx 2.5fx FX* FX."), "3.75000 ");
    assert_eq!(run(&mut f, "1.5fx 2fx FX/ FX."), "0.75000 ");
    assert_eq!(run(&mut f, "2fx FXSQRT FX."), "1.41422 ");
    assert_eq!(run(&mut f, "9fx FXSQRT FX."), "3.00000 ");
    // 0.1 cannot be exact in binary: an error, unless rounding is allowed
    assert_eq!(code(&mut f, "0.1fx"), -24);
    assert_eq!(code(&mut f, ": X 0.1fx ;"), -24);
    run(&mut f, "TRUE FX-ROUND-LITERALS");
    assert_eq!(run(&mut f, "0.1fx . -0.1fx ."), "6554 -6554 ");
    run(&mut f, "FALSE FX-ROUND-LITERALS");
    assert_eq!(code(&mut f, "0.1fx"), -24);
    run(&mut f, "0 FX-BITS");
    assert_eq!(code(&mut f, "2.5fx"), -24);
    run(&mut f, "TRUE FX-ROUND-LITERALS");
    assert_eq!(run(&mut f, "7 S>FX . 2.5fx . 7fx FX."), "7 3 7 ");
}

#[test]
fn q32_32_is_the_most() {
    let mut f = forth();
    run(&mut f, "32 FX-BITS");
    assert_eq!(run(&mut f, "1fx . 1.5fx FX."), "4294967296 1.5000000000 ");
    assert_eq!(run(&mut f, "0.5fx 0.5fx FX* FX."), "0.2500000000 ");
    assert_eq!(code(&mut f, "33 FX-BITS"), -24);
    assert_eq!(code(&mut f, "-1 FX-BITS"), -24);
    assert_eq!(code(&mut f, "19 FX-DECIMALS"), -24);
    // the largest number: just under 2^31
    assert_eq!(code(&mut f, "2147483648fx"), -11);
    assert_eq!(code(&mut f, "2147483647fx 1fx FX+"), -11);
}

#[test]
fn rounding_literals_is_a_choice() {
    let mut f = forth();
    run(&mut f, "2 FX-DECIMALS");
    assert_eq!(code(&mut f, "1.005fx"), -24);
    run(&mut f, "TRUE FX-ROUND-LITERALS");
    assert_eq!(run(&mut f, "1.005fx FX. -1.005fx FX. 1.0049fx FX."), "1.01 -1.01 1.00 ");
    run(&mut f, "FX-FLOOR");
    assert_eq!(run(&mut f, "1.005fx FX. -1.005fx FX."), "1.00 -1.01 ");
    run(&mut f, "FX-NEAREST FALSE FX-ROUND-LITERALS");
    assert_eq!(code(&mut f, "1.005fx"), -24);
    // the message says what to do
    let e = f.interpret("1.005fx").unwrap_err();
    assert!(e.to_string().contains("FX-ROUND-LITERALS"), "{e}");
}

#[test]
fn overflow_throws_and_never_wraps() {
    let mut f = forth();
    // nine decimals hold up to about 9.2 billion
    assert_eq!(code(&mut f, "5000000000 S>FX 2 S>FX FX*"), -11);
    assert_eq!(code(&mut f, "5000000000 S>FX 5000000000 S>FX FX+"), -11);
    assert_eq!(code(&mut f, "-5000000000 S>FX -5000000000 S>FX FX+"), -11);
    assert_eq!(code(&mut f, "10000000000 S>FX"), -11);
    assert_eq!(code(&mut f, "1fx FX-"), -4);
    assert_eq!(code(&mut f, "1fx 0 FX/"), -10);
    assert_eq!(code(&mut f, "-1fx FXSQRT"), -24);
    assert_eq!(code(&mut f, "1e100 F>FX"), -11);
}

#[test]
fn conversions() {
    let mut f = forth();
    run(&mut f, "3 FX-DECIMALS");
    assert_eq!(run(&mut f, "2.5E F>FX FX. 0.1E F>FX FX."), "2.500 0.100 ");
    assert_eq!(run(&mut f, "1.25fx FX>F F. "), "1.25 ");
    assert_eq!(run(&mut f, "-7 S>FX FX. -7 S>FX FX>S ."), "-7.000 -7 ");
    assert_eq!(run(&mut f, r#"S" 12.345" >FX . ."#), "-1 12345 ");
    assert_eq!(run(&mut f, r#"S" 12.3456" >FX . . S" abc" >FX . ."#), "0 0 0 0 ");
}

#[test]
fn text_of_a_number() {
    let mut f = forth();
    run(&mut f, "2 FX-DECIMALS");
    assert_eq!(run(&mut f, "-0.05fx FX>STRING TYPE"), "-0.05");
    assert_eq!(run(&mut f, "0.00fx FX>STRING TYPE"), "0.00");
    assert_eq!(run(&mut f, "-123456.78fx FX>STRING TYPE"), "-123456.78");
    run(&mut f, "0 FX-DECIMALS");
    assert_eq!(run(&mut f, "42fx FX."), "42 ");
}

#[test]
fn literals_need_the_word_list() {
    let mut f = Forth::new();
    f.set_input(Box::new(Cursor::new(Vec::new())));
    f.install(&Fixed);
    assert_eq!(code(&mut f, "1.5fx"), -13);
    f.interpret("ALSO FIXED").unwrap();
    f.interpret("2 FX-DECIMALS 1.5fx").unwrap();
    f.interpret("PREVIOUS DROP").unwrap();
    assert_eq!(code(&mut f, "1.5fx"), -13);
    // other words are untouched
    assert_eq!(run(&mut f, "1 2 + . 1E0 F. ALSO FIXED 2fx FX."), "3 1. 2.00 ");
    assert_eq!(code(&mut f, "fx"), -13);
    assert_eq!(code(&mut f, "xfx"), -13);
}

#[test]
fn compiled_and_environment_query() {
    let mut f = forth();
    run(&mut f, "2 FX-DECIMALS");
    run(&mut f, ": PRICE 19.99fx ; : TOTAL ( n -- fx ) S>FX PRICE FX* ;");
    assert_eq!(run(&mut f, "3 TOTAL FX."), "59.97 ");
    assert_eq!(run(&mut f, r#"S" FORTHLING-FIXED" ENVIRONMENT? . ."#), "-1 1 ");
}

#[test]
fn long_literals() {
    let mut f = forth();
    // many digits are an error, unless rounding is allowed
    assert_eq!(code(&mut f, "0.1234567890123456789012345678901234567890fx"), -24);
    run(&mut f, "TRUE FX-ROUND-LITERALS");
    // many digits after a binary scale: rounded; a tie goes away from zero
    assert_eq!(
        run(
            &mut f,
            "32 FX-BITS 0.1234567890123456789012345678901234567890fx 0 FX-BITS 0.5fx . -0.5fx ."
        ),
        "1 -1 "
    );
    assert_eq!(code(&mut f, "99999999999999999999fx"), -11);
    run(&mut f, "18 FX-DECIMALS");
    assert_eq!(run(&mut f, "1.000000000000000001fx FX."), "1.000000000000000001 ");
    assert_eq!(run(&mut f, "0.1234567890123456785fx FX."), "0.123456789012345679 ");
    assert_eq!(run(&mut f, "0.1234567890123456784999999999fx FX."), "0.123456789012345678 ");
    assert_eq!(code(&mut f, "10fx"), -11);
}
