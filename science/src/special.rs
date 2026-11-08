//! Special functions: gamma and log-gamma, factorial, the error function and
//! its complement, the incomplete gamma and beta functions, and the logistic
//! function with its derivative. Scalars on the float stack.
//!
//! The methods are the textbook ones, built from the mathematics: the Stirling
//! series (with a shift up by the recurrence) for the gamma function, a series
//! and a continued fraction (evaluated with the modified Lentz method) for the
//! incomplete gamma function, and the continued fraction of Euler for the
//! incomplete beta function. The error function is the incomplete gamma
//! function with a = 1/2.

use std::f64::consts::PI;

use forthling::{Cell, Forth, Res, Throw};

use crate::{SCIENCE_ERROR, word, work};

/// The standard throw for a floating point argument that is not valid.
const INVALID_FLOAT: Cell = -46;
/// The standard throw for a floating point result that is out of range.
const FLOAT_RANGE: Cell = -43;
/// The standard throw for a number that is not valid as an argument.
const INVALID_ARGUMENT: Cell = -24;

/// ln(sqrt(2 pi)).
const LN_SQRT_2PI: f64 = 0.918_938_533_204_672_7;
/// Above this x the gamma function does not fit in a float.
const GAMMA_MAX: f64 = 171.624_376_956_302_7;
/// The relative accuracy that the series and the fractions run to.
const EPS: f64 = 1e-16;
/// Stops a fraction or a series that does not converge.
const MAX_ITERATIONS: usize = 200_000;
const TINY: f64 = 1e-300;

fn invalid(what: &str) -> Throw {
    Throw::with(INVALID_FLOAT, format!("{what}: the argument is not valid"))
}

fn out_of_range(what: &str) -> Throw {
    Throw::with(FLOAT_RANGE, format!("{what}: the result does not fit in a float"))
}

/// The sine of pi x, exact at the integers and accurate close to them.
fn sin_pi(x: f64) -> f64 {
    let mut r = x - 2.0 * (x / 2.0).floor(); // 0 <= r < 2
    let mut sign = 1.0;
    if r >= 1.0 {
        r -= 1.0;
        sign = -1.0;
    }
    if r > 0.5 {
        r = 1.0 - r;
    }
    sign * (PI * r).sin()
}

/// The gamma function for x >= 15, from the Stirling series. The power is
/// split in two halves so that it does not overflow before the end.
fn gamma_stirling(x: f64) -> f64 {
    let t = x.powf(0.5 * x - 0.25) * (-0.5 * x).exp();
    t * t * LN_SQRT_2PI.exp() * stirling_series(x).exp()
}

/// The correction of the Stirling series: sum of B(2k) / (2k (2k-1) x^(2k-1)).
fn stirling_series(x: f64) -> f64 {
    let inv = 1.0 / x;
    let inv2 = inv * inv;
    inv * (1.0 / 12.0
        - inv2
            * (1.0 / 360.0
                - inv2
                    * (1.0 / 1260.0
                        - inv2
                            * (1.0 / 1680.0
                                - inv2
                                    * (1.0 / 1188.0 - inv2 * (691.0 / 360_360.0 - inv2 / 156.0))))))
}

/// The gamma function for 0.5 <= x <= `GAMMA_MAX`, by shifting x up to 15
/// with Gamma(x + 1) = x Gamma(x).
fn gamma_shifted(x: f64) -> f64 {
    let mut y = x;
    let mut product = 1.0;
    while y < 15.0 {
        product *= y;
        y += 1.0;
    }
    gamma_stirling(y) / product
}

/// The product 1 * 2 * ... * (n - 1), which is Gamma(n), for a whole n >= 1.
fn gamma_integer(n: f64) -> f64 {
    let mut p = 1.0;
    let mut k = 2.0;
    while k < n {
        p *= k;
        k += 1.0;
    }
    p
}

fn gamma(x: f64) -> Res<f64> {
    if x.is_nan() || (x <= 0.0 && x.fract() == 0.0) {
        return Err(invalid("GAMMA"));
    }
    if x.fract() == 0.0 {
        return if x <= 171.0 { Ok(gamma_integer(x)) } else { Err(out_of_range("GAMMA")) };
    }
    if x > GAMMA_MAX {
        return Err(out_of_range("GAMMA"));
    }
    if x < 0.5 {
        // The reflection formula: Gamma(x) Gamma(1 - x) = pi / sin(pi x).
        let g = 1.0 - x;
        return Ok(if g > GAMMA_MAX { 0.0 } else { PI / (sin_pi(x) * gamma_shifted(g)) });
    }
    Ok(gamma_shifted(x))
}

