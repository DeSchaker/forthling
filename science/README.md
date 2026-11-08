# Numerical words for forthling: linear algebra, special functions, calculus, roots, interpolation and the FFT

The word list `SCIENCE` for [forthling](https://github.com/DeSchaker/forthling): vectors of
floats, matrices, and the linear algebra on them (solving equations, determinants,
inverses and condition numbers), and the special functions: gamma, error function,
incomplete gamma and beta, logistic, and numerical integration and ordinary differential
equations for a function that the program passes as an execution token, and polynomials,
root finding and interpolation, complex numbers, the fast Fourier transform and complex
roots. It is built from
the mathematics, with no dependencies (the arrays and the special functions are Rust, the
integration and the differential equations are Forth, in `src/calculus.fs`), and it reaches
nothing outside Forth: no permission is needed, so the words work in the sandbox, in the
MCP server, in the Python module and in the playground, where they are built in.

## Running

`forthling-science` is forthling with these words installed:

```
cargo run --release --manifest-path science/Cargo.toml -- science/examples/linear_algebra.fs
```

In a Rust program: `forth.install(&forthling_science::Science)`, or
`forthling::cli::main(&[&forthling_science::Science])` for the whole program.

`S" FORTHLING-SCIENCE" ENVIRONMENT?` gives `1 true` when the words are there. They are in
the word list `SCIENCE`, so a program starts with `ALSO SCIENCE`.

## Arrays

An array lives on the Forth heap, as the block that `ALLOCATE` gives: a header of 24 bytes
(the kind, the rows and the columns) followed by the floats, a row after a row, 8 bytes
each. The program `FREE`s it. A vector of n floats is an n by 1 matrix. A word that makes
an array returns its address; `FV[]` and `MAT[]` give the address of an element, to read
with `F@` and to write with `F!`, and throw -24 for an index out of range.

Words that compute a result (`MAT*`, `MAT-INV`, `FV+` and the like) make a new array and
leave their arguments as they were. A word that is given an address that is not an array
of the right kind throws -9 (not an address) or -264 (the wrong kind or shape). A size that
does not fit in the memory throws -59, as `ALLOCATE` does.

There is no word that prints an array, since a loop with `F.` says what you want to see;
the [example](examples/linear_algebra.fs) has two small ones.

## Words

| Word | Stack | |
|---|---|---|
| `FV-NEW` | `( n -- v )` | a vector of n zeros |
| `FV-FROM-STACK` | `( n -- v ) ( F: x1 … xn -- )` | a vector of the floats on the float stack |
| `FV-LEN` | `( v -- n )` | the number of elements |
| `FV[]` | `( i v -- f-addr )` | the address of element i |
| `FV-FILL` | `( v -- ) ( F: x -- )` | set every element to x |
| `FV-DOT` | `( v w -- ) ( F: -- r )` | the dot product |
| `FV-NORM` | `( v -- ) ( F: -- r )` | the Euclidean length |
| `FV-SCALE` | `( v -- w ) ( F: x -- )` | every element times x |
| `FV+`, `FV-` | `( v w -- u )` | the sum and the difference |
| `MAT-NEW` | `( rows cols -- m )` | a matrix of zeros |
| `MAT-FROM-STACK` | `( rows cols -- m ) ( F: x11 x12 … -- )` | a matrix of the floats on the float stack, a row after a row |
| `MAT-IDENTITY` | `( n -- m )` | the identity matrix |
| `MAT-HILBERT` | `( n -- m )` | the Hilbert matrix, 1/(i+j+1) in row i and column j |
| `MAT-ROWS`, `MAT-COLS` | `( m -- n )` | the shape |
| `MAT[]` | `( i j m -- f-addr )` | the address of the element in row i and column j |
| `MAT-T` | `( m -- t )` | the transpose |
| `MAT+`, `MAT-` | `( a b -- c )` | the sum and the difference |
| `MAT-SCALE` | `( m -- c ) ( F: x -- )` | every element times x |
| `MAT*` | `( a b -- c )` | the product |
| `MAT*V` | `( m v -- w )` | a matrix times a vector |
| `MAT-NORM` | `( m -- ) ( F: -- r )` | the infinity norm: the largest sum of absolute values in a row |
| `MAT-LU` | `( a -- lu piv sign )` | LU factorization with partial pivoting |
| `LU-SOLVE` | `( b lu piv -- x )` | solve A x = b with a factorization; cheap for many b |
| `MAT-SOLVE` | `( b a -- x )` | solve A x = b |
| `MAT-DET` | `( a -- ) ( F: -- r )` | the determinant; 0 for a singular matrix |
| `MAT-INV` | `( a -- c )` | the inverse, by Gauss-Jordan elimination |
| `MAT-COND` | `( a -- ) ( F: -- r )` | the condition number in the infinity norm |
| `MAT-GAUSS-JORDAN` | `( a b -- x )` | solve A X = B for a matrix B of right-hand sides |

`MAT-LU`, `MAT-SOLVE` and `MAT-INV` throw -264 for a singular matrix; `MAT-DET` gives 0.

## Special functions

These work on the float stack and need no array. `ALSO SCIENCE` first.

| Word | Stack | |
|---|---|---|
| `GAMMA` | `( F: x -- r )` | the gamma function; `GAMMA(n)` is (n-1)! at whole n |
| `LNGAMMA` | `( F: x -- r )` | ln of the absolute value of gamma; works where `GAMMA` overflows |
| `FACTORIAL` | `( n -- ) ( F: -- r )` | n! for n from 0 to 170 |
| `ERF`, `ERFC` | `( F: x -- r )` | the error function and 1 - erf; `ERFC` keeps its digits in the tail |
| `GAMMA-P`, `GAMMA-Q` | `( F: a x -- r )` | the regularized incomplete gamma function P(a, x) and 1 - P(a, x) |
| `BETA` | `( F: a b -- r )` | the beta function |
| `BETA-I` | `( F: a b x -- r )` | the regularized incomplete beta function I(x; a, b) |
| `LOGISTIC` | `( F: x -- r )` | 1 / (1 + e^-x), without overflow |
| `LOGISTIC-D` | `( F: x -- r )` | its derivative |

`GAMMA` and `LNGAMMA` throw -46 (invalid argument) at 0 and the negative whole numbers;
`GAMMA` and `FACTORIAL` throw -43 (result out of range) above 171.6 and 170; `FACTORIAL` of
a negative number throws -24. `GAMMA-P`, `GAMMA-Q`, `BETA` and `BETA-I` throw -46 for a
parameter that is not above 0 and for an x outside the range (x from 0 for the gamma
functions, from 0 to 1 for `BETA-I`). The error function is the incomplete gamma function
with a = 1/2, and the gamma function comes from the Stirling series. The tests compare with
Python's `math` module (`gamma`, `lgamma`, `erf`, `erfc`) to 2e-13 relative (the gamma
function) and 1e-12 or better for the others, and with the closed forms that exist for
whole parameters (P(1, x) = 1 - e^-x, the Poisson sum for a whole a, and the binomial sum for
a whole a and b).

