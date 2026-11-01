//! The FACILITY word set; the structure words and the key constants are in
//! prelude.fs.

#![allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "the clock and a sleep time are checked non-negative and far below the limits"
)]

use crate::*;
use std::time::Duration;

pub(crate) fn install(f: &mut Forth) {
    f.prim("KEY?", |f| {
        if f.interactive {
            f.flush();
        }
        let ready = f.input.ready();
        f.push(flag(ready));
        Ok(())
    });
    f.prim("MS", |f| {
        let ms = f.pop()?;
        if f.interactive {
            f.flush();
        }
        f.sleep(Duration::from_millis(ms.max(0) as u64))
    });
    f.prim("TIME&DATE", |f| {
        let secs = f.now() as i64;
        let (year, month, day) = civil_from_days(secs.div_euclid(86_400));
        let time = secs.rem_euclid(86_400);
        for v in [time % 60, time / 60 % 60, time / 3600, day, month, year] {
            f.push(v);
        }
        Ok(())
    });
    // Gforth: the time in microseconds, and random numbers (splitmix64).
    f.prim("UTIME", |f| {
        let t = f.now_micros();
        f.push_ud(u128::from(t));
        Ok(())
    });
    // There is no CPU clock in the sandbox: the user time is the time since the
    // system began, the system time is 0.
    f.prim("CPUTIME", |f| {
        let t = f.now_micros().saturating_sub(f.started);
        f.push_ud(u128::from(t));
        f.push_ud(0);
        Ok(())
    });
    f.prim("RND", |f| {
        let x = f.next_random();
        f.push(x as Cell);
        Ok(())
    });
    f.prim("RANDOM", |f| {
        let n = f.pop()? as u64;
        let x = f.next_random();
        f.push(((u128::from(x) * u128::from(n)) >> 64) as Cell);
        Ok(())
    });
    f.prim("SEED!", |f| {
        let x = f.pop()?;
        f.rng = Some(x as u64);
        Ok(())
    });
    // AT-XY and PAGE write ANSI escape sequences.
    f.prim("AT-XY", |f| {
        let (col, row) = f.pop2()?;
        f.emit_str(&format!("\x1b[{};{}H", row.max(0) + 1, col.max(0) + 1));
        Ok(())
    });
    f.prim("PAGE", |f| {
        f.emit_str("\x1b[2J\x1b[H");
        Ok(())
    });
}

/// The date (year, month, day) of a number of days since 1970-01-01, in the
/// proleptic Gregorian calendar (Howard Hinnant's `civil_from_days`).
#[allow(clippy::similar_names, reason = "the names of Hinnant's algorithm")]
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097); // day of the 400-year era
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of the year, from March
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

impl Forth {
    /// The next number of the generator behind RND and RANDOM (splitmix64),
    /// seeded from the clock the first time.
    fn next_random(&mut self) -> u64 {
        let state = self.rng.unwrap_or_else(|| self.now_micros() ^ 0x9E37_79B9_7F4A_7C15);
        let state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        self.rng = Some(state);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

#[cfg(test)]
mod tests {
    use super::civil_from_days;

    #[test]
    fn calendar() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(11_017), (2000, 3, 1));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
        assert_eq!(civil_from_days(20_756), (2026, 10, 30));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
    }
}
