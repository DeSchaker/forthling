//! Polynomials, orthogonal polynomials, closed-form roots and interpolation.
//! A polynomial is a float vector of coefficients, the constant term first.
//!
//! Built from the mathematics: Horner's rule, the three-term recurrences of the
//! orthogonal polynomials, the stable form of the quadratic formula,
//! Cardano's and the trigonometric solution of the cubic (with a Newton step to
//! polish), Laguerre's iteration, Neville's scheme (Aitken's repeated linear
//! interpolation), Newton's divided differences and the Bulirsch-Stoer
//! recurrence for rational interpolation.

use forthling::{Cell, Forth, Res, Throw};

use crate::{Mat, TAG_VEC, error, load_vec, pop_size, store, word, work};

/// The most steps of Laguerre's iteration.
const MAX_ITERATIONS: usize = 100;

#[allow(clippy::cast_precision_loss, reason = "degrees and counts are far below 2^53")]
fn float(n: usize) -> f64 {
    n as f64
}

/// Horner's rule: the value and the derivative at x.
fn horner(c: &[f64], x: f64) -> (f64, f64) {
    let mut p = 0.0;
    let mut d = 0.0;
    for &a in c.iter().rev() {
        d = d * x + p;
        p = p * x + a;
    }
    (p, d)
}

/// The value, the first and the second derivative at x.
fn horner2(c: &[f64], x: f64) -> (f64, f64, f64) {
    let mut p = 0.0;
    let mut d = 0.0;
    let mut e = 0.0;
    for &a in c.iter().rev() {
        e = e * x + 2.0 * d;
        d = d * x + p;
        p = p * x + a;
    }
    (p, d, e)
}

fn vector(v: Vec<f64>) -> Mat {
    Mat { rows: v.len(), cols: 1, v }
}

/// Pops a vector and checks that it has at least `least` elements.
fn load_len(f: &mut Forth, what: &str, least: usize) -> Res<Vec<f64>> {
    let m = load_vec(f)?;
    if m.v.len() < least {
        return Err(error(format!("{what}: the vector needs at least {least} elements")));
    }
    Ok(m.v)
}

/// The values of a three-term recurrence: `next(k, x, p_k, p_(k-1))` gives p_(k+1).
fn recurrence(n: usize, x: f64, p1: f64, next: impl Fn(f64, f64, f64, f64) -> f64) -> f64 {
    if n == 0 {
        return 1.0;
    }
    let mut prev = 1.0;
    let mut cur = p1;
    for k in 1..n {
        let new = next(float(k), x, cur, prev);
        prev = cur;
        cur = new;
    }
    cur
}

/// An orthogonal polynomial word: `( n -- ) ( F: x -- r )`.
fn orthogonal(
    f: &mut Forth,
    name: &'static str,
    first: fn(f64) -> f64,
    next: fn(f64, f64, f64, f64) -> f64,
) {
    word(f, name, move |f| {
        let x = f.fpop()?;
        let n = pop_size(f)?;
        work(f, n.saturating_mul(4))?;
        f.fpush(recurrence(n, x, first(x), next))
    });
}

fn install_orthogonal(f: &mut Forth) {
    // CHEBYSHEV-T ( n -- ) ( F: x -- r ): T0 = 1, T1 = x, T(k+1) = 2 x Tk - T(k-1).
    orthogonal(f, "CHEBYSHEV-T", |x| x, |_, x, p, q| 2.0 * x * p - q);
    // CHEBYSHEV-U ( n -- ) ( F: x -- r ): U0 = 1, U1 = 2 x, the same recurrence.
    orthogonal(f, "CHEBYSHEV-U", |x| 2.0 * x, |_, x, p, q| 2.0 * x * p - q);
    // LEGENDRE ( n -- ) ( F: x -- r ): (k+1) P(k+1) = (2k+1) x Pk - k P(k-1).
    orthogonal(f, "LEGENDRE", |x| x, |k, x, p, q| ((2.0 * k + 1.0) * x * p - k * q) / (k + 1.0));
    // HERMITE ( n -- ) ( F: x -- r ): the physicists' H, H(k+1) = 2 x Hk - 2 k H(k-1).
    orthogonal(f, "HERMITE", |x| 2.0 * x, |k, x, p, q| 2.0 * x * p - 2.0 * k * q);
    // LAGUERRE ( n -- ) ( F: x -- r ): (k+1) L(k+1) = (2k+1-x) Lk - k L(k-1).
    orthogonal(
        f,
        "LAGUERRE",
        |x| 1.0 - x,
        |k, x, p, q| ((2.0 * k + 1.0 - x) * p - k * q) / (k + 1.0),
    );
}

