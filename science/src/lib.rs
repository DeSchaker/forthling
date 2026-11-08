//! Numerical words for forthling: the word list SCIENCE. It has float vectors
//! and matrices and linear algebra (LU, solve, determinant, inverse,
//! Gauss-Jordan), and special functions (gamma, error function, incomplete
//! gamma and beta, logistic; module `special`) and numerical integration and
//! ordinary differential equations (`calculus.fs`, written in Forth because the
//! words call a function that the program passes as an execution token). The
//! arrays are plain Rust on the Forth memory, so nothing reaches outside Forth,
//! no permission is needed and the words work in the sandbox, in the MCP server
//! and in the playground.
//!
//! An array is a block on the heap that the program `FREE`s: a header of three
//! cells (a tag, the rows, the columns) followed by the floats, a row after a
//! row. A word that makes an array pushes the address of the block, and the
//! words check that address, so a wrong address or a wrong kind of array is an
//! error and never a read of other memory. `FV[]` and `MAT[]` give the address
//! of an element, to use with `F@` and `F!`. A word that calculates a new array
//! returns a new block and leaves its arguments as they were.
//!
//! ```
//! use forthling::Forth;
//!
//! let mut f = Forth::new();
//! f.install(&forthling_science::Science);
//! f.interpret("ALSO SCIENCE  2e 3e 2 FV-FROM-STACK  1e 4e 2 FV-FROM-STACK FV-DOT F>S .")?;
//! assert_eq!(f.output(), b"14 ");
//! # Ok::<(), forthling::Throw>(())
//! ```
//!
//! The algorithms are built from the mathematics, not from the Forth
//! Scientific Library.

#![allow(clippy::many_single_char_names, reason = "the names of the mathematics: a, b, x, n")]

mod special;

use forthling::{Cell, Extension, Forth, Res, Throw, to_cell};

/// The throw of an array of the wrong kind or shape, of a singular matrix and
/// of an array larger than the Forth memory.
pub const SCIENCE_ERROR: Cell = -264;

/// The standard throw for an index or a size that is not valid.
const INVALID_ARGUMENT: Cell = -24;
/// The standard throw for an address that is not in memory.
const INVALID_ADDRESS: Cell = -9;

/// Cells in front of the floats: the tag, the rows and the columns.
const HEADER: usize = 24;
const TAG_VEC: i64 = 0x4654_4C56_4543_0001;
const TAG_MAT: i64 = 0x4654_4C4D_4154_0001;

/// The extension: `forth.install(&Science)`.
pub struct Science;

/// An array copied out of the Forth memory: `rows` by `cols` floats, a row
/// after a row. A vector has one column.
struct Mat {
    rows: usize,
    cols: usize,
    v: Vec<f64>,
}

impl Mat {
    fn at(&self, r: usize, c: usize) -> f64 {
        self.v[r * self.cols + c]
    }

    fn square(&self, what: &str) -> Res<usize> {
        if self.rows == self.cols {
            Ok(self.rows)
        } else {
            Err(error(format!("{what} needs a square matrix, not {} by {}", self.rows, self.cols)))
        }
    }
}

fn error(detail: impl Into<String>) -> Throw {
    Throw::with(SCIENCE_ERROR, detail)
}

fn read_i64(mem: &[u8], a: usize) -> i64 {
    i64::from_le_bytes(mem[a..a + 8].try_into().expect("a slice of 8 bytes"))
}

fn read_f64(mem: &[u8], a: usize) -> f64 {
    f64::from_le_bytes(mem[a..a + 8].try_into().expect("a slice of 8 bytes"))
}

#[allow(clippy::cast_precision_loss, reason = "sizes and indexes are far below 2^53")]
fn float(n: usize) -> f64 {
    n as f64
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is checked to be a whole number in range first"
)]
fn index_of(r: f64, limit: usize) -> Option<usize> {
    (r >= 0.0 && r.fract() == 0.0 && r < float(limit)).then_some(r as usize)
}

fn kind(tag: i64) -> &'static str {
    if tag == TAG_VEC { "a float vector" } else { "a matrix" }
}

