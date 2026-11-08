# Numerical words for forthling: linear algebra and special functions

The word list `SCIENCE` for [forthling](https://github.com/DeSchaker/forthling): vectors of
floats, matrices, and the linear algebra on them (solving equations, determinants,
inverses and condition numbers), and the special functions: gamma, error function,
incomplete gamma and beta, logistic. It is plain Rust, built from the mathematics, with no
dependencies, and it reaches nothing outside Forth: no permission is needed, so the words
work in the sandbox, in the MCP server, in the Python module and in the playground, where
they are built in.

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