The [example](examples/special_functions.fs) turns them into the usual distributions: the
normal (`ERF`), the chi-square (`GAMMA-Q`) and the binomial (`BETA-I`):

```
P(Z <= 1.96) = 0.975002
P(|Z| > 1.96) = 0.049996
chi-square, 1 degree, 3.841: p = 0.050014
chi-square, 10 degrees, 18.307: p = 0.050001
P(heads <= 14 of 40) = 0.040345
```

## Integration

The function is an execution token with the effect `( F: x -- y )`. `ALSO SCIENCE` first.

| Word | Stack | |
|---|---|---|
| `TRAPEZOID` | `( xt n -- ) ( F: a b -- r )` | the trapezoid rule with n panels |
| `SIMPSON` | `( xt n -- ) ( F: a b -- r )` | Simpson's rule with n panels (n is made even); exact for a cubic |
| `TRAPEZOID-TOL` | `( xt -- ) ( F: a b tol -- r err )` | the trapezoid rule, halving the panels until two sums differ by at most `tol * max(1, abs(r))` |
| `ROMBERG` | `( xt -- ) ( F: a b tol -- r err )` | the same sums with Richardson extrapolation; fast for a smooth function |
| `INTEGRATE` | `( xt -- ) ( F: a b tol -- r err )` | adaptive Gauss-Legendre: the interval with the largest error is split until the errors add up to at most `tol * max(1, abs(r))` |

