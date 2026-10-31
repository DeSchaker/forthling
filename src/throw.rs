//! Errors: THROW codes and their messages.

use crate::*;
use std::fmt;

/// System THROW code: INCLUDEs nested deeper than MAX_SOURCES.
pub(crate) const NESTING_TOO_DEEP: Cell = -256;

/// THROW code of QUIT: back to the terminal, keeping the data stack.
pub const QUIT: Cell = -56;

/// A Forth exception: a THROW code from table 9.1 of the standard, or a
/// system code below -255.
#[derive(Debug)]
pub struct Throw {
    pub code: Cell,
    pub(crate) detail: Option<String>,
    pub(crate) location: Option<String>,
}

impl Throw {
    pub(crate) fn new(code: Cell) -> Self {
        Throw { code, detail: None, location: None }
    }

    pub(crate) fn with(code: Cell, detail: impl Into<String>) -> Self {
        Throw { detail: Some(detail.into()), ..Throw::new(code) }
    }

    /// Records where the error happened; the innermost location wins.
    pub(crate) fn at(mut self, path: &str, line: usize) -> Self {
        if self.location.is_none() {
            self.location = Some(format!("{path}:{line}"));
        }
        self
    }

    pub(crate) fn message(&self) -> &'static str {
        match self.code {
            -1 | -2 => "aborted",
            -3 => "stack overflow",
            -4 => "stack underflow",
            -5 => "return stack overflow",
            -6 => "return stack underflow",
            -8 => "dictionary overflow",
            -9 => "invalid memory address",
            -10 => "division by zero",
            -11 => "result out of range",
            -13 => "undefined word",
            -14 => "interpreting a compile-only word",
            -16 => "attempt to use zero-length string as a name",
            -18 => "parsed string overflow",
            -21 => "unsupported operation",
            -22 => "control structure mismatch",
            -24 => "invalid numeric argument",
            -26 => "loop parameters unavailable",
            -17 => "pictured numeric output string overflow",
            -29 => "compiler nesting",
            -31 => ">BODY used on non-CREATEd definition",
            -32 => "invalid name argument",
            -37 => "file I/O exception",
            -38 => "non-existent file",
            -39 => "unexpected end of file",
            -59 => "ALLOCATE",
            -60 => "FREE",
            -61 => "RESIZE",
            -62 => "CLOSE-FILE exception",
            -63 => "CREATE-FILE exception",
            -64 => "DELETE-FILE exception",
            -65 => "FILE-POSITION exception",
            -66 => "FILE-SIZE exception",
            -67 => "FILE-STATUS exception",
            -68 => "FLUSH-FILE exception",
            -69 => "OPEN-FILE exception",
            -70 => "READ-FILE exception",
            -71 => "READ-LINE exception",
            -72 => "RENAME-FILE exception",
            -73 => "REPOSITION-FILE exception",
            -74 => "RESIZE-FILE exception",
            -75 => "WRITE-FILE exception",
            -76 => "WRITE-LINE exception",
            -78 => "SUBSTITUTE",
            -79 => "REPLACES",
            QUIT => "quit",
            NESTING_TOO_DEEP => "input nesting too deep",
            _ => "exception",
        }
    }
}

impl fmt::Display for Throw {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(loc) = &self.location {
            write!(f, "{loc}: ")?;
        }
        write!(f, "error {}: {}", self.code, self.message())?;
        if let Some(detail) = &self.detail {
            write!(f, ": {detail}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Throw {}