/// Pops an address and copies the array behind it, checking the header and
/// that the floats lie in memory; counts the copy as work.
fn load(f: &mut Forth, tag: i64) -> Res<Mat> {
    let addr = f.pop()?;
    let mem = f.memory();
    let a = usize::try_from(addr)
        .ok()
        .filter(|a| a.checked_add(HEADER).is_some_and(|end| end <= mem.len()))
        .ok_or(Throw::new(INVALID_ADDRESS))?;
    if read_i64(mem, a) != tag {
        return Err(error(format!("{addr} is not {}", kind(tag))));
    }
    let bad = || error(format!("the header at {addr} is not valid"));
    let rows = usize::try_from(read_i64(mem, a + 8)).map_err(|_| bad())?;
    let cols = usize::try_from(read_i64(mem, a + 16)).map_err(|_| bad())?;
    let n = rows
        .checked_mul(cols)
        .filter(|n| {
            n.checked_mul(8)
                .and_then(|b| b.checked_add(a + HEADER))
                .is_some_and(|end| end <= mem.len())
        })
        .ok_or_else(bad)?;
    let v = (0..n).map(|i| read_f64(mem, a + HEADER + 8 * i)).collect();
    work(f, n)?;
    Ok(Mat { rows, cols, v })
}

fn load_vec(f: &mut Forth) -> Res<Mat> {
    load(f, TAG_VEC)
}

fn load_mat(f: &mut Forth) -> Res<Mat> {
    load(f, TAG_MAT)
}

/// The bytes of an array of `rows` by `cols`, or an error when that is larger
/// than the Forth memory.
fn bytes_for(f: &Forth, rows: usize, cols: usize) -> Res<usize> {
    rows.checked_mul(cols)
        .and_then(|n| n.checked_mul(8))
        .and_then(|b| b.checked_add(HEADER))
        .filter(|&b| b <= f.memory().len())
        .ok_or_else(|| error(format!("{rows} by {cols} floats do not fit in the Forth memory")))
}

/// A new array of zeros.
fn zeros(f: &Forth, rows: usize, cols: usize) -> Res<Mat> {
    bytes_for(f, rows, cols)?;
    Ok(Mat { rows, cols, v: vec![0.0; rows * cols] })
}

/// Copies an array to a new block on the heap and pushes its address.
///
/// # Errors
/// Throws -59 when the heap is full.
fn store(f: &mut Forth, tag: i64, m: &Mat) -> Res {
    let bytes = bytes_for(f, m.rows, m.cols)?;
    work(f, m.v.len())?;
    let mut buf = Vec::with_capacity(bytes);
    buf.extend_from_slice(&tag.to_le_bytes());
    buf.extend_from_slice(&to_cell(m.rows).to_le_bytes());
    buf.extend_from_slice(&to_cell(m.cols).to_le_bytes());
    for r in &m.v {
        buf.extend_from_slice(&r.to_le_bytes());
    }
    f.push_bytes(&buf)?;
    f.pop()?; // the length; the program only needs the address
    Ok(())
}

fn pop_size(f: &mut Forth) -> Res<usize> {
    let n = f.pop()?;
    usize::try_from(n).map_err(|_| Throw::with(INVALID_ARGUMENT, format!("{n} is not a size")))
}

fn same_shape(a: &Mat, b: &Mat, what: &str) -> Res {
    if a.rows == b.rows && a.cols == b.cols {
        Ok(())
    } else {
        Err(error(format!(
            "{what} needs arrays of one shape, not {} by {} and {} by {}",
            a.rows, a.cols, b.rows, b.cols
        )))
    }
}

/// Pops `rows * cols` floats from the float stack, the last one on top, into
/// a new array (row after row).
fn from_stack(f: &mut Forth, rows: usize, cols: usize) -> Res<Mat> {
    let mut m = zeros(f, rows, cols)?;
    for i in (0..m.v.len()).rev() {
        m.v[i] = f.fpop()?;
    }
    Ok(m)
}

fn matmul(a: &Mat, b: &Mat) -> Res<Mat> {
    if a.cols != b.rows {
        return Err(error(format!(
            "cannot multiply {} by {} with {} by {}",
            a.rows, a.cols, b.rows, b.cols
        )));
    }
    let mut v = vec![0.0; a.rows * b.cols];
    for i in 0..a.rows {
        for k in 0..a.cols {
            let x = a.at(i, k);
            for j in 0..b.cols {
                v[i * b.cols + j] += x * b.at(k, j);
            }
        }
    }
    Ok(Mat { rows: a.rows, cols: b.cols, v })
}

