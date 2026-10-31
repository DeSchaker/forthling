//! Typed words: the macro `forth_word!` adds a word written as a Rust
//! function with typed arguments and a typed result, so that the host does
//! not write `pop` and `push` itself. The traits `FromForth` and
//! `IntoForth` say how each type comes from and goes to the stacks.

use crate::*;

/// A value that a word takes from the stacks: `Cell` (`i64`) and `bool` (a
/// flag: not 0 is true) from the data stack, `f64` from the float stack,
/// and `String` and `Vec<u8>` from a string ( c-addr u ), copied (a String
/// with U+FFFD for bytes that are no UTF-8). A tuple takes its parts in
/// stack order: the last part is the top.
pub trait FromForth: Sized {
    /// The cells it takes from the data stack.
    const CELLS: usize;
    /// The floats it takes from the float stack.
    const FLOATS: usize;

    /// Takes the value from the stacks.
    fn from_forth(f: &mut Forth) -> Res<Self>;
}

/// A value that a word gives back on the stacks: `Cell`, `bool` (as -1 or
/// 0) and `f64` as one item; `String`, `&str` and `Vec<u8>` as a string
/// ( c-addr u ) in ALLOCATEd memory, which the program frees with FREE; a
/// tuple as its parts in order, so that the last is the top; `()` as
/// nothing; and `Res<T>` as T, or the THROW of its error.
pub trait IntoForth {
    /// Puts the value on the stacks.
    fn into_forth(self, f: &mut Forth) -> Res;
}

impl FromForth for Cell {
    const CELLS: usize = 1;
    const FLOATS: usize = 0;
    fn from_forth(f: &mut Forth) -> Res<Self> {
        f.pop()
    }
}

impl FromForth for bool {
    const CELLS: usize = 1;
    const FLOATS: usize = 0;
    fn from_forth(f: &mut Forth) -> Res<Self> {
        Ok(f.pop()? != 0)
    }
}

impl FromForth for f64 {
    const CELLS: usize = 0;
    const FLOATS: usize = 1;
    fn from_forth(f: &mut Forth) -> Res<Self> {
        f.fpop()
    }
}

impl FromForth for Vec<u8> {
    const CELLS: usize = 2;
    const FLOATS: usize = 0;
    fn from_forth(f: &mut Forth) -> Res<Self> {
        let bytes = f.pop_bytes()?;
        f.charge(bytes.len())?;
        Ok(bytes)
    }
}

