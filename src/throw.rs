//! Errors: THROW codes and their messages.

use crate::*;
use std::fmt;

// The THROW codes of table 9.1 that forthling uses.
pub(crate) const ABORT: Cell = -1;
pub(crate) const ABORT_QUOTE: Cell = -2;
pub(crate) const STACK_OVERFLOW: Cell = -3;
pub(crate) const STACK_UNDERFLOW: Cell = -4;
pub(crate) const RSTACK_OVERFLOW: Cell = -5;
pub(crate) const RSTACK_UNDERFLOW: Cell = -6;
pub(crate) const DICTIONARY_OVERFLOW: Cell = -8;
pub(crate) const INVALID_ADDRESS: Cell = -9;
pub(crate) const DIVISION_BY_ZERO: Cell = -10;
pub(crate) const OUT_OF_RANGE: Cell = -11;
pub(crate) const UNDEFINED_WORD: Cell = -13;
pub(crate) const COMPILE_ONLY: Cell = -14;
pub(crate) const INVALID_FORGET: Cell = -15;
pub(crate) const ZERO_LENGTH_NAME: Cell = -16;
pub(crate) const PICTURED_OVERFLOW: Cell = -17;
pub(crate) const PARSED_OVERFLOW: Cell = -18;
pub(crate) const UNSUPPORTED: Cell = -21;
pub(crate) const CONTROL_MISMATCH: Cell = -22;
pub(crate) const INVALID_ARGUMENT: Cell = -24;
pub(crate) const NO_LOOP: Cell = -26;
pub(crate) const COMPILER_NESTING: Cell = -29;
pub(crate) const NOT_CREATED: Cell = -31;
pub(crate) const INVALID_NAME: Cell = -32;
pub(crate) const BLOCK_READ: Cell = -33;
pub(crate) const BLOCK_WRITE: Cell = -34;
pub(crate) const INVALID_BLOCK: Cell = -35;
pub(crate) const FILE_IO: Cell = -37;
pub(crate) const NO_SUCH_FILE: Cell = -38;
pub(crate) const UNEXPECTED_EOF: Cell = -39;
pub(crate) const FSTACK_OVERFLOW: Cell = -44;
pub(crate) const FSTACK_UNDERFLOW: Cell = -45;
pub(crate) const ORDER_OVERFLOW: Cell = -49;
pub(crate) const ORDER_UNDERFLOW: Cell = -50;
pub(crate) const ALLOCATE_FAILED: Cell = -59;
pub(crate) const FREE_FAILED: Cell = -60;
pub(crate) const RESIZE_FAILED: Cell = -61;
pub(crate) const CLOSE_FILE_FAILED: Cell = -62;
pub(crate) const CREATE_FILE_FAILED: Cell = -63;
pub(crate) const DELETE_FILE_FAILED: Cell = -64;
pub(crate) const FILE_POSITION_FAILED: Cell = -65;
pub(crate) const FILE_SIZE_FAILED: Cell = -66;
pub(crate) const FILE_STATUS_FAILED: Cell = -67;
pub(crate) const FLUSH_FILE_FAILED: Cell = -68;
pub(crate) const OPEN_FILE_FAILED: Cell = -69;
pub(crate) const READ_FILE_FAILED: Cell = -70;
pub(crate) const READ_LINE_FAILED: Cell = -71;
pub(crate) const RENAME_FILE_FAILED: Cell = -72;
pub(crate) const REPOSITION_FILE_FAILED: Cell = -73;
pub(crate) const RESIZE_FILE_FAILED: Cell = -74;
pub(crate) const WRITE_FILE_FAILED: Cell = -75;
pub(crate) const WRITE_LINE_FAILED: Cell = -76;
pub(crate) const SUBSTITUTE_FAILED: Cell = -78;
pub(crate) const REPLACES_FAILED: Cell = -79;

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
            ABORT | ABORT_QUOTE => "aborted",
            STACK_OVERFLOW => "stack overflow",
            STACK_UNDERFLOW => "stack underflow",
            RSTACK_OVERFLOW => "return stack overflow",
            RSTACK_UNDERFLOW => "return stack underflow",
            DICTIONARY_OVERFLOW => "dictionary overflow",
            INVALID_ADDRESS => "invalid memory address",
            DIVISION_BY_ZERO => "division by zero",
            OUT_OF_RANGE => "result out of range",
            UNDEFINED_WORD => "undefined word",
            COMPILE_ONLY => "interpreting a compile-only word",
            INVALID_FORGET => "invalid FORGET",
            ZERO_LENGTH_NAME => "attempt to use zero-length string as a name",
            PICTURED_OVERFLOW => "pictured numeric output string overflow",
            PARSED_OVERFLOW => "parsed string overflow",
            UNSUPPORTED => "unsupported operation",
            CONTROL_MISMATCH => "control structure mismatch",
            INVALID_ARGUMENT => "invalid numeric argument",
            NO_LOOP => "loop parameters unavailable",
            COMPILER_NESTING => "compiler nesting",
            NOT_CREATED => ">BODY used on non-CREATEd definition",
            INVALID_NAME => "invalid name argument",
            BLOCK_READ => "block read exception",
            BLOCK_WRITE => "block write exception",
            INVALID_BLOCK => "invalid block number",
            FILE_IO => "file I/O exception",
            NO_SUCH_FILE => "non-existent file",
            UNEXPECTED_EOF => "unexpected end of file",
            FSTACK_OVERFLOW => "floating-point stack overflow",
            FSTACK_UNDERFLOW => "floating-point stack underflow",
            ORDER_OVERFLOW => "search-order overflow",
            ORDER_UNDERFLOW => "search-order underflow",
            ALLOCATE_FAILED => "ALLOCATE",
            FREE_FAILED => "FREE",
            RESIZE_FAILED => "RESIZE",
            CLOSE_FILE_FAILED => "CLOSE-FILE exception",
            CREATE_FILE_FAILED => "CREATE-FILE exception",
            DELETE_FILE_FAILED => "DELETE-FILE exception",
            FILE_POSITION_FAILED => "FILE-POSITION exception",
            FILE_SIZE_FAILED => "FILE-SIZE exception",
            FILE_STATUS_FAILED => "FILE-STATUS exception",
            FLUSH_FILE_FAILED => "FLUSH-FILE exception",
            OPEN_FILE_FAILED => "OPEN-FILE exception",
            READ_FILE_FAILED => "READ-FILE exception",
            READ_LINE_FAILED => "READ-LINE exception",
            RENAME_FILE_FAILED => "RENAME-FILE exception",
            REPOSITION_FILE_FAILED => "REPOSITION-FILE exception",
            RESIZE_FILE_FAILED => "RESIZE-FILE exception",
            WRITE_FILE_FAILED => "WRITE-FILE exception",
            WRITE_LINE_FAILED => "WRITE-LINE exception",
            SUBSTITUTE_FAILED => "SUBSTITUTE",
            REPLACES_FAILED => "REPLACES",
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