/// The largest row sum of the absolute values (the infinity norm).
fn norm_inf(a: &Mat) -> f64 {
    (0..a.rows).map(|i| (0..a.cols).map(|j| a.at(i, j).abs()).sum::<f64>()).fold(0.0, f64::max)
}

/// LU factorization with partial pivoting of a square matrix: L (with a unit
/// diagonal) and U in one array, the row order, and the sign of that
/// permutation; `None` when a pivot is zero (the matrix is singular).
fn lu(a: &Mat) -> Option<(Vec<f64>, Vec<usize>, i64)> {
    let n = a.rows;
    let mut m = a.v.clone();
    let mut piv: Vec<usize> = (0..n).collect();
    let mut sign = 1;
    for c in 0..n {
        let p = (c..n).max_by(|&x, &y| m[x * n + c].abs().total_cmp(&m[y * n + c].abs()))?;
        if m[p * n + c] == 0.0 {
            return None;
        }
        if p != c {
            for j in 0..n {
                m.swap(p * n + j, c * n + j);
            }
            piv.swap(p, c);
            sign = -sign;
        }
        for r in c + 1..n {
            let factor = m[r * n + c] / m[c * n + c];
            m[r * n + c] = factor;
            for j in c + 1..n {
                m[r * n + j] -= factor * m[c * n + j];
            }
        }
    }
    Some((m, piv, sign))
}

/// Solves with a factorization: permute b, then forward and back substitution.
fn lu_solve(lu: &[f64], piv: &[usize], b: &[f64]) -> Vec<f64> {
    let n = piv.len();
    let mut x: Vec<f64> = piv.iter().map(|&p| b[p]).collect();
    for i in 0..n {
        for j in 0..i {
            x[i] -= lu[i * n + j] * x[j];
        }
    }
    for i in (0..n).rev() {
        for j in i + 1..n {
            x[i] -= lu[i * n + j] * x[j];
        }
        x[i] /= lu[i * n + i];
    }
    x
}

/// Gauss-Jordan elimination with partial pivoting: solves A X = B for a matrix
/// B of several right-hand sides; `None` when A is singular.
fn gauss_jordan(a: &Mat, b: &Mat) -> Option<Mat> {
    let n = a.rows;
    let m = b.cols;
    let mut a = a.v.clone();
    let mut x = b.v.clone();
    for c in 0..n {
        let p = (c..n).max_by(|&r, &s| a[r * n + c].abs().total_cmp(&a[s * n + c].abs()))?;
        if a[p * n + c] == 0.0 {
            return None;
        }
        if p != c {
            for j in 0..n {
                a.swap(p * n + j, c * n + j);
            }
            for j in 0..m {
                x.swap(p * m + j, c * m + j);
            }
        }
        let pivot = a[c * n + c];
        for j in 0..n {
            a[c * n + j] /= pivot;
        }
        for j in 0..m {
            x[c * m + j] /= pivot;
        }
        for r in (0..n).filter(|&r| r != c) {
            let factor = a[r * n + c];
            if factor != 0.0 {
                for j in 0..n {
                    a[r * n + j] -= factor * a[c * n + j];
                }
                for j in 0..m {
                    x[r * m + j] -= factor * x[c * m + j];
                }
            }
        }
    }
    Some(Mat { rows: n, cols: m, v: x })
}

fn identity(n: usize) -> Mat {
    let mut v = vec![0.0; n * n];
    for i in 0..n {
        v[i * n + i] = 1.0;
    }
    Mat { rows: n, cols: n, v }
}

fn singular() -> Throw {
    error("the matrix is singular")
}

/// Counts the work of `flops` float operations as steps: `charge` counts
/// bytes, one step per 64 of them, so a float operation is 8 bytes.
fn work(f: &mut Forth, flops: usize) -> Res {
    f.charge(flops.saturating_mul(8))
}

/// Counts the work of an O(n^3) algorithm.
fn charge_cube(f: &mut Forth, n: usize) -> Res {
    work(f, n.saturating_mul(n).saturating_mul(n))
}

/// A word of the SCIENCE word list; the word list exists once installed.
fn word(f: &mut Forth, name: &str, word: impl Fn(&mut Forth) -> Res + 'static) {
    let wid = f.wordlist("SCIENCE");
    f.add_word(wid, name, word).expect("SCIENCE is a word list and names are not empty");
}