fn ln_gamma(x: f64) -> Res<f64> {
    if x.is_nan() || (x <= 0.0 && x.fract() == 0.0) {
        return Err(invalid("LNGAMMA"));
    }
    if x < 0.5 {
        let g = 1.0 - x;
        return Ok((PI / sin_pi(x).abs()).ln() - ln_gamma(g)?);
    }
    if x.fract() == 0.0 && x <= 171.0 {
        return Ok(gamma_integer(x).ln());
    }
    if x < 15.0 {
        return Ok(gamma_shifted(x).ln());
    }
    let r = (x - 0.5) * x.ln() - x + LN_SQRT_2PI + stirling_series(x);
    if r.is_finite() { Ok(r) } else { Err(out_of_range("LNGAMMA")) }
}

/// Both P(a, x) and Q(a, x) = 1 - P(a, x), the regularized incomplete gamma
/// functions. The series is used below a + 1, the fraction above it.
fn gamma_pq(f: &mut Forth, a: f64, x: f64, what: &str) -> Res<(f64, f64)> {
    if a.is_nan() || x.is_nan() || a <= 0.0 || x < 0.0 {
        return Err(invalid(what));
    }
    if x == 0.0 {
        return Ok((0.0, 1.0));
    }
    if x.is_infinite() {
        return Ok((1.0, 0.0));
    }
    let prefix = (a * x.ln() - x - ln_gamma(a)?).exp();
    let mut steps = 0;
    let result = if x < a + 1.0 {
        let mut ap = a;
        let mut term = 1.0 / a;
        let mut sum = term;
        loop {
            steps += 1;
            ap += 1.0;
            term *= x / ap;
            sum += term;
            if term.abs() < sum.abs() * EPS {
                break;
            }
            if steps > MAX_ITERATIONS {
                return Err(no_convergence(what));
            }
        }
        let p = (sum * prefix).min(1.0);
        (p, 1.0 - p)
    } else {
        let mut b = x + 1.0 - a;
        let mut c = 1.0 / TINY;
        let mut d = 1.0 / b;
        let mut h = d;
        loop {
            steps += 1;
            let n = f64::from(u32::try_from(steps).unwrap_or(u32::MAX));
            let an = -n * (n - a);
            b += 2.0;
            d = an * d + b;
            if d.abs() < TINY {
                d = TINY;
            }
            c = b + an / c;
            if c.abs() < TINY {
                c = TINY;
            }
            d = 1.0 / d;
            let delta = d * c;
            h *= delta;
            if (delta - 1.0).abs() < EPS {
                break;
            }
            if steps > MAX_ITERATIONS {
                return Err(no_convergence(what));
            }
        }
        let q = (prefix * h).min(1.0);
        (1.0 - q, q)
    };
    work(f, steps.saturating_mul(8))?;
    Ok(result)
}

fn no_convergence(what: &str) -> Throw {
    Throw::with(SCIENCE_ERROR, format!("{what}: the calculation did not converge"))
}

/// erf(x) and erfc(x), through the incomplete gamma function with a = 1/2.
fn erf_erfc(f: &mut Forth, x: f64) -> Res<(f64, f64)> {
    if x.is_nan() {
        return Err(invalid("ERF"));
    }
    let (p, q) = gamma_pq(f, 0.5, x * x, "ERF")?;
    Ok(if x < 0.0 { (-p, 1.0 + p) } else { (p, q) })
}

/// The continued fraction of the incomplete beta function (modified Lentz).
fn beta_fraction(a: f64, b: f64, x: f64, what: &str) -> Res<(f64, usize)> {
    let sum_ab = a + b;
    let mut c = 1.0;
    let mut d = 1.0 - sum_ab * x / (a + 1.0);
    if d.abs() < TINY {
        d = TINY;
    }
    d = 1.0 / d;
    let mut h = d;
    let mut m = 1.0_f64;
    for steps in 1..=MAX_ITERATIONS {
        let m2 = 2.0 * m;
        let even = m * (b - m) * x / ((a - 1.0 + m2) * (a + m2));
        d = 1.0 + even * d;
        if d.abs() < TINY {
            d = TINY;
        }
        c = 1.0 + even / c;
        if c.abs() < TINY {
            c = TINY;
        }
        d = 1.0 / d;
        h *= d * c;
        let odd = -(a + m) * (sum_ab + m) * x / ((a + m2) * (a + 1.0 + m2));
        d = 1.0 + odd * d;
        if d.abs() < TINY {
            d = TINY;
        }
        c = 1.0 + odd / c;
        if c.abs() < TINY {
            c = TINY;
        }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if (delta - 1.0).abs() < EPS {
            return Ok((h, steps));
        }
        m += 1.0;
    }
    Err(no_convergence(what))
}