/// The real roots of a x^2 + b x + c, in ascending order.
fn quadratic(a: f64, b: f64, c: f64) -> Vec<f64> {
    if a == 0.0 {
        return if b == 0.0 { vec![] } else { vec![-c / b] };
    }
    let disc = b * b - 4.0 * a * c;
    if disc.is_nan() || disc < 0.0 {
        return vec![];
    }
    // q has the sign of b, so that no digits cancel.
    let q = -0.5 * (b + disc.sqrt().copysign(b));
    let (x1, x2) = if q == 0.0 { (0.0, 0.0) } else { (q / a, c / q) };
    vec![x1.min(x2), x1.max(x2)]
}

/// The real roots of a x^3 + b x^2 + c x + d, in ascending order.
pub(crate) fn cubic(a: f64, b: f64, c: f64, d: f64) -> Vec<f64> {
    if a == 0.0 {
        return quadratic(b, c, d);
    }
    let (b, c, d) = (b / a, c / a, d / a);
    // x = t - b/3 gives t^3 + p t + q.
    let p = c - b * b / 3.0;
    let q = 2.0 * b * b * b / 27.0 - b * c / 3.0 + d;
    let shift = -b / 3.0;
    let disc = (q / 2.0).powi(2) + (p / 3.0).powi(3);
    let mut roots: Vec<f64> = if disc > 0.0 {
        let s = disc.sqrt();
        vec![(-q / 2.0 + s).cbrt() + (-q / 2.0 - s).cbrt() + shift]
    } else if p == 0.0 {
        vec![shift; 3]
    } else {
        let m = 2.0 * (-p / 3.0).sqrt();
        let arg = (3.0 * q / (p * m)).clamp(-1.0, 1.0);
        let theta = arg.acos() / 3.0;
        let third = 2.0 * std::f64::consts::PI / 3.0;
        (0..3).map(|k| m * (theta - third * float(k)).cos() + shift).collect()
    };
    for r in &mut roots {
        // One Newton step on the polynomial itself removes the last rounding.
        let (v, dv) = horner(&[d, c, b, 1.0], *r);
        if dv != 0.0 && v.is_finite() {
            let step = v / dv;
            if step.abs() < 1e-6 * r.abs().max(1.0) {
                *r -= step;
            }
        }
    }
    roots.sort_by(f64::total_cmp);
    roots
}

fn install_closed_forms(f: &mut Forth) {
    // QUADRATIC ( F: a b c -- x1 .. xn ) ( -- n ): the real roots of a x^2 + b x + c
    // in ascending order, n = 0 or 2 (a double root is given twice), or n = 1 for a
    // straight line (a = 0) and n = 0 when there is no root.
    word(f, "QUADRATIC", |f| {
        let c = f.fpop()?;
        let b = f.fpop()?;
        let a = f.fpop()?;
        let roots = quadratic(a, b, c);
        for &r in &roots {
            f.fpush(r)?;
        }
        f.push(Cell::try_from(roots.len()).expect("at most 2"));
        Ok(())
    });
    // CUBIC ( F: a b c d -- x1 .. xn ) ( -- n ): the real roots of a x^3 + b x^2 + c x + d
    // in ascending order: 1 or 3 of them (a multiple root is given as often as it counts);
    // a = 0 makes it a quadratic.
    word(f, "CUBIC", |f| {
        let d = f.fpop()?;
        let c = f.fpop()?;
        let b = f.fpop()?;
        let a = f.fpop()?;
        let roots = cubic(a, b, c, d);
        for &r in &roots {
            f.fpush(r)?;
        }
        f.push(Cell::try_from(roots.len()).expect("at most 3"));
        Ok(())
    });
}

/// A root of the polynomial near `x`, by Laguerre's iteration with real
/// numbers: `None` when a step would be complex or the iteration does not settle.
fn laguerre(c: &[f64], mut x: f64, tol: f64) -> Option<f64> {
    let n = float(c.len() - 1);
    for _ in 0..MAX_ITERATIONS {
        let (p, d, e) = horner2(c, x);
        if p == 0.0 {
            return Some(x);
        }
        let g = d / p;
        let h = g * g - e / p;
        let disc = (n - 1.0) * (n * h - g * g);
        if disc.is_nan() || disc < 0.0 {
            return None;
        }
        let root = disc.sqrt();
        let denom = if (g + root).abs() >= (g - root).abs() { g + root } else { g - root };
        if denom == 0.0 {
            return None;
        }
        let step = n / denom;
        x -= step;
        if step.abs() <= tol * x.abs().max(1.0) {
            return Some(x);
        }
    }
    None
}

