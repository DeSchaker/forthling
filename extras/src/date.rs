//! The word list DATE: dates and times as a number of seconds since
//! 1970-01-01 00:00:00 UTC, in the proleptic Gregorian calendar, for the
//! years 0 to 9999 (the four digits of ISO 8601). UTC only: a zone offset is
//! understood when reading and then gone. The words never read a clock; the
//! number of seconds comes from the program (`TIME&DATE DATE>SECONDS`).

use crate::{EXTRAS_ERROR, word};
use forthling::{Forth, Res, Throw};

const LIST: &str = "DATE";

/// 0000-01-01T00:00:00Z and 9999-12-31T23:59:59Z.
const MIN_SECONDS: i64 = -62_167_219_200;
const MAX_SECONDS: i64 = 253_402_300_799;
const DAY: i64 = 86_400;

fn error(what: &str) -> Throw {
    Throw::with(EXTRAS_ERROR, what.to_string())
}

fn is_leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// The day number (0 is 1970-01-01) of a civil date. The month and day must be valid.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    // Years run from March, so that the leap day is the last day of the year.
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let era_year = y - era * 400;
    let year_day = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let era_day = era_year * 365 + era_year / 4 - era_year / 100 + year_day;
    era * 146_097 + era_day - 719_468
}

/// The civil date (year, month, day) of a day number.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let era_day = z - era * 146_097;
    let era_year = (era_day - era_day / 1460 + era_day / 36_524 - era_day / 146_096) / 365;
    let year_day = era_day - (365 * era_year + era_year / 4 - era_year / 100);
    let mp = (5 * year_day + 2) / 153;
    let day = year_day - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (era_year + era * 400 + i64::from(month <= 2), month, day)
}

fn check_range(seconds: i64) -> Res<i64> {
    if (MIN_SECONDS..=MAX_SECONDS).contains(&seconds) {
        Ok(seconds)
    } else {
        Err(error("a time outside the years 0 to 9999"))
    }
}

/// The seconds of a date and time; every part is checked.
fn to_seconds(year: i64, month: i64, day: i64, hour: i64, minute: i64, second: i64) -> Res<i64> {
    if !(0..=9999).contains(&year)
        || !(1..=12).contains(&month)
        || !(1..=days_in_month(year, month)).contains(&day)
        || !(0..24).contains(&hour)
        || !(0..60).contains(&minute)
        || !(0..60).contains(&second)
    {
        return Err(error("a date or time that does not exist"));
    }
    Ok(days_from_civil(year, month, day) * DAY + hour * 3600 + minute * 60 + second)
}

/// (year, month, day, hour, minute, second) of a number of seconds.
fn from_seconds(seconds: i64) -> Res<[i64; 6]> {
    let seconds = check_range(seconds)?;
    let (year, month, day) = civil_from_days(seconds.div_euclid(DAY));
    let rest = seconds.rem_euclid(DAY);
    Ok([year, month, day, rest / 3600, rest % 3600 / 60, rest % 60])
}

