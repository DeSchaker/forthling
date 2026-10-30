//! The data stack, the return stack and the double-cell helpers.

use crate::*;

pub(crate) const STACK_CELLS: usize = 1 << 16; // data stack size, reported by ENVIRONMENT?
pub(crate) const RSTACK_CELLS: usize = 1 << 16; // return stack size, idem

impl Forth {
    pub(crate) fn push(&mut self, v: Cell) {
        self.stack.push(v);
    }

    pub(crate) fn pop(&mut self) -> Res<Cell> {
        self.stack.pop().ok_or(Throw::new(-4))
    }

    pub(crate) fn check_stack(&self) -> Res {
        if self.stack.len() > STACK_CELLS {
            return Err(Throw::new(-3));
        }
        Ok(())
    }

    pub(crate) fn pop2(&mut self) -> Res<(Cell, Cell)> {
        let b = self.pop()?;
        let a = self.pop()?;
        Ok((a, b))
    }

    pub(crate) fn binop(&mut self, op: impl Fn(Cell, Cell) -> Cell) -> Res {
        let (a, b) = self.pop2()?;
        self.push(op(a, b));
        Ok(())
    }

    /// Pushes a double cell: the low cell first, the high cell on top.
    pub(crate) fn push_d(&mut self, d: i128) {
        self.push_ud(d as u128);
    }

    pub(crate) fn push_ud(&mut self, d: u128) {
        self.push(d as u64 as Cell);
        self.push((d >> 64) as u64 as Cell);
    }

    pub(crate) fn d_unop(&mut self, op: impl Fn(i128) -> i128) -> Res {
        let d = self.pop_d()?;
        self.push_d(op(d));
        Ok(())
    }

    pub(crate) fn d_binop(&mut self, op: impl Fn(i128, i128) -> i128) -> Res {
        let b = self.pop_d()?;
        let a = self.pop_d()?;
        self.push_d(op(a, b));
        Ok(())
    }

    pub(crate) fn d_compare(&mut self, op: impl Fn(i128, i128) -> bool) -> Res {
        let b = self.pop_d()?;
        let a = self.pop_d()?;
        self.push(flag(op(a, b)));
        Ok(())
    }

    pub(crate) fn pop_d(&mut self) -> Res<i128> {
        Ok(self.pop_ud()? as i128)
    }

    pub(crate) fn pop_ud(&mut self) -> Res<u128> {
        let (lo, hi) = self.pop2()?;
        Ok((hi as u64 as u128) << 64 | lo as u64 as u128)
    }

    /// FM/MOD (floored) or SM/REM (symmetric): ( d n -- rem quot ).
    pub(crate) fn divide(&mut self, floored: bool) -> Res {
        let d = self.pop()? as i128;
        let n = self.pop_d()?;
        if d == 0 {
            return Err(Throw::new(-10));
        }
        // The only overflow, the smallest double divided by -1, is out of range anyway.
        let mut q = n.checked_div(d).ok_or(Throw::new(-11))?;
        let mut r = n % d;
        if floored && r != 0 && (r < 0) != (d < 0) {
            q -= 1;
            r += d;
        }
        let q = Cell::try_from(q).map_err(|_| Throw::new(-11))?;
        self.push(r as Cell);
        self.push(q);
        Ok(())
    }

    /// The run-time part of ABORT".
    pub(crate) fn abort_if(&mut self, msg: &[u8]) -> Res {
        if self.pop()? != 0 {
            return Err(Throw::with(-2, String::from_utf8_lossy(msg)));
        }
        Ok(())
    }

    pub(crate) fn loop_index(&self, depth: usize) -> Res<Cell> {
        let n = self.loops.len();
        if depth < n {
            Ok(self.loops[n - 1 - depth].index)
        } else {
            Err(Throw::new(-26))
        }
    }
}