impl FromForth for String {
    const CELLS: usize = 2;
    const FLOATS: usize = 0;
    fn from_forth(f: &mut Forth) -> Res<Self> {
        let bytes = Vec::<u8>::from_forth(f)?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
}

impl FromForth for () {
    const CELLS: usize = 0;
    const FLOATS: usize = 0;
    fn from_forth(_: &mut Forth) -> Res<Self> {
        Ok(())
    }
}

impl IntoForth for () {
    fn into_forth(self, _: &mut Forth) -> Res {
        Ok(())
    }
}

impl IntoForth for Cell {
    fn into_forth(self, f: &mut Forth) -> Res {
        f.push(self);
        Ok(())
    }
}

impl IntoForth for bool {
    fn into_forth(self, f: &mut Forth) -> Res {
        f.push(flag(self));
        Ok(())
    }
}

impl IntoForth for f64 {
    fn into_forth(self, f: &mut Forth) -> Res {
        f.fpush(self)
    }
}

impl IntoForth for &[u8] {
    fn into_forth(self, f: &mut Forth) -> Res {
        f.charge(self.len())?;
        f.push_bytes(self)
    }
}

impl IntoForth for Vec<u8> {
    fn into_forth(self, f: &mut Forth) -> Res {
        self.as_slice().into_forth(f)
    }
}

impl IntoForth for &str {
    fn into_forth(self, f: &mut Forth) -> Res {
        self.as_bytes().into_forth(f)
    }
}

impl IntoForth for String {
    fn into_forth(self, f: &mut Forth) -> Res {
        self.as_bytes().into_forth(f)
    }
}

impl<T: IntoForth> IntoForth for Res<T> {
    fn into_forth(self, f: &mut Forth) -> Res {
        self?.into_forth(f)
    }
}

/// Checks that the stacks hold what a tuple takes, so that a word that
/// gets too few takes nothing.
fn check_depth(f: &Forth, cells: usize, floats: usize) -> Res {
    if f.stack.len() < cells {
        return Err(Throw::new(STACK_UNDERFLOW));
    }
    if f.fstack.len() < floats {
        return Err(Throw::new(FSTACK_UNDERFLOW));
    }
    Ok(())
}

/// The tuple impls: the parts in order, and the same parts reversed, in
/// which order they are taken from the stacks.
macro_rules! tuple {
    ($($t:ident $v:ident),+ ; $($rt:ident $rv:ident),+) => {
        impl<$($t: FromForth),+> FromForth for ($($t,)+) {
            const CELLS: usize = 0 $(+ $t::CELLS)+;
            const FLOATS: usize = 0 $(+ $t::FLOATS)+;
            fn from_forth(f: &mut Forth) -> Res<Self> {
                check_depth(f, Self::CELLS, Self::FLOATS)?;
                $(let $rv = $rt::from_forth(f)?;)+
                Ok(($($v,)+))
            }
        }

        impl<$($t: IntoForth),+> IntoForth for ($($t,)+) {
            fn into_forth(self, f: &mut Forth) -> Res {
                let ($($v,)+) = self;
                $($v.into_forth(f)?;)+
                Ok(())
            }
        }
    };
}

tuple!(A a; A a);
tuple!(A a, B b; B b, A a);
tuple!(A a, B b, C c; C c, B b, A a);
tuple!(A a, B b, C c, D d; D d, C c, B b, A a);
tuple!(A a, B b, C c, D d, E e; E e, D d, C c, B b, A a);
tuple!(A a, B b, C c, D d, E e, G g; G g, E e, D d, C c, B b, A a);

/// Adds a word written as a Rust function, with typed arguments and
/// result, to a word list; it expands to `Forth::add_word` and gives its
/// `Res`. The arguments are taken in stack order (the last is the top),
/// and the result is put back; see `FromForth` and `IntoForth` for the
/// types. Without `-> T` the word gives nothing back. A result of type
/// `Res<T>` lets the body throw, with `?` or `Err(Throw::new(code))`.
///
/// ```
/// use forthling::{Forth, Res, Throw, forth_word};
///
/// let mut forth = Forth::new();
/// let wid = forth.wordlist("MATHS");
/// forth_word!(forth, wid, "CUBE", (n: i64) -> i64 { n * n * n })?;
/// forth_word!(forth, wid, "HYPOT", (a: f64, b: f64) -> f64 { a.hypot(b) })?;
/// forth_word!(forth, wid, "SHOUT", (s: String) -> String { s.to_uppercase() })?;
/// forth_word!(forth, wid, "DIVMOD", (a: i64, b: i64) -> Res<(i64, i64)> {
///     if b == 0 {
///         return Err(Throw::new(-10));
///     }
///     Ok((a % b, a / b))
/// })?;
/// forth.interpret("ALSO MATHS 3 CUBE . 3E0 4E0 HYPOT F. S\" hi\" SHOUT TYPE 7 2 DIVMOD . .")?;
/// assert_eq!(forth.output(), b"27 5. HI3 1 ");
/// # Ok::<(), Throw>(())
/// ```
#[macro_export]
macro_rules! forth_word {
    ($forth:expr, $wid:expr, $name:expr, ($($arg:ident : $ty:ty),* $(,)?) -> $ret:ty $body:block) => {
        $forth.add_word($wid, $name, move |f: &mut $crate::Forth| -> $crate::Res {
            let ($($arg,)*): ($($ty,)*) = $crate::FromForth::from_forth(f)?;
            // A closure, so that `return` in the body returns its result.
            #[allow(clippy::redundant_closure_call)]
            let result: $ret = (|| -> $ret { $body })();
            $crate::IntoForth::into_forth(result, f)
        })
    };
    ($forth:expr, $wid:expr, $name:expr, ($($arg:ident : $ty:ty),* $(,)?) $body:block) => {
        $crate::forth_word!($forth, $wid, $name, ($($arg: $ty),*) -> () $body)
    };
}
