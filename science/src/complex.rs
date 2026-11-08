//! Complex numbers, the fast Fourier transform and the complex roots of
//! polynomials.
//!
//! A complex number lives on the float stack as two floats, the real part
//! below and the imaginary part on top: `( F: re im -- )`. A complex vector is
//! a pair of float vectors, the real parts and the imaginary parts.
//!
//! Built from the mathematics: the rules of complex arithmetic, Smith's
//! division, the polar form for the powers, the iterative radix-2
//! Cooley-Tukey transform with a table of twiddle factors, the direct sum of
//! the discrete transform, and Laguerre's iteration with complex numbers and
//! deflation.

use std::f64::consts::PI;
use std::ops::{Add, Div, Mul, Neg, Sub};

use forthling::{Forth, Res};

use crate::{Mat, TAG_VEC, error, float, load_vec, store, word, work};

/// The most steps of Laguerre's iteration for one root.
const MAX_ITERATIONS: usize = 200;

#[derive(Clone, Copy, Debug, PartialEq)]
struct C {
    re: f64,
    im: f64,
}

const ZERO: C = C { re: 0.0, im: 0.0 };
const ONE: C = C { re: 1.0, im: 0.0 };

const fn c(re: f64, im: f64) -> C {
    C { re, im }
}

impl Add for C {
    type Output = C;
    fn add(self, o: C) -> C {
        c(self.re + o.re, self.im + o.im)
    }
}

impl Sub for C {
    type Output = C;
    fn sub(self, o: C) -> C {
        c(self.re - o.re, self.im - o.im)
    }
}

impl Mul for C {
    type Output = C;
    fn mul(self, o: C) -> C {
        c(self.re * o.re - self.im * o.im, self.re * o.im + self.im * o.re)
    }
}

impl Mul<f64> for C {
    type Output = C;
    fn mul(self, k: f64) -> C {
        c(self.re * k, self.im * k)
    }
}

impl Neg for C {
    type Output = C;
    fn neg(self) -> C {
        c(-self.re, -self.im)
    }
}

/// Smith's division: no overflow where the plain formula overflows.
impl Div for C {
    type Output = C;
    fn div(self, o: C) -> C {
        if o.re.abs() >= o.im.abs() {
            let r = o.im / o.re;
            let d = o.re + o.im * r;
            c((self.re + self.im * r) / d, (self.im - self.re * r) / d)
        } else {
            let r = o.re / o.im;
            let d = o.re * r + o.im;
            c((self.re * r + self.im) / d, (self.im * r - self.re) / d)
        }
    }
}

impl C {
    fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }

    fn arg(self) -> f64 {
        self.im.atan2(self.re)
    }

    fn conj(self) -> C {
        c(self.re, -self.im)
    }

    fn from_polar(r: f64, theta: f64) -> C {
        c(r * theta.cos(), r * theta.sin())
    }

    fn exp(self) -> C {
        C::from_polar(self.re.exp(), self.im)
    }

    /// The principal logarithm.
    fn ln(self) -> C {
        c(self.abs().ln(), self.arg())
    }

    /// The principal square root, with a non-negative real part.
    fn sqrt(self) -> C {
        if self.re == 0.0 && self.im == 0.0 {
            return ZERO;
        }
        let m = self.abs();
        let t = m.midpoint(self.re.abs()).sqrt();
        if self.re >= 0.0 {
            c(t, self.im / (2.0 * t))
        } else {
            c(self.im.abs() / (2.0 * t), t.copysign(self.im))
        }
    }

    fn sin(self) -> C {
        c(self.re.sin() * self.im.cosh(), self.re.cos() * self.im.sinh())
    }

    fn cos(self) -> C {
        c(self.re.cos() * self.im.cosh(), -self.re.sin() * self.im.sinh())
    }

    fn sinh(self) -> C {
        c(self.re.sinh() * self.im.cos(), self.re.cosh() * self.im.sin())
    }

    fn cosh(self) -> C {
        c(self.re.cosh() * self.im.cos(), self.re.sinh() * self.im.sin())
    }

    /// An integer power by repeated squaring.
    fn powi(self, n: i64) -> C {
        let mut base = if n < 0 { ONE / self } else { self };
        let mut e = n.unsigned_abs();
        let mut r = ONE;
        while e > 0 {
            if e & 1 == 1 {
                r = r * base;
            }
            base = base * base;
            e >>= 1;
        }
        r
    }

    fn is_finite(self) -> bool {
        self.re.is_finite() && self.im.is_finite()
    }
}