fn format(seconds: i64) -> Res<String> {
    let [y, mo, d, h, mi, s] = from_seconds(seconds)?;
    Ok(format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z"))
}

/// A fixed number of digits at the front of the text.
fn digits(text: &mut &[u8], n: usize) -> Res<i64> {
    if text.len() < n || !text[..n].iter().all(u8::is_ascii_digit) {
        return Err(error("text that is not ISO 8601"));
    }
    let (head, rest) = text.split_at(n);
    *text = rest;
    Ok(head.iter().fold(0, |v, &c| v * 10 + i64::from(c - b'0')))
}

fn expect(text: &mut &[u8], c: u8) -> Res<()> {
    match text.split_first() {
        Some((&first, rest)) if first == c => {
            *text = rest;
            Ok(())
        }
        _ => Err(error("text that is not ISO 8601")),
    }
}

/// `YYYY-MM-DD`, or `YYYY-MM-DDTHH:MM:SS` followed by `Z`, a zone offset
/// `+HH:MM` or `-HH:MM`, or nothing (UTC). The result is in UTC.
fn parse(text: &[u8]) -> Res<i64> {
    let mut t = text;
    let year = digits(&mut t, 4)?;
    expect(&mut t, b'-')?;
    let month = digits(&mut t, 2)?;
    expect(&mut t, b'-')?;
    let day = digits(&mut t, 2)?;
    if t.is_empty() {
        return to_seconds(year, month, day, 0, 0, 0);
    }
    expect(&mut t, b'T')?;
    let hour = digits(&mut t, 2)?;
    expect(&mut t, b':')?;
    let minute = digits(&mut t, 2)?;
    expect(&mut t, b':')?;
    let second = digits(&mut t, 2)?;
    let local = to_seconds(year, month, day, hour, minute, second)?;
    let offset = match t.split_first() {
        None | Some((b'Z', [])) => 0,
        Some((&sign @ (b'+' | b'-'), rest)) => {
            t = rest;
            let (h, m) = (digits(&mut t, 2)?, {
                expect(&mut t, b':')?;
                digits(&mut t, 2)?
            });
            if !t.is_empty() || h > 23 || m > 59 {
                return Err(error("text that is not ISO 8601"));
            }
            let offset = h * 3600 + m * 60;
            if sign == b'-' { -offset } else { offset }
        }
        _ => return Err(error("text that is not ISO 8601")),
    };
    check_range(local - offset)
}

fn add_months(seconds: i64, months: i64) -> Res<i64> {
    let [year, month, day, h, mi, s] = from_seconds(seconds)?;
    let index = months
        .checked_add(year * 12 + month - 1)
        .ok_or_else(|| error("a time outside the years 0 to 9999"))?;
    let (year, month) = (index.div_euclid(12), index.rem_euclid(12) + 1);
    // A day that the new month does not have becomes its last day.
    let day = day.min(if (0..=9999).contains(&year) { days_in_month(year, month) } else { 31 });
    to_seconds(year, month, day, h, mi, s)
}

fn add_days(seconds: i64, days: i64) -> Res<i64> {
    let delta = days.checked_mul(DAY).ok_or_else(|| error("a time outside the years 0 to 9999"))?;
    check_range(
        seconds.checked_add(delta).ok_or_else(|| error("a time outside the years 0 to 9999"))?,
    )
}

pub fn install(f: &mut Forth) {
    word(f, LIST, "DATE>SECONDS", |f| {
        // The order of TIME&DATE: ( sec min hour day month year ).
        let year = f.pop()?;
        let month = f.pop()?;
        let day = f.pop()?;
        let hour = f.pop()?;
        let minute = f.pop()?;
        let second = f.pop()?;
        f.push(to_seconds(year, month, day, hour, minute, second)?);
        Ok(())
    });
    word(f, LIST, "SECONDS>DATE", |f| {
        let seconds = f.pop()?;
        let [year, month, day, hour, minute, second] = from_seconds(seconds)?;
        for v in [second, minute, hour, day, month, year] {
            f.push(v);
        }
        Ok(())
    });
    word(f, LIST, ">ISO8601", |f| {
        let seconds = f.pop()?;
        f.push_bytes(format(seconds)?.as_bytes())
    });
    word(f, LIST, "ISO8601>", |f| {
        let text = f.pop_bytes()?;
        f.charge(text.len())?;
        f.push(parse(&text)?);
        Ok(())
    });
    word(f, LIST, "DAY-OF-WEEK", |f| {
        let seconds = check_range(f.pop()?)?;
        // 1970-01-01 was a Thursday (4); Monday is 1 and Sunday 7.
        f.push((seconds.div_euclid(DAY) + 3).rem_euclid(7) + 1);
        Ok(())
    });
    word(f, LIST, "+DAYS", |f| {
        let days = f.pop()?;
        let seconds = f.pop()?;
        f.push(add_days(check_range(seconds)?, days)?);
        Ok(())
    });
    word(f, LIST, "+MONTHS", |f| {
        let months = f.pop()?;
        let seconds = f.pop()?;
        f.push(add_months(seconds, months)?);
        Ok(())
    });
    word(f, LIST, "LEAP-YEAR?", |f| {
        let year = f.pop()?;
        f.push(if is_leap(year) { -1 } else { 0 });
        Ok(())
    });
    word(f, LIST, "DAYS-IN-MONTH", |f| {
        let month = f.pop()?;
        let year = f.pop()?;
        if !(1..=12).contains(&month) {
            return Err(error("a month that does not exist"));
        }
        f.push(days_in_month(year, month));
        Ok(())
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_conversions_round_trip() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
        for days in (-719_528..2_932_897).step_by(997) {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
        assert_eq!(MIN_SECONDS, days_from_civil(0, 1, 1) * DAY);
        assert_eq!(MAX_SECONDS, days_from_civil(9999, 12, 31) * DAY + DAY - 1);
    }

    #[test]
    fn leap_years() {
        assert!(is_leap(2000) && is_leap(2024) && !is_leap(1900) && !is_leap(2023));
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2100, 2), 28);
    }

    #[test]
    fn iso_round_trip_and_offsets() {
        assert_eq!(format(0).unwrap(), "1970-01-01T00:00:00Z");
        assert_eq!(format(1_000_000_000).unwrap(), "2001-09-09T01:46:40Z");
        assert_eq!(parse(b"2001-09-09T01:46:40Z").unwrap(), 1_000_000_000);
        assert_eq!(parse(b"2001-09-09T03:46:40+02:00").unwrap(), 1_000_000_000);
        assert_eq!(parse(b"2001-09-08T20:46:40-05:00").unwrap(), 1_000_000_000);
        assert_eq!(parse(b"2001-09-09T01:46:40").unwrap(), 1_000_000_000);
        assert_eq!(parse(b"2001-09-09").unwrap(), 999_993_600);
        assert_eq!(format(MIN_SECONDS).unwrap(), "0000-01-01T00:00:00Z");
        assert_eq!(format(MAX_SECONDS).unwrap(), "9999-12-31T23:59:59Z");
        assert_eq!(parse(b"0000-01-01T00:00:00Z").unwrap(), MIN_SECONDS);
    }

    #[test]
    fn bad_text_is_refused() {
        for text in [
            "",
            "2001",
            "2001-9-09",
            "2001-13-01",
            "2001-02-29",
            "2001-09-09T24:00:00Z",
            "2001-09-09T01:46Z",
            "2001-09-09 01:46:40Z",
            "2001-09-09T01:46:40z",
            "2001-09-09T01:46:40+0200",
            "2001-09-09T01:46:40Zx",
            "2001-09-09T01:46:40.5Z",
            "0000-01-01T00:00:00+01:00",
            "+2001-09-09",
            "2001-09-09T01:46:40+24:00",
        ] {
            assert!(parse(text.as_bytes()).is_err(), "{text}");
        }
    }

    #[test]
    fn months_clamp_to_the_end() {
        let jan31 = to_seconds(2023, 1, 31, 12, 0, 0).unwrap();
        assert_eq!(format(add_months(jan31, 1).unwrap()).unwrap(), "2023-02-28T12:00:00Z");
        assert_eq!(format(add_months(jan31, 13).unwrap()).unwrap(), "2024-02-29T12:00:00Z");
        assert_eq!(format(add_months(jan31, -2).unwrap()).unwrap(), "2022-11-30T12:00:00Z");
        assert!(add_months(jan31, 100_000).is_err());
        assert!(add_months(jan31, i64::MAX).is_err());
        assert!(add_months(MIN_SECONDS, -1).is_err());
    }
}