/// Combines two arrays of one shape element by element into a new array.
fn zip(f: &mut Forth, tag: i64, what: &str, op: impl Fn(f64, f64) -> f64) -> Res {
    let load = if tag == TAG_VEC { load_vec } else { load_mat };
    let b = load(f)?;
    let a = load(f)?;
    same_shape(&a, &b, what)?;
    let v = a.v.iter().zip(&b.v).map(|(&x, &y)| op(x, y)).collect();
    store(f, tag, &Mat { rows: a.rows, cols: a.cols, v })
}

fn install_vectors(f: &mut Forth) {
    // FV-NEW ( n -- a ): a vector of n floats, all zero.
    word(f, "FV-NEW", |f| {
        let n = pop_size(f)?;
        let m = zeros(f, n, 1)?;
        store(f, TAG_VEC, &m)
    });
    // FV-FROM-STACK ( n -- a ) ( F: x1 ... xn -- ): a vector of the n floats.
    word(f, "FV-FROM-STACK", |f| {
        let n = pop_size(f)?;
        let m = from_stack(f, n, 1)?;
        store(f, TAG_VEC, &m)
    });
    // FV-LEN ( a -- n )
    word(f, "FV-LEN", |f| {
        let a = load_vec(f)?;
        f.push(to_cell(a.rows));
        Ok(())
    });
    // FV[] ( i a -- f-addr ): the address of element i, to use with F@ and F!.
    word(f, "FV[]", |f| {
        let a = f.pop()?;
        let i = f.pop()?;
        f.push(a);
        let m = load_vec(f)?;
        let at = usize::try_from(i)
            .ok()
            .filter(|&i| i < m.rows)
            .ok_or_else(|| Throw::with(INVALID_ARGUMENT, format!("index {i} of {}", m.rows)))?;
        f.push(a + to_cell(HEADER + 8 * at));
        Ok(())
    });
    // FV-FILL ( a -- ) ( F: x -- ): sets every element to x.
    word(f, "FV-FILL", |f| {
        let x = f.fpop()?;
        let a = f.pop()?;
        f.push(a);
        let m = load_vec(f)?;
        let start = usize::try_from(a).expect("the address was checked by load");
        for i in 0..m.rows {
            let at = start + HEADER + 8 * i;
            f.memory_mut()[at..at + 8].copy_from_slice(&x.to_le_bytes());
        }
        Ok(())
    });
    // FV-DOT ( a b -- ) ( F: -- r ): the dot product.
    word(f, "FV-DOT", |f| {
        let b = load_vec(f)?;
        let a = load_vec(f)?;
        same_shape(&a, &b, "FV-DOT")?;
        f.fpush(a.v.iter().zip(&b.v).map(|(x, y)| x * y).sum())
    });
    // FV-NORM ( a -- ) ( F: -- r ): the Euclidean length.
    word(f, "FV-NORM", |f| {
        let a = load_vec(f)?;
        f.fpush(a.v.iter().map(|x| x * x).sum::<f64>().sqrt())
    });
    // FV-SCALE ( a -- c ) ( F: x -- ): a new vector, every element times x.
    word(f, "FV-SCALE", |f| {
        let x = f.fpop()?;
        let mut a = load_vec(f)?;
        a.v.iter_mut().for_each(|e| *e *= x);
        store(f, TAG_VEC, &a)
    });
    // FV+ ( a b -- c ) and FV- ( a b -- c ): a new vector.
    word(f, "FV+", |f| zip(f, TAG_VEC, "FV+", |x, y| x + y));
    word(f, "FV-", |f| zip(f, TAG_VEC, "FV-", |x, y| x - y));
}