fn pop_c(f: &mut Forth) -> Res<C> {
    let im = f.fpop()?;
    let re = f.fpop()?;
    Ok(c(re, im))
}

fn push_c(f: &mut Forth, z: C) -> Res {
    f.fpush(z.re)?;
    f.fpush(z.im)
}

fn unary(f: &mut Forth, name: &'static str, op: fn(C) -> C) {
    word(f, name, move |f| {
        let z = pop_c(f)?;
        push_c(f, op(z))
    });
}

fn binary(f: &mut Forth, name: &'static str, op: fn(C, C) -> C) {
    word(f, name, move |f| {
        let w = pop_c(f)?;
        let z = pop_c(f)?;
        push_c(f, op(z, w))
    });
}

fn install_arithmetic(f: &mut Forth) {
    binary(f, "Z+", |z, w| z + w);
    binary(f, "Z-", |z, w| z - w);
    binary(f, "Z*", |z, w| z * w);
    binary(f, "Z/", |z, w| z / w);
    unary(f, "ZNEGATE", |z| -z);
    unary(f, "ZCONJG", C::conj);
    unary(f, "ZSQRT", C::sqrt);
    unary(f, "ZEXP", C::exp);
    unary(f, "ZLN", C::ln);
    unary(f, "ZSIN", C::sin);
    unary(f, "ZCOS", C::cos);
    unary(f, "ZSINH", C::sinh);
    unary(f, "ZCOSH", C::cosh);
    // ZABS ( F: re im -- r ) and ZARG ( F: re im -- theta )
    word(f, "ZABS", |f| {
        let z = pop_c(f)?;
        f.fpush(z.abs())
    });
    word(f, "ZARG", |f| {
        let z = pop_c(f)?;
        f.fpush(z.arg())
    });
    // POLAR>Z ( F: r theta -- re im )
    word(f, "POLAR>Z", |f| {
        let theta = f.fpop()?;
        let r = f.fpop()?;
        push_c(f, C::from_polar(r, theta))
    });
    // Z^N ( n -- ) ( F: z -- z^n ): a whole power.
    word(f, "Z^N", |f| {
        let n = f.pop()?;
        let z = pop_c(f)?;
        work(f, 64)?;
        push_c(f, z.powi(n))
    });
    // Z** ( F: z w -- z^w ): exp(w ln z) with the principal logarithm; 0^w is 0
    // for a w with a positive real part.
    word(f, "Z**", |f| {
        let w = pop_c(f)?;
        let z = pop_c(f)?;
        let r = if z == ZERO {
            if w.re > 0.0 {
                ZERO
            } else {
                return Err(error("Z**: 0 to a power whose real part is not above 0"));
            }
        } else {
            (w * z.ln()).exp()
        };
        push_c(f, r)
    });
}

// ---------------------------------------------------------------------------
// The transforms

/// A vector of the Forth memory, with the address of its first float.
struct Vector {
    start: usize,
    v: Vec<f64>,
}

/// Pops the address of a vector and copies it, keeping where it is.
fn load_at(f: &mut Forth) -> Res<Vector> {
    let addr = f.pop()?;
    f.push(addr);
    let m = load_vec(f)?;
    let start = usize::try_from(addr).expect("the address was checked by load") + crate::HEADER;
    Ok(Vector { start, v: m.v })
}

fn write_back(f: &mut Forth, x: &Vector) {
    for (i, r) in x.v.iter().enumerate() {
        let at = x.start + 8 * i;
        f.memory_mut()[at..at + 8].copy_from_slice(&r.to_le_bytes());
    }
}

/// Pops the imaginary and then the real parts of a complex vector, of one length.
fn load_pair(f: &mut Forth, what: &str) -> Res<(Vector, Vector)> {
    let im = load_at(f)?;
    let re = load_at(f)?;
    if re.v.len() != im.v.len() {
        return Err(error(format!(
            "{what}: the real and the imaginary parts need one length, not {} and {}",
            re.v.len(),
            im.v.len()
        )));
    }
    Ok((re, im))
}

fn vector(v: Vec<f64>) -> Mat {
    Mat { rows: v.len(), cols: 1, v }
}

/// The twiddle factors exp(-2 pi i k / n) for k below n, each from its own
/// sine and cosine, so that no error builds up along the table.
fn twiddles(n: usize) -> Vec<C> {
    (0..n).map(|k| C::from_polar(1.0, -2.0 * PI * float(k) / float(n))).collect()
}