`err` is an estimate, not a bound. For `ROMBERG` and `TRAPEZOID-TOL` it is the difference
between the last two values. For `INTEGRATE` it is the difference between one 5-point
Gauss-Legendre rule and two on the halves of each interval, which is usually far above the
true error (5 points are exact up to degree 9). `ROMBERG` and `TRAPEZOID-TOL` throw -264
after 20 halvings, `INTEGRATE` after 500 intervals; `TRAPEZOID` and `SIMPSON` throw -24 for
fewer than 1 panel. The ends may be in either order (the integral changes sign), and the
function may integrate again: a double integral is an `INTEGRATE` in the word that
`INTEGRATE` calls. Each word keeps its floats in a scratch vector that it frees at the end.

## Ordinary differential equations

The state `y` is a float vector. The derivative is an execution token with the effect
`( y dy -- ) ( F: t -- )` that fills `dy` with y'(t). Both words change `y` in place.

| Word | Stack | |
|---|---|---|
| `RK4` | `( y xt steps -- ) ( F: t0 t1 -- )` | the classical Runge-Kutta method of order 4 with a fixed number of steps |
| `ODE-RKF` | `( y xt -- ) ( F: t0 t1 tol -- )` | the adaptive Runge-Kutta-Fehlberg method of orders 4 and 5; t1 may be below t0 |

`ODE-RKF` keeps the error of a step below `tol * (1 + abs(y))` in each component, and
throws -264 when the step gets too small (for example at a singularity) or after 100000
steps. `RK4` throws -24 for fewer than 1 step. The tests check the fourth-order
convergence of `RK4` (twice the steps gives about 16 times less error) and compare
`ODE-RKF` with closed solutions (decay, the oscillator over a period and backwards in time).

The [example](examples/calculus.fs) integrates the normal density, a function that is not
smooth and a double integral, and solves logistic growth:

```
P(|Z| < 1.96) = 0.950004
error estimate below 1e-9: -1
Simpson, 20 panels: 0.950003
Romberg: 0.950004
sqrt: 0.666667
double integral: 0.25
y(10) with ODE-RKF: 88.6508
y(10) exactly: 88.6508
y(10) with RK4: 88.6508
```

## Polynomials and orthogonal polynomials

A polynomial is a float vector of coefficients, the constant term first: `1e 2e 3e 3
FV-FROM-STACK` is 1 + 2x + 3x^2. `ALSO SCIENCE` first.