fn install_matrices(f: &mut Forth) {
    // MAT-NEW ( rows cols -- a ): a matrix of zeros.
    word(f, "MAT-NEW", |f| {
        let cols = pop_size(f)?;
        let rows = pop_size(f)?;
        let m = zeros(f, rows, cols)?;
        store(f, TAG_MAT, &m)
    });
    // MAT-FROM-STACK ( rows cols -- a ) ( F: x11 x12 ... -- ): row by row.
    word(f, "MAT-FROM-STACK", |f| {
        let cols = pop_size(f)?;
        let rows = pop_size(f)?;
        let m = from_stack(f, rows, cols)?;
        store(f, TAG_MAT, &m)
    });
    // MAT-IDENTITY ( n -- a ) and MAT-HILBERT ( n -- a ): H(i,j) = 1/(i+j+1).
    word(f, "MAT-IDENTITY", |f| {
        let n = pop_size(f)?;
        bytes_for(f, n, n)?;
        store(f, TAG_MAT, &identity(n))
    });
    word(f, "MAT-HILBERT", |f| {
        let n = pop_size(f)?;
        let mut m = zeros(f, n, n)?;
        for i in 0..n {
            for j in 0..n {
                m.v[i * n + j] = 1.0 / float(i + j + 1);
            }
        }
        store(f, TAG_MAT, &m)
    });
    // MAT-ROWS ( a -- n ) and MAT-COLS ( a -- n )
    word(f, "MAT-ROWS", |f| {
        let a = load_mat(f)?;
        f.push(to_cell(a.rows));
        Ok(())
    });
    word(f, "MAT-COLS", |f| {
        let a = load_mat(f)?;
        f.push(to_cell(a.cols));
        Ok(())
    });
    // MAT[] ( i j a -- f-addr ): the address of the element in row i and
    // column j (counted from 0), to use with F@ and F!.
    word(f, "MAT[]", |f| {
        let a = f.pop()?;
        let j = f.pop()?;
        let i = f.pop()?;
        f.push(a);
        let m = load_mat(f)?;
        let pick = |x: Cell, limit: usize| {
            usize::try_from(x)
                .ok()
                .filter(|&x| x < limit)
                .ok_or_else(|| Throw::with(INVALID_ARGUMENT, format!("index {x} of {limit}")))
        };
        let at = pick(i, m.rows)? * m.cols + pick(j, m.cols)?;
        f.push(a + to_cell(HEADER + 8 * at));
        Ok(())
    });
    // MAT-T ( a -- c ): the transpose.
    word(f, "MAT-T", |f| {
        let a = load_mat(f)?;
        let mut t = zeros(f, a.cols, a.rows)?;
        for i in 0..a.rows {
            for j in 0..a.cols {
                t.v[j * a.rows + i] = a.at(i, j);
            }
        }
        store(f, TAG_MAT, &t)
    });
    // MAT+ ( a b -- c ), MAT- ( a b -- c )
    word(f, "MAT+", |f| zip(f, TAG_MAT, "MAT+", |x, y| x + y));
    word(f, "MAT-", |f| zip(f, TAG_MAT, "MAT-", |x, y| x - y));
    // MAT-SCALE ( a -- c ) ( F: x -- ): every element times x.
    word(f, "MAT-SCALE", |f| {
        let x = f.fpop()?;
        let mut a = load_mat(f)?;
        a.v.iter_mut().for_each(|e| *e *= x);
        store(f, TAG_MAT, &a)
    });
    // MAT* ( a b -- c ): the matrix product.
    word(f, "MAT*", |f| {
        let b = load_mat(f)?;
        let a = load_mat(f)?;
        work(f, a.rows.saturating_mul(a.cols).saturating_mul(b.cols))?;
        store(f, TAG_MAT, &matmul(&a, &b)?)
    });
    // MAT*V ( a v -- w ): a matrix times a vector.
    word(f, "MAT*V", |f| {
        let v = load_vec(f)?;
        let a = load_mat(f)?;
        work(f, a.rows.saturating_mul(a.cols))?;
        store(f, TAG_VEC, &matmul(&a, &v)?)
    });
    // MAT-NORM ( a -- ) ( F: -- r ): the infinity norm, the largest row sum
    // of absolute values.
    word(f, "MAT-NORM", |f| {
        let a = load_mat(f)?;
        f.fpush(norm_inf(&a))
    });
}

