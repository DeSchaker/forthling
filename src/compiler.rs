//! Compiler helpers: definitions, control-flow markers and TO/IS.

use crate::*;

/// Compile-time control-flow markers: an unresolved forward jump, a
/// backward jump target, or an open DO.
#[derive(Clone, Copy)]
pub(crate) enum Ctrl {
    Orig(usize),
    Dest(usize),
    Do(usize),
    Case,         // an open CASE
    Of(usize),    // an OF waiting for its ENDOF
    EndOf(usize), // an ENDOF jump waiting for ENDCASE
}

impl Forth {
    /// Starts a colon definition (hidden until `;`) and enters compilation state.
    pub(crate) fn start_def(&mut self, name: &str) -> Res<usize> {
        if self.current.is_some() {
            return Err(Throw::new(COMPILER_NESTING));
        }
        let xt = self.new_word(name, Kind::Colon(Rc::new(Vec::new())))?;
        self.dict[xt].hidden = true;
        self.current = Some(xt);
        self.set_state(true);
        self.code.clear();
        self.ctrl.clear();
        self.end_local_scope();
        Ok(xt)
    }

    /// The cell behind a VALUE (`defer` false) or a DEFER (`defer` true).
    pub(crate) fn cell_of(&self, xt: Cell, defer: bool) -> Res<usize> {
        let word = usize::try_from(xt).ok().and_then(|i| self.dict.get(i));
        match word.map(|w| &w.kind) {
            Some(Kind::Value(addr)) if !defer => Ok(*addr),
            Some(Kind::Defer(addr)) if defer => Ok(*addr),
            _ => Err(Throw::new(INVALID_NAME)),
        }
    }

    /// Parses a name and returns the cell behind it, as `cell_of`.
    pub(crate) fn named_cell(&mut self, defer: bool) -> Res<usize> {
        let name = self.name()?;
        let xt = self.find_or_err(&name)?;
        self.cell_of(xt as Cell, defer).map_err(|e| Throw { detail: Some(name), ..e })
    }

    /// Stores into a VALUE or DEFER cell now, or compiles the store.
    pub(crate) fn store_to(&mut self, addr: usize) -> Res {
        if self.compiling() {
            self.code.push(Instr::Store(addr));
        } else {
            let v = self.pop()?;
            self.set(addr, v);
        }
        Ok(())
    }

    pub(crate) fn need_def(&self) -> Res {
        if self.current.is_some() { Ok(()) } else { Err(Throw::new(COMPILE_ONLY)) }
    }

    pub(crate) fn emit_fwd(&mut self, instr: Instr) -> usize {
        self.code.push(instr);
        self.code.len() - 1
    }

    pub(crate) fn patch(&mut self, at: usize, target: usize) {
        match &mut self.code[at] {
            Instr::Branch(t) | Instr::ZBranch(t) | Instr::Do(t) | Instr::QDo(t) | Instr::Of(t) => {
                *t = target
            }
            _ => unreachable!("patching a non-jump instruction"),
        }
    }

    /// The index in the control-flow stack of item u (0 is the top), for
    /// CS-PICK and CS-ROLL.
    pub(crate) fn cs_index(&mut self) -> Res<usize> {
        let u = self.pop()?;
        match usize::try_from(u).ok().filter(|&u| u < self.ctrl.len()) {
            Some(u) => Ok(self.ctrl.len() - 1 - u),
            None => Err(Throw::new(CONTROL_MISMATCH)),
        }
    }

    /// Skips words for [IF] and [ELSE], across lines, up to the matching
    /// [THEN], or the matching [ELSE] when `to_else` is set. A nested
    /// [IF] … [THEN] is skipped whole.
    pub(crate) fn skip_conditional(&mut self, to_else: bool) -> Res {
        let mut depth = 0;
        loop {
            let Some(word) = self.word() else {
                if self.refill()? {
                    continue;
                }
                return Ok(()); // the input ended first
            };
            match word.to_ascii_uppercase().as_str() {
                "[IF]" => depth += 1,
                "[ELSE]" if depth == 0 && to_else => return Ok(()),
                "[THEN]" if depth == 0 => return Ok(()),
                "[THEN]" => depth -= 1,
                _ => {}
            }
        }
    }

    pub(crate) fn pop_orig(&mut self) -> Res<usize> {
        match self.ctrl.pop() {
            Some(Ctrl::Orig(p)) => Ok(p),
            _ => Err(Throw::new(CONTROL_MISMATCH)),
        }
    }

    pub(crate) fn pop_dest(&mut self) -> Res<usize> {
        match self.ctrl.pop() {
            Some(Ctrl::Dest(p)) => Ok(p),
            _ => Err(Throw::new(CONTROL_MISMATCH)),
        }
    }

    pub(crate) fn pop_do(&mut self) -> Res<usize> {
        match self.ctrl.pop() {
            Some(Ctrl::Do(p)) => Ok(p),
            _ => Err(Throw::new(CONTROL_MISMATCH)),
        }
    }

    pub(crate) fn close_loop(&mut self, make: fn(usize) -> Instr) -> Res {
        let p = self.pop_do()?;
        self.code.push(make(p + 1));
        let end = self.code.len();
        self.patch(p, end);
        Ok(())
    }
}