/// The transform in place: radix 2, iterative, with a bit-reversal first.
/// `inverse` uses the conjugate factors and divides by n.
fn fft(z: &mut [C], inverse: bool) {
    let n = z.len();
    if n <= 1 {
        return;
    }
    let bits = n.trailing_zeros();
    for i in 0..n {
        let j = i.reverse_bits() >> (usize::BITS - bits);
        if j > i {
            z.swap(i, j);
        }
    }
    let table = twiddles(n);
    let mut half = 1;
    while half < n {
        let stride = n / (2 * half);
        for start in (0..n).step_by(2 * half) {
            for k in 0..half {
                let w = if inverse { table[k * stride].conj() } else { table[k * stride] };
                let a = z[start + k];
                let b = z[start + k + half] * w;
                z[start + k] = a + b;
                z[start + k + half] = a - b;
            }
        }
        half *= 2;
    }
    if inverse {
        let scale = 1.0 / float(n);
        for x in z.iter_mut() {
            *x = *x * scale;
        }
    }
}

/// The discrete transform by its definition, for any length: n squared steps.
fn dft(z: &[C], inverse: bool) -> Vec<C> {
    let n = z.len();
    let table = twiddles(n);
    let scale = if inverse { 1.0 / float(n) } else { 1.0 };
    (0..n)
        .map(|k| {
            let mut sum = ZERO;
            for (j, &x) in z.iter().enumerate() {
                let w = table[(k * j) % n];
                sum = sum + x * if inverse { w.conj() } else { w };
            }
            sum * scale
        })
        .collect()
}

fn join(re: &[f64], im: &[f64]) -> Vec<C> {
    re.iter().zip(im).map(|(&r, &i)| c(r, i)).collect()
}

fn split(z: &[C]) -> (Vec<f64>, Vec<f64>) {
    (z.iter().map(|x| x.re).collect(), z.iter().map(|x| x.im).collect())
}

/// The work of a transform of n points in float operations: 5 n log2 n.
fn fft_work(n: usize) -> usize {
    n.saturating_mul(5).saturating_mul(n.max(2).ilog2() as usize)
}

fn install_fft(f: &mut Forth) {
    // FFT ( re im -- ) and IFFT ( re im -- ): the transform in place, for a
    // length that is a power of 2; the inverse divides by n.
    for (name, inverse) in [("FFT", false), ("IFFT", true)] {
        word(f, name, move |f| {
            let (mut re, mut im) = load_pair(f, name)?;
            let n = re.v.len();
            if !n.is_power_of_two() {
                return Err(error(format!(
                    "{name}: the length {n} is not a power of 2 (DFT takes any length)"
                )));
            }
            work(f, fft_work(n))?;
            let mut z = join(&re.v, &im.v);
            fft(&mut z, inverse);
            (re.v, im.v) = split(&z);
            write_back(f, &re);
            write_back(f, &im);
            Ok(())
        });
    }
    // DFT ( re im -- re' im' ) and IDFT ( re im -- re' im' ): new vectors, any
    // length, by the definition: n squared steps.
    for (name, inverse) in [("DFT", false), ("IDFT", true)] {
        word(f, name, move |f| {
            let (re, im) = load_pair(f, name)?;
            let n = re.v.len();
            if n == 0 {
                return Err(error(format!("{name}: the vectors are empty")));
            }
            work(f, n.saturating_mul(n).saturating_mul(8))?;
            let (r, i) = split(&dft(&join(&re.v, &im.v), inverse));
            store(f, TAG_VEC, &vector(r))?;
            store(f, TAG_VEC, &vector(i))
        });
    }
    // ZV-ABS ( re im -- m ): a new vector with the absolute value of each element.
    word(f, "ZV-ABS", |f| {
        let (re, im) = load_pair(f, "ZV-ABS")?;
        work(f, re.v.len().saturating_mul(4))?;
        let m = re.v.iter().zip(&im.v).map(|(&r, &i)| r.hypot(i)).collect();
        store(f, TAG_VEC, &vector(m))
    });
}

// ---------------------------------------------------------------------------
// Roots of polynomials

/// The value, the first and the second derivative at x (Horner's rule).
fn horner2(co: &[C], x: C) -> (C, C, C) {
    let mut p = ZERO;
    let mut d = ZERO;
    let mut e = ZERO;
    for &a in co.iter().rev() {
        e = e * x + d * 2.0;
        d = d * x + p;
        p = p * x + a;
    }
    (p, d, e)
}