fn install_algebra(f: &mut Forth) {
    // MAT-LU ( a -- lu piv sign ): LU factorization with partial pivoting.
    // lu holds L (unit diagonal, not stored) below and U on and above the
    // diagonal; piv is a vector with the original row of each row; sign is 1
    // or -1, the sign of that permutation.
    word(f, "MAT-LU", |f| {
        let a = load_mat(f)?;
        let n = a.square("MAT-LU")?;
        charge_cube(f, n)?;
        let (m, piv, sign) = lu(&a).ok_or_else(singular)?;
        store(f, TAG_MAT, &Mat { rows: n, cols: n, v: m })?;
        let piv = Mat { rows: n, cols: 1, v: piv.into_iter().map(float).collect() };
        store(f, TAG_VEC, &piv)?;
        f.push(sign);
        Ok(())
    });
    // LU-SOLVE ( b lu piv -- x ): solves A x = b with the factorization of A.
    word(f, "LU-SOLVE", |f| {
        let piv = load_vec(f)?;
        let m = load_mat(f)?;
        let b = load_vec(f)?;
        let n = m.square("LU-SOLVE")?;
        if piv.rows != n || b.rows != n {
            return Err(error(format!("LU-SOLVE: a {n} by {n} matrix needs vectors of {n}")));
        }
        let order: Vec<usize> = piv
            .v
            .iter()
            .map(|&p| index_of(p, n))
            .collect::<Option<_>>()
            .ok_or_else(|| error("LU-SOLVE: the pivot vector is not a row order"))?;
        work(f, n.saturating_mul(n))?;
        let x = lu_solve(&m.v, &order, &b.v);
        store(f, TAG_VEC, &Mat { rows: n, cols: 1, v: x })
    });
    // MAT-SOLVE ( b a -- x ): solves A x = b for a vector b.
    word(f, "MAT-SOLVE", |f| {
        let a = load_mat(f)?;
        let b = load_vec(f)?;
        let n = a.square("MAT-SOLVE")?;
        if b.rows != n {
            return Err(error(format!("MAT-SOLVE: a {n} by {n} matrix needs a vector of {n}")));
        }
        charge_cube(f, n)?;
        let (m, piv, _) = lu(&a).ok_or_else(singular)?;
        let x = lu_solve(&m, &piv, &b.v);
        store(f, TAG_VEC, &Mat { rows: n, cols: 1, v: x })
    });
    // MAT-DET ( a -- ) ( F: -- r ): the determinant; 0 for a singular matrix.
    word(f, "MAT-DET", |f| {
        let a = load_mat(f)?;
        let n = a.square("MAT-DET")?;
        charge_cube(f, n)?;
        let det = lu(&a).map_or(0.0, |(m, _, sign)| {
            (0..n).map(|i| m[i * n + i]).product::<f64>() * float_sign(sign)
        });
        f.fpush(det)
    });
    // MAT-INV ( a -- c ): the inverse, by Gauss-Jordan elimination.
    word(f, "MAT-INV", |f| {
        let a = load_mat(f)?;
        let n = a.square("MAT-INV")?;
        charge_cube(f, n)?;
        let inv = gauss_jordan(&a, &identity(n)).ok_or_else(singular)?;
        store(f, TAG_MAT, &inv)
    });
    // MAT-COND ( a -- ) ( F: -- r ): the condition number in the infinity
    // norm, the norm of A times the norm of its inverse.
    word(f, "MAT-COND", |f| {
        let a = load_mat(f)?;
        let n = a.square("MAT-COND")?;
        charge_cube(f, n)?;
        let inv = gauss_jordan(&a, &identity(n)).ok_or_else(singular)?;
        f.fpush(norm_inf(&a) * norm_inf(&inv))
    });
    // MAT-GAUSS-JORDAN ( a b -- x ): solves A X = B for a matrix B with one
    // column for each right-hand side.
    word(f, "MAT-GAUSS-JORDAN", |f| {
        let b = load_mat(f)?;
        let a = load_mat(f)?;
        let n = a.square("MAT-GAUSS-JORDAN")?;
        if b.rows != n {
            return Err(error(format!(
                "MAT-GAUSS-JORDAN: a {n} by {n} matrix needs a right-hand side of {n} rows"
            )));
        }
        work(f, n.saturating_mul(n).saturating_mul(n + b.cols))?;
        let x = gauss_jordan(&a, &b).ok_or_else(singular)?;
        store(f, TAG_MAT, &x)
    });
}

fn float_sign(sign: i64) -> f64 {
    if sign < 0 { -1.0 } else { 1.0 }
}

impl Extension for Science {
    fn name(&self) -> &'static str {
        "FORTHLING-SCIENCE"
    }

    fn install(&self, f: &mut Forth) {
        f.add_help(include_str!("help.txt"));
        install_vectors(f);
        install_matrices(f);
        install_algebra(f);
        special::install(f);
        // Integration and differential equations call a function the program
        // passes as an execution token, so they are written in Forth.
        f.interpret("ALSO SCIENCE DEFINITIONS").expect("SCIENCE is a word list");
        let result = f.interpret(include_str!("calculus.fs"));
        f.interpret("PREVIOUS FORTH DEFINITIONS").expect("FORTH is a word list");
        result.expect("calculus.fs must compile");
    }
}