fn install_polynomials(f: &mut Forth) {
    // POLY-EVAL ( c -- ) ( F: x -- r ): Horner's rule.
    word(f, "POLY-EVAL", |f| {
        let c = load_len(f, "POLY-EVAL", 1)?;
        let x = f.fpop()?;
        f.fpush(horner(&c, x).0)
    });
    // POLY-EVAL-D ( c -- ) ( F: x -- p dp ): the value and the derivative.
    word(f, "POLY-EVAL-D", |f| {
        let c = load_len(f, "POLY-EVAL-D", 1)?;
        let x = f.fpop()?;
        let (p, d) = horner(&c, x);
        f.fpush(p)?;
        f.fpush(d)
    });
    // POLY-DERIV ( c -- d ): the derivative polynomial, one coefficient shorter.
    word(f, "POLY-DERIV", |f| {
        let c = load_len(f, "POLY-DERIV", 1)?;
        let d: Vec<f64> = c.iter().enumerate().skip(1).map(|(i, &a)| a * float(i)).collect();
        store(f, TAG_VEC, &vector(d))
    });
    // POLY-DEFLATE ( c -- q ) ( F: r -- ): divides by (x - r) and drops the remainder.
    word(f, "POLY-DEFLATE", |f| {
        let c = load_len(f, "POLY-DEFLATE", 2)?;
        let r = f.fpop()?;
        let mut q = vec![0.0; c.len() - 1];
        let mut carry = 0.0;
        for i in (1..c.len()).rev() {
            carry = c[i] + carry * r;
            q[i - 1] = carry;
        }
        store(f, TAG_VEC, &vector(q))
    });
    // POLY-LAGUERRE ( c -- ) ( F: x0 tol -- r ): a real root near x0 by Laguerre's
    // method; -264 when a step would be complex (there may be no real root near x0)
    // or when it does not settle in 100 steps.
    word(f, "POLY-LAGUERRE", |f| {
        let c = load_len(f, "POLY-LAGUERRE", 2)?;
        let tol = f.fpop()?;
        let x0 = f.fpop()?;
        work(f, c.len().saturating_mul(MAX_ITERATIONS))?;
        let r = laguerre(&c, x0, tol).ok_or_else(|| {
            error("POLY-LAGUERRE: no real root found from this start (the step turned complex, or it did not settle)")
        })?;
        f.fpush(r)
    });
}

/// Pops the nodes xs and the values ys (ys on top), equal in length, at least one.
fn load_pairs(f: &mut Forth, what: &str) -> Res<(Vec<f64>, Vec<f64>)> {
    let ys = load_vec(f)?;
    let xs = load_vec(f)?;
    if xs.v.len() != ys.v.len() || xs.v.is_empty() {
        return Err(error(format!(
            "{what}: the nodes and the values need one length, at least 1 (not {} and {})",
            xs.v.len(),
            ys.v.len()
        )));
    }
    work(f, xs.v.len().saturating_mul(xs.v.len()))?;
    Ok((xs.v, ys.v))
}

fn same_node() -> Throw {
    error("two nodes are equal")
}

/// The divided differences of Newton's form: d[k] = y[x0 .. xk].
fn divided_differences(xs: &[f64], ys: &[f64]) -> Option<Vec<f64>> {
    let n = xs.len();
    let mut d = ys.to_vec();
    for k in 1..n {
        for i in (k..n).rev() {
            let gap = xs[i] - xs[i - k];
            if gap == 0.0 {
                return None;
            }
            d[i] = (d[i] - d[i - 1]) / gap;
        }
    }
    Some(d)
}

/// Neville's scheme: the value at x and the size of the last correction.
fn neville(xs: &[f64], ys: &[f64], x: f64) -> Option<(f64, f64)> {
    let n = xs.len();
    let mut p = ys.to_vec();
    let mut last = 0.0;
    for m in 1..n {
        for i in 0..n - m {
            let gap = xs[i] - xs[i + m];
            if gap == 0.0 {
                return None;
            }
            let new = ((x - xs[i + m]) * p[i] - (x - xs[i]) * p[i + 1]) / gap;
            last = new - p[i + 1];
            p[i] = new;
        }
    }
    Some((p[0], last.abs()))
}