/// A root of the polynomial near `x`, by Laguerre's iteration; `None` when it
/// does not settle. Every tenth step is shortened, which breaks the rare cycle.
fn laguerre(co: &[C], mut x: C, tol: f64) -> Option<C> {
    const SHORTEN: [f64; 8] = [0.5, 0.25, 0.75, 0.13, 0.38, 0.62, 0.88, 1.0];
    let n = float(co.len() - 1);
    for iteration in 1..=MAX_ITERATIONS {
        let (p, d, e) = horner2(co, x);
        if p == ZERO {
            return Some(x);
        }
        let g = d / p;
        let h = g * g - e / p;
        let root = ((h * n - g * g) * (n - 1.0)).sqrt();
        let plus = g + root;
        let minus = g - root;
        let denom = if plus.abs() >= minus.abs() { plus } else { minus };
        let step = if denom == ZERO {
            // The derivative vanishes: a step of fixed size, in a turning direction.
            C::from_polar(1.0 + x.abs(), float(iteration))
        } else {
            c(n, 0.0) / denom
        };
        if !step.is_finite() {
            return None;
        }
        let shorten = if iteration % 10 == 0 { SHORTEN[iteration / 10 % 8] } else { 1.0 };
        x = x - step * shorten;
        if step.abs() <= tol * x.abs().max(1.0) {
            return Some(x);
        }
    }
    None
}

/// All the roots of the polynomial: find one, divide it out, go on; then polish
/// each on the whole polynomial. The roots come sorted by real and then
/// imaginary part.
fn all_roots(co: &[C]) -> Option<Vec<C>> {
    let mut rest = co.to_vec();
    // Leading zero coefficients lower the degree.
    while rest.len() > 1 && rest[rest.len() - 1] == ZERO {
        rest.pop();
    }
    if rest.is_empty() || rest[0] == ZERO && rest.len() == 1 {
        return None; // the zero polynomial has every number as a root
    }
    let mut roots = Vec::new();
    while rest.len() > 1 {
        let r = laguerre(&rest, ZERO, 1e-14)?;
        let r = laguerre(co, r, 1e-15).unwrap_or(r);
        // Divide by (x - r), the remainder dropped.
        let mut carry = ZERO;
        let mut q = vec![ZERO; rest.len() - 1];
        for i in (1..rest.len()).rev() {
            carry = rest[i] + carry * r;
            q[i - 1] = carry;
        }
        rest = q;
        roots.push(r);
    }
    for r in &mut roots {
        *r = laguerre(co, *r, 1e-15).unwrap_or(*r);
    }
    roots.sort_by(|a, b| a.re.total_cmp(&b.re).then(a.im.total_cmp(&b.im)));
    Some(roots)
}

fn push_roots(f: &mut Forth, roots: &[C]) -> Res {
    let (re, im) = split(roots);
    store(f, TAG_VEC, &vector(re))?;
    store(f, TAG_VEC, &vector(im))
}

/// Both roots of a x^2 + b x + c for real coefficients with a not 0, sorted.
fn quadratic_z(a: f64, b: f64, cc: f64) -> [C; 2] {
    let disc = b * b - 4.0 * a * cc;
    if disc < 0.0 {
        let re = -b / (2.0 * a) + 0.0; // + 0.0 turns -0 into 0
        let im = (-disc).sqrt() / (2.0 * a).abs();
        [c(re, -im), c(re, im)]
    } else {
        // q has the sign of b, so that no digits cancel.
        let q = -0.5 * (b + disc.sqrt().copysign(b));
        let (x1, x2) = if q == 0.0 { (0.0, 0.0) } else { (q / a, cc / q) };
        [c(x1.min(x2), 0.0), c(x1.max(x2), 0.0)]
    }
}