| Word | Stack | |
|---|---|---|
| `POLY-EVAL` | `( c -- ) ( F: x -- r )` | the value at x, by Horner's rule |
| `POLY-EVAL-D` | `( c -- ) ( F: x -- p dp )` | the value and the derivative |
| `POLY-DERIV` | `( c -- d )` | the derivative, a vector one shorter |
| `POLY-DEFLATE` | `( c -- q ) ( F: r -- )` | the quotient by (x - r); the remainder is dropped |
| `POLY-LAGUERRE` | `( c -- ) ( F: x0 tol -- r )` | a real root near x0 by Laguerre's method |
| `CHEBYSHEV-T`, `CHEBYSHEV-U` | `( n -- ) ( F: x -- r )` | Chebyshev polynomials of the first and the second kind |
| `LEGENDRE` | `( n -- ) ( F: x -- r )` | the Legendre polynomial |
| `HERMITE` | `( n -- ) ( F: x -- r )` | the Hermite polynomial, physicists' form (H_1 = 2x) |
| `LAGUERRE` | `( n -- ) ( F: x -- r )` | the Laguerre polynomial |
| `QUADRATIC` | `( F: a b c -- x1 … xn ) ( -- n )` | the real roots of a x^2 + b x + c, smallest first |
| `CUBIC` | `( F: a b c d -- x1 … xn ) ( -- n )` | the real roots of a x^3 + b x^2 + c x + d, smallest first |

The orthogonal polynomials use their three-term recurrences (so `n` may be large); a
negative `n` throws -24. `QUADRATIC` and `CUBIC` leave n real roots on the float stack
(the smallest deepest) and n on the data stack: 2 or 0 for a quadratic (a double root
counts twice; 1 when a is 0), 3 or 1 for a cubic. `QUADRATIC` avoids the cancellation of the
textbook formula, so the small root of x^2 + 10^8 x + 1 keeps all its digits; `CUBIC` uses
Cardano's formula or the trigonometric form, and one Newton step polishes each root.
`POLY-LAGUERRE` works with real numbers only: it throws -264 when a step would be complex
(a polynomial without a real root near the start, such as x^2 + 1) or when it does not
settle in 100 steps. Deflating by a root found leaves a polynomial for the other roots; the
[example](examples/roots.fs) finds all four roots of a quartic that way. For complex roots
see "Complex roots" below: `POLY-ROOTS` finds all of them at once.

## Root finding

The function is an execution token with the effect `( F: x -- y )`. `ALSO SCIENCE` first.

| Word | Stack | |
|---|---|---|
| `BISECT` | `( xt -- ) ( F: a b tol -- r )` | bisection in [a, b] until the interval is no wider than tol |
| `REGULA-FALSI` | `( xt -- ) ( F: a b tol -- r )` | false position with the Illinois rule |
| `NEWTON` | `( xt dxt -- ) ( F: x0 tol -- r )` | Newton's method with the function and its derivative |

`BISECT` and `REGULA-FALSI` need f(a) and f(b) of different sign, or one of them 0 (that end
is the answer); otherwise they throw -264, as for a NaN from the function. Bisection always
gets there. Regula falsi usually needs far fewer calls (for x^2 - 2 on [0, 2] with a tolerance
of 1e-14 it calls the function 11 times, bisection 50 times), and the Illinois rule halves the value at an end that stays twice, so that
one end cannot stay for ever; for a very steep function (x^1024 - 1 on [0, 1.5]) it still
needs hundreds of steps, throws -264 after 200, and `BISECT` is the word to use. `NEWTON`
ends when the step is at most `tol * max(1, abs(x))` and throws -264 when the derivative is
0 or after 100 steps. Each word keeps its floats in a scratch vector that it frees at the
end.

## Interpolation

`xs` and `ys` are float vectors of one length (the nodes and the values at them). The nodes
must differ: equal nodes, or vectors of unequal length, throw -264. `ALSO SCIENCE` first.