/// The Bulirsch-Stoer recurrence of rational interpolation (in the form of the
/// differences of consecutive interpolants): the value at x and the size of the last
/// correction; `None` at a pole.
#[allow(clippy::float_cmp, reason = "a point that is exactly a node has its value")]
fn rational(xs: &[f64], ys: &[f64], x: f64) -> Option<(f64, f64)> {
    const TINY: f64 = 1e-25;
    let n = xs.len();
    if let Some(i) = xs.iter().position(|&xi| xi == x) {
        return Some((ys[i], 0.0));
    }
    let mut ns = (0..n)
        .min_by(|&i, &j| (x - xs[i]).abs().total_cmp(&(x - xs[j]).abs()))
        .expect("n is at least 1");
    let mut c = ys.to_vec();
    let mut d: Vec<f64> = ys.iter().map(|&y| y + TINY).collect();
    let mut y = ys[ns];
    let mut last = 0.0;
    for m in 1..n {
        for i in 0..n - m {
            let w = c[i + 1] - d[i];
            let h = xs[i + m] - x;
            let t = (xs[i] - x) * d[i] / h;
            let dd = t - c[i + 1];
            if dd == 0.0 || !dd.is_finite() {
                return None;
            }
            let dd = w / dd;
            d[i] = c[i + 1] * dd;
            c[i] = t * dd;
        }
        last = if 2 * ns < n - m {
            c[ns]
        } else {
            ns -= 1;
            d[ns]
        };
        y += last;
    }
    Some((y, last.abs()))
}

fn install_interpolation(f: &mut Forth) {
    // LAGRANGE ( xs ys -- ) ( F: x -- y ): the polynomial through the points, at x.
    word(f, "LAGRANGE", |f| {
        let (xs, ys) = load_pairs(f, "LAGRANGE")?;
        let x = f.fpop()?;
        let mut sum = 0.0;
        for i in 0..xs.len() {
            let mut term = ys[i];
            for j in 0..xs.len() {
                if i != j {
                    let gap = xs[i] - xs[j];
                    if gap == 0.0 {
                        return Err(same_node());
                    }
                    term *= (x - xs[j]) / gap;
                }
            }
            sum += term;
        }
        f.fpush(sum)
    });
    // NEWTON-DD ( xs ys -- d ): the divided differences y[x0], y[x0 x1], ... as a vector.
    word(f, "NEWTON-DD", |f| {
        let (xs, ys) = load_pairs(f, "NEWTON-DD")?;
        let d = divided_differences(&xs, &ys).ok_or_else(same_node)?;
        store(f, TAG_VEC, &vector(d))
    });
    // NEWTON-EVAL ( xs d -- ) ( F: x -- y ): Newton's form with the divided differences d.
    word(f, "NEWTON-EVAL", |f| {
        let (xs, d) = load_pairs(f, "NEWTON-EVAL")?;
        let x = f.fpop()?;
        let mut y = 0.0;
        for k in (0..d.len()).rev() {
            y = y * (x - xs[k]) + d[k];
        }
        f.fpush(y)
    });
    // AITKEN ( xs ys -- ) ( F: x -- y err ): Neville's scheme, repeated linear
    // interpolation, with the last correction as an estimate of the error.
    word(f, "AITKEN", |f| {
        let (xs, ys) = load_pairs(f, "AITKEN")?;
        let x = f.fpop()?;
        let (y, err) = neville(&xs, &ys, x).ok_or_else(same_node)?;
        f.fpush(y)?;
        f.fpush(err)
    });
    // RATIONAL-INTERP ( xs ys -- ) ( F: x -- y err ): the rational function through the
    // points (Bulirsch-Stoer), with the last correction as an estimate of the error;
    // -264 at a pole.
    word(f, "RATIONAL-INTERP", |f| {
        let (xs, ys) = load_pairs(f, "RATIONAL-INTERP")?;
        let x = f.fpop()?;
        let (y, err) = rational(&xs, &ys, x)
            .ok_or_else(|| error("RATIONAL-INTERP: the interpolating function has a pole at x"))?;
        f.fpush(y)?;
        f.fpush(err)
    });
}

pub fn install(f: &mut Forth) {
    install_orthogonal(f);
    install_closed_forms(f);
    install_polynomials(f);
    install_interpolation(f);
}