fn install_roots(f: &mut Forth) {
    // ZPOLY-EVAL ( cre cim -- ) ( F: re im -- re' im' ): Horner's rule.
    word(f, "ZPOLY-EVAL", |f| {
        let (cre, cim) = load_pair(f, "ZPOLY-EVAL")?;
        let x = pop_c(f)?;
        let co = join(&cre.v, &cim.v);
        if co.is_empty() {
            return Err(error("ZPOLY-EVAL: the polynomial has no coefficients"));
        }
        work(f, co.len().saturating_mul(8))?;
        push_c(f, horner2(&co, x).0)
    });
    // ZPOLY-LAGUERRE ( cre cim -- ) ( F: re0 im0 tol -- re im ): a root near the
    // start, with complex coefficients and steps.
    word(f, "ZPOLY-LAGUERRE", |f| {
        let (cre, cim) = load_pair(f, "ZPOLY-LAGUERRE")?;
        let tol = f.fpop()?;
        let x0 = pop_c(f)?;
        let co = join(&cre.v, &cim.v);
        if co.len() < 2 {
            return Err(error("ZPOLY-LAGUERRE: the polynomial needs at least 2 coefficients"));
        }
        work(f, co.len().saturating_mul(MAX_ITERATIONS).saturating_mul(16))?;
        let r = laguerre(&co, x0, tol)
            .ok_or_else(|| error("ZPOLY-LAGUERRE: the iteration did not settle"))?;
        push_c(f, r)
    });
    // ZPOLY-ROOTS ( cre cim -- re im ): all the roots, as two new vectors.
    word(f, "ZPOLY-ROOTS", |f| {
        let (cre, cim) = load_pair(f, "ZPOLY-ROOTS")?;
        let co = join(&cre.v, &cim.v);
        let n = co.len();
        work(f, n.saturating_mul(n).saturating_mul(MAX_ITERATIONS).saturating_mul(4))?;
        let roots = all_roots(&co).ok_or_else(|| {
            error("ZPOLY-ROOTS: the iteration did not settle (is the polynomial 0?)")
        })?;
        push_roots(f, &roots)
    });
    // POLY-ROOTS ( c -- re im ): all the roots of a real polynomial.
    word(f, "POLY-ROOTS", |f| {
        let cv = load_vec(f)?;
        let co: Vec<C> = cv.v.iter().map(|&a| c(a, 0.0)).collect();
        let n = co.len();
        work(f, n.saturating_mul(n).saturating_mul(MAX_ITERATIONS).saturating_mul(4))?;
        let roots = all_roots(&co).ok_or_else(|| {
            error("POLY-ROOTS: the iteration did not settle (is the polynomial 0?)")
        })?;
        push_roots(f, &roots)
    });
    // ZQUADRATIC ( F: a b c -- z1 z2 ): both roots of a x^2 + b x + c, real
    // coefficients, sorted by real and then imaginary part.
    word(f, "ZQUADRATIC", |f| {
        let cc = f.fpop()?;
        let b = f.fpop()?;
        let a = f.fpop()?;
        if a == 0.0 || !a.is_finite() {
            return Err(error("ZQUADRATIC: the coefficient of x^2 must not be 0"));
        }
        for z in quadratic_z(a, b, cc) {
            push_c(f, z)?;
        }
        Ok(())
    });
    // ZCUBIC ( F: a b c d -- z1 z2 z3 ): the three roots of a x^3 + b x^2 + c x + d,
    // real coefficients, sorted by real and then imaginary part.
    word(f, "ZCUBIC", |f| {
        let d = f.fpop()?;
        let cc = f.fpop()?;
        let b = f.fpop()?;
        let a = f.fpop()?;
        if a == 0.0 || !a.is_finite() {
            return Err(error("ZCUBIC: the coefficient of x^3 must not be 0"));
        }
        let mut roots = cubic_z(a, b, cc, d);
        roots.sort_by(|p, q| p.re.total_cmp(&q.re).then(p.im.total_cmp(&q.im)));
        for z in roots {
            push_c(f, z)?;
        }
        Ok(())
    });
}

/// The three roots of a real cubic: the real ones from the trigonometric or
/// Cardano form, and when only one is real, the quadratic that is left after
/// dividing it out.
fn cubic_z(a: f64, b: f64, cc: f64, d: f64) -> Vec<C> {
    let real = crate::poly::cubic(a, b, cc, d);
    if real.len() == 3 {
        return real.into_iter().map(|r| c(r, 0.0)).collect();
    }
    let r = real[0];
    // a x^3 + b x^2 + c x + d = (x - r)(a x^2 + (b + a r) x + (c + (b + a r) r)).
    let b2 = b + a * r;
    let c2 = cc + b2 * r;
    let [z1, z2] = quadratic_z(a, b2, c2);
    vec![c(r, 0.0), z1, z2]
}

pub fn install(f: &mut Forth) {
    install_arithmetic(f);
    install_fft(f);
    install_roots(f);
}