/// The regularized incomplete beta function I(x; a, b).
fn beta_inc(f: &mut Forth, a: f64, b: f64, x: f64) -> Res<f64> {
    let what = "BETA-I";
    if a.is_nan() || b.is_nan() || x.is_nan() || a <= 0.0 || b <= 0.0 || !(0.0..=1.0).contains(&x) {
        return Err(invalid(what));
    }
    if x <= 0.0 {
        return Ok(0.0);
    }
    if x >= 1.0 {
        return Ok(1.0);
    }
    let prefix =
        (ln_gamma(a + b)? - ln_gamma(a)? - ln_gamma(b)? + a * x.ln() + b * (1.0 - x).ln()).exp();
    let (r, steps) = if x < (a + 1.0) / (a + b + 2.0) {
        let (h, steps) = beta_fraction(a, b, x, what)?;
        (prefix * h / a, steps)
    } else {
        let (h, steps) = beta_fraction(b, a, 1.0 - x, what)?;
        (1.0 - prefix * h / b, steps)
    };
    work(f, steps.saturating_mul(16))?;
    Ok(r.clamp(0.0, 1.0))
}

/// The logistic function 1 / (1 + e^-x), without overflow for any x.
fn logistic(x: f64) -> f64 {
    if x >= 0.0 {
        1.0 / (1.0 + (-x).exp())
    } else {
        let e = x.exp();
        e / (1.0 + e)
    }
}

/// The derivative of the logistic function, s (1 - s).
fn logistic_derivative(x: f64) -> f64 {
    let e = (-x.abs()).exp();
    e / ((1.0 + e) * (1.0 + e))
}

/// A word that takes one float and leaves one.
fn unary(f: &mut Forth, name: &'static str, op: impl Fn(f64) -> Res<f64> + 'static) {
    word(f, name, move |f| {
        let x = f.fpop()?;
        f.fpush(op(x)?)
    });
}

pub fn install(f: &mut Forth) {
    // GAMMA ( F: x -- r ): the gamma function; -46 at 0 and the negative
    // integers, -43 above 171.6.
    unary(f, "GAMMA", gamma);
    // LNGAMMA ( F: x -- r ): the natural logarithm of the absolute value of
    // the gamma function.
    unary(f, "LNGAMMA", ln_gamma);
    // FACTORIAL ( n -- ) ( F: -- r ): n! for 0 <= n <= 170.
    word(f, "FACTORIAL", |f| {
        let n = f.pop()?;
        if n < 0 {
            return Err(Throw::new(INVALID_ARGUMENT));
        }
        if n > 170 {
            return Err(out_of_range("FACTORIAL"));
        }
        let mut p = 1.0;
        for k in 2..=n {
            p *= f64::from(i32::try_from(k).expect("k is at most 170"));
        }
        f.fpush(p)
    });
    // ERF ( F: x -- r ) and ERFC ( F: x -- r ): the error function and 1 - erf.
    word(f, "ERF", |f| {
        let x = f.fpop()?;
        let (erf, _) = erf_erfc(f, x)?;
        f.fpush(erf)
    });
    word(f, "ERFC", |f| {
        let x = f.fpop()?;
        let (_, erfc) = erf_erfc(f, x)?;
        f.fpush(erfc)
    });
    // GAMMA-P ( F: a x -- r ) and GAMMA-Q ( F: a x -- r ): the regularized
    // incomplete gamma function and its complement 1 - P.
    word(f, "GAMMA-P", |f| {
        let x = f.fpop()?;
        let a = f.fpop()?;
        let (p, _) = gamma_pq(f, a, x, "GAMMA-P")?;
        f.fpush(p)
    });
    word(f, "GAMMA-Q", |f| {
        let x = f.fpop()?;
        let a = f.fpop()?;
        let (_, q) = gamma_pq(f, a, x, "GAMMA-Q")?;
        f.fpush(q)
    });
    // BETA ( F: a b -- r ): the beta function, Gamma(a) Gamma(b) / Gamma(a+b).
    word(f, "BETA", |f| {
        let b = f.fpop()?;
        let a = f.fpop()?;
        if a.is_nan() || b.is_nan() || a <= 0.0 || b <= 0.0 {
            return Err(invalid("BETA"));
        }
        f.fpush((ln_gamma(a)? + ln_gamma(b)? - ln_gamma(a + b)?).exp())
    });
    // BETA-I ( F: a b x -- r ): the regularized incomplete beta function.
    word(f, "BETA-I", |f| {
        let x = f.fpop()?;
        let b = f.fpop()?;
        let a = f.fpop()?;
        let r = beta_inc(f, a, b, x)?;
        f.fpush(r)
    });
    // LOGISTIC ( F: x -- r ) and LOGISTIC-D ( F: x -- r ): 1 / (1 + e^-x) and
    // its derivative.
    unary(f, "LOGISTIC", |x| Ok(logistic(x)));
    unary(f, "LOGISTIC-D", |x| Ok(logistic_derivative(x)));
}