| Word | Stack | |
|---|---|---|
| `LAGRANGE` | `( xs ys -- ) ( F: x -- y )` | the polynomial through the points, at x |
| `NEWTON-DD` | `( xs ys -- d )` | the divided differences, as a vector |
| `NEWTON-EVAL` | `( xs d -- ) ( F: x -- y )` | Newton's form with those differences, at x |
| `AITKEN` | `( xs ys -- ) ( F: x -- y err )` | repeated linear interpolation (Neville's scheme) with an error estimate |
| `RATIONAL-INTERP` | `( xs ys -- ) ( F: x -- y err )` | a rational function through the points (Bulirsch-Stoer) with an error estimate |

`NEWTON-DD` once and `NEWTON-EVAL` many times is cheaper than `LAGRANGE` for many x.
`err` is the size of the last correction: an estimate of the error, not a bound. A rational
function does better than a polynomial near a pole: with the five points of 1/(1 - x^2) at
x = 0, 0.2, ..., 0.8, the value at 0.9 is 5.26316 from `RATIONAL-INTERP` and 4.0332 from
`LAGRANGE`, where the function is 5.26316 (this is in the example). Every word costs time
in proportion to n squared for n points, charged against the limits.

The [example](examples/roots.fs) shows all of this:

```
quadratic: 2. 1.
cubic: 3. 2. 1.
no real roots of x^2 + 1: 0
roots by Laguerre and deflation: 1. 2. 3. 4.
p(2.5) = 0.5625
T5(0.3) = 0.99888
P3(0.3) = -0.3825
H3(0.3) = -3.384
bisection: 0.739085
regula falsi: 0.739085
Newton: 0.739085
sin(0.9) = 0.783327
Lagrange: 0.783327
Aitken: 0.783327
Newton's form: 0.783327
1/(1-0.9^2) = 5.26316
polynomial: 4.0332
rational: 5.26316
```

## Complex numbers

A complex number is two floats on the float stack, the real part below and the imaginary
part on top: `3e 4e` is 3 + 4i. Every word below works on the float stack, so the usual
`FDUP`, `FSWAP` and the float locals are there for them. `ALSO SCIENCE` first.

| Word | Stack | |
|---|---|---|
| `Z+`, `Z-`, `Z*`, `Z/` | `( F: z w -- r )` | add, subtract, multiply, divide (Smith's method: no overflow where the plain formula overflows) |
| `ZNEGATE`, `ZCONJG` | `( F: z -- w )` | the negative and the conjugate |
| `ZABS`, `ZARG` | `( F: z -- r )` | the modulus, and the argument from -pi to pi |
| `POLAR>Z` | `( F: r theta -- z )` | the number with that modulus and argument |
| `ZSQRT`, `ZEXP`, `ZLN` | `( F: z -- w )` | the principal square root (real part not negative), the exponential, the principal logarithm |
| `ZSIN`, `ZCOS`, `ZSINH`, `ZCOSH` | `( F: z -- w )` | sine, cosine and the hyperbolic ones |
| `Z^N` | `( n -- ) ( F: z -- w )` | a whole power, also negative, by repeated squaring |
| `Z**` | `( F: z w -- r )` | exp(w ln z), the principal value; 0 to a power with a real part above 0 is 0, otherwise -264 |

The functions follow the principal values, so `-4e 0e ZSQRT` is 2i and `0e 1e 0e 1e Z**`
(i to the power i) is 0.20788, a real number. A division by zero is no error: the floats
give inf or NaN, as for `F/`.

## The fast Fourier transform

A complex vector is a pair of float vectors: the real parts and the imaginary parts, of one
length. `ALSO SCIENCE` first.

| Word | Stack | |
|---|---|---|
| `FFT` | `( re im -- )` | the transform in place, for a length that is a power of 2 |
| `IFFT` | `( re im -- )` | the inverse in place, with the factor 1/N |
| `DFT`, `IDFT` | `( re im -- re' im' )` | the same by the definition, for any length, as two new vectors |
| `ZV-ABS` | `( re im -- m )` | a new vector with the absolute value of each element |

The transform is X(k) = the sum over n of x(n) e^(-2 pi i k n / N). `FFT` is the iterative
radix-2 algorithm with a table of twiddle factors, each from its own sine and cosine, and
costs about 5 N log2 N operations, charged as work: a signal of 16384 points is no
problem. Any other length throws -264 (as do two vectors of unequal length); `DFT` takes
any length but costs N squared. `IFFT` of `FFT` gives the vectors back (for the signal of 8 points in the tests the
difference is below 1e-14). For a real signal, leave the imaginary vector at zero, and
`ZV-ABS` after `FFT` gives the amplitude spectrum: a wave of amplitude a with k periods in
the N samples has peaks of height a N/2 at bin k and at bin N - k.

## Complex roots

| Word | Stack | |
|---|---|---|
| `ZQUADRATIC` | `( F: a b c -- z1 z2 )` | both roots of a x^2 + b x + c, real coefficients |
| `ZCUBIC` | `( F: a b c d -- z1 z2 z3 )` | the three roots of a x^3 + b x^2 + c x + d, real coefficients |
| `POLY-ROOTS` | `( c -- re im )` | all roots of a real polynomial, as two new vectors |
| `ZPOLY-ROOTS` | `( cre cim -- re im )` | all roots of a polynomial with complex coefficients |
| `ZPOLY-LAGUERRE` | `( cre cim -- ) ( F: z0 tol -- z )` | one root near the complex start z0 |
| `ZPOLY-EVAL` | `( cre cim -- ) ( F: z -- w )` | the value at z, by Horner's rule |

`ZQUADRATIC` and `ZCUBIC` always leave all the roots, each as two floats, real or in a
complex pair; the roots lie in ascending order of the real part and then of the imaginary part,
so the one on top of the float stack is the last in that order. A real root has an imaginary part of exactly 0; `ZCUBIC` takes the real roots
from `CUBIC` and, when only one is real, the pair from the quadratic that is left after
dividing it out. A leading coefficient of 0 throws -264.

`POLY-ROOTS` and `ZPOLY-ROOTS` use Laguerre's method with complex numbers: they find a
root from the start 0, divide it out (deflation), go on with the smaller polynomial, and
finally polish every root on the whole polynomial. They return the real and the imaginary
parts as two vectors that the program `FREE`s, sorted in the same way. A multiple root is the hard case for any method: it is found with fewer correct
digits than a simple root, and the accuracy varies from root to root. The zero polynomial throws
-264; a constant has no roots (two vectors of length 0). The work is charged in proportion to
the square of the degree, so a very high degree stops at the limits.

The [example](examples/complex.fs) shows all of this:

```
(1+2i)(3+4i) = -5. + 10. i
(1+2i)/(3+4i) = 0.44 + 0.08 i
sqrt(-4) = 0. + 2. i
e^(i pi) = -1. + 0. i
i^i = 0.20788 + 0. i
|3+4i| = 5. 
(1+i)^8 = 16. + 0. i
x^2 + 2x + 5: -1. + 2. i ,  -1. - 2. i
x^3 - 1: 1. + 0. i ,  -0.5 + 0.866025 i ,  -0.5 - 0.866025 i
x^4 + 4: -1. - 1. i;  -1. + 1. i;  1. - 1. i;  1. + 1. i;  
bin 5 has 32. 
bin 12 has 16. 
sample 0 again: 1.5 
```

## Conditioning

The Hilbert matrices are the classic test: the condition number grows fast, and a
solution loses digits with it. This is measured with the example (solving H x = b, where b
is H times a vector of ones, and looking at the largest distance of x to 1):

| n | condition number | largest error |
|---|---|---|
| 3 | 7.480E2 | 9.992E-15 |
| 5 | 9.437E5 | 6.168E-13 |
| 7 | 9.852E8 | 2.566E-8 |
| 9 | 1.100E12 | 1.958E-5 |

## Limits

The work is charged against the limits of the sandbox: a matrix product, a
factorization and an inverse cost steps in proportion to their number of floating point
operations (one step per 8 operations), so a large matrix cannot run past `--max-steps` or
`--max-time`. The arrays need room in the Forth memory: in the default 1 MiB an inverse of
a Hilbert matrix of 250 by 250 works and one of 300 by 300 does not; start with `-m 16M`
for larger ones.
