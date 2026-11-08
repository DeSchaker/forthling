//! The DATE words, used from Forth.

use forthling::{Cell, Forth, Limits};
use forthling_extras::{EXTRAS_ERROR, Extras};

fn forth() -> Forth {
    let mut f = Forth::new();
    f.install(&Extras);
    f.interpret("ALSO DATE").unwrap();
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
fn iso_text_both_ways() {
    let mut f = forth();
    f.interpret(SHOW).unwrap();
    assert_eq!(run(&mut f, "0 >ISO8601 SHOW"), "1970-01-01T00:00:00Z");
    assert_eq!(run(&mut f, "-1 >ISO8601 SHOW"), "1969-12-31T23:59:59Z");
    assert_eq!(run(&mut f, "1000000000 >ISO8601 SHOW"), "2001-09-09T01:46:40Z");
    assert_eq!(run(&mut f, r#"S" 2001-09-09T01:46:40Z" ISO8601> ."#), "1000000000 ");
    assert_eq!(run(&mut f, r#"S" 2001-09-09T03:46:40+02:00" ISO8601> ."#), "1000000000 ");
    assert_eq!(run(&mut f, r#"S" 2001-09-09" ISO8601> ."#), "999993600 ");
    assert_eq!(run(&mut f, r#"S" 1969-12-31T23:59:59" ISO8601> ."#), "-1 ");
}

#[test]
fn the_parts_in_the_order_of_time_and_date() {
    let mut f = forth();
    assert_eq!(run(&mut f, "1000000000 SECONDS>DATE . . . . . ."), "2001 9 9 1 46 40 ");
    assert_eq!(run(&mut f, "40 46 1 9 9 2001 DATE>SECONDS ."), "1000000000 ");
    // TIME&DATE gives the parts in this order, so the two meet.
    assert_eq!(run(&mut f, "TIME&DATE DATE>SECONDS 1700000000 > ."), "-1 ");
}

#[test]
fn leap_days_and_the_edges() {
    let mut f = forth();
    f.interpret(SHOW).unwrap();
    assert_eq!(run(&mut f, "0 0 0 29 2 2024 DATE>SECONDS >ISO8601 SHOW"), "2024-02-29T00:00:00Z");
    assert_eq!(code(&mut f, "0 0 0 29 2 2023 DATE>SECONDS"), EXTRAS_ERROR);
    assert_eq!(code(&mut f, "0 0 0 29 2 1900 DATE>SECONDS"), EXTRAS_ERROR);
    assert_eq!(run(&mut f, "0 0 0 29 2 2000 DATE>SECONDS >ISO8601 SHOW"), "2000-02-29T00:00:00Z");
    assert_eq!(
        run(&mut f, "59 59 23 31 12 9999 DATE>SECONDS DUP . >ISO8601 SHOW"),
        "253402300799 9999-12-31T23:59:59Z"
    );
    assert_eq!(
        run(&mut f, "0 0 0 1 1 0 DATE>SECONDS DUP . >ISO8601 SHOW"),
        "-62167219200 0000-01-01T00:00:00Z"
    );
    assert_eq!(code(&mut f, "253402300800 >ISO8601"), EXTRAS_ERROR);
    assert_eq!(code(&mut f, "-62167219201 SECONDS>DATE"), EXTRAS_ERROR);
    for bad in [
        "0 0 24 1 1 2000",
        "60 0 0 1 1 2000",
        "0 60 0 1 1 2000",
        "0 0 0 0 1 2000",
        "0 0 0 1 13 2000",
        "0 0 0 1 0 2000",
        "0 0 0 1 1 10000",
        "0 0 0 1 1 -1",
    ] {
        assert_eq!(code(&mut f, &format!("{bad} DATE>SECONDS")), EXTRAS_ERROR, "{bad}");
    }
}

#[test]
fn day_of_week_is_iso() {
    let mut f = forth();
    // 1970-01-01 was a Thursday, 2001-09-09 a Sunday, 2024-02-29 a Thursday, 0001-01-01 a Monday.
    assert_eq!(run(&mut f, "0 DAY-OF-WEEK ."), "4 ");
    assert_eq!(run(&mut f, "1000000000 DAY-OF-WEEK ."), "7 ");
    assert_eq!(run(&mut f, "-86400 DAY-OF-WEEK ."), "3 ");
    assert_eq!(run(&mut f, r#"S" 2024-02-29" ISO8601> DAY-OF-WEEK ."#), "4 ");
    assert_eq!(run(&mut f, r#"S" 0001-01-01" ISO8601> DAY-OF-WEEK ."#), "1 ");
    assert_eq!(run(&mut f, r#"S" 2026-11-09" ISO8601> DAY-OF-WEEK ."#), "1 ");
}

#[test]
fn adding_days_and_months() {
    let mut f = forth();
    f.interpret(SHOW).unwrap();
    let day = |f: &mut Forth, text: &str| run(f, &format!("{text} >ISO8601 SHOW"));
    assert_eq!(day(&mut f, r#"S" 2024-02-28" ISO8601> 2 +DAYS"#), "2024-03-01T00:00:00Z");
    assert_eq!(day(&mut f, r#"S" 2024-03-01" ISO8601> -1 +DAYS"#), "2024-02-29T00:00:00Z");
    assert_eq!(
        day(&mut f, r#"S" 2023-01-31T12:00:00Z" ISO8601> 1 +MONTHS"#),
        "2023-02-28T12:00:00Z"
    );
    assert_eq!(day(&mut f, r#"S" 2023-12-15" ISO8601> 2 +MONTHS"#), "2024-02-15T00:00:00Z");
    assert_eq!(day(&mut f, r#"S" 2024-01-15" ISO8601> -1 +MONTHS"#), "2023-12-15T00:00:00Z");
    assert_eq!(code(&mut f, r#"S" 9999-12-31" ISO8601> 1 +DAYS"#), EXTRAS_ERROR);
    assert_eq!(code(&mut f, r#"S" 9999-12-31" ISO8601> 1 +MONTHS"#), EXTRAS_ERROR);
    assert_eq!(code(&mut f, "0 9223372036854775807 +DAYS"), EXTRAS_ERROR);
    assert_eq!(code(&mut f, "0 9223372036854775807 +MONTHS"), EXTRAS_ERROR);
}

#[test]
fn leap_years_and_month_lengths() {
    let mut f = forth();
    assert_eq!(
        run(&mut f, "1900 LEAP-YEAR? . 2000 LEAP-YEAR? . 2023 LEAP-YEAR? . 2024 LEAP-YEAR? ."),
        "0 -1 0 -1 "
    );
    assert_eq!(
        run(
            &mut f,
            "2024 2 DAYS-IN-MONTH . 2023 2 DAYS-IN-MONTH . 2023 4 DAYS-IN-MONTH . 2023 12 DAYS-IN-MONTH ."
        ),
        "29 28 30 31 "
    );
    assert_eq!(code(&mut f, "2023 13 DAYS-IN-MONTH"), EXTRAS_ERROR);
    assert_eq!(code(&mut f, "2023 0 DAYS-IN-MONTH"), EXTRAS_ERROR);
}

#[test]
fn bad_text_throws() {
    let mut f = forth();
    for bad in [
        "",
        "2001",
        "2001-02-30",
        "2001-09-09T25:00:00Z",
        "2001-09-09T01:46:40+2:00",
        "2001-09-09T01:46:40.5Z",
        "9999-12-31T23:59:59-01:00",
    ] {
        assert_eq!(code(&mut f, &format!("S\" {bad}\" ISO8601>")), EXTRAS_ERROR, "{bad:?}");
    }
}

#[test]
fn the_work_is_counted_and_the_stack_is_checked() {
    let mut f = Forth::sandboxed();
    f.install(&Extras);
    f.set_limits(Limits { steps: Some(1000), ..Limits::default() });
    f.interpret("ALSO DATE").unwrap();
    f.interpret("CREATE T 100000 ALLOT").unwrap();
    let e = f.interpret("T 100000 ISO8601>").unwrap_err();
    assert_eq!(e.code, forthling::LIMIT_EXCEEDED, "{e}");
    let mut f = forth();
    assert_eq!(code(&mut f, "1 2 3 DATE>SECONDS"), -4);
}

#[test]
fn every_word_has_a_help_line_and_its_examples_work() {
    let mut f = Forth::new();
    f.install(&Extras);
    let problems = f.help_problems();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    let examples: Vec<_> = f.help_examples().into_iter().filter(|e| e.list == "DATE").collect();
    assert!(examples.len() >= 9);
    for e in examples {
        let mut f = Forth::new();
        f.install(&Extras);
        f.interpret(&e.code).unwrap();
        let out = String::from_utf8_lossy(&f.take_output()).trim().to_string();
        assert_eq!(out, e.output, "{}: {}", e.name, e.code);
    }
}
