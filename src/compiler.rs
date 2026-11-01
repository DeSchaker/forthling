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

/// The definition that was being compiled when a quotation `[:` began,
/// put back by `;]`.
pub(crate) struct Quote {
    pub(crate) inner: usize, // the word of the quotation
    current: Option<usize>,
    code: Vec<Instr>,
    ctrl: Vec<Ctrl>,
    local_names: Vec<String>,
    locals_bound: usize,
}

impl Forth {
    /// Starts a quotation inside the definition being compiled. It is a
    /// word of its own, without a name; the locals of the outer definition
    /// are not visible in it.
    pub(crate) fn start_quote(&mut self) -> Res {
        if self.current.is_none() {
            // Outside a definition it is a :NONAME that ;] ends with the xt on the stack.
            let inner = self.start_def("")?;
            self.quotes.push(Quote {
                inner,
                current: None,
                code: Vec::new(),
                ctrl: Vec::new(),
                local_names: Vec::new(),
                locals_bound: 0,
            });
            return Ok(());
        }
        let inner = self.new_word("", Kind::Colon(Rc::new(Vec::new())))?;
        self.dict[inner].hidden = true;
        self.quotes.push(Quote {
            inner,
            current: self.current,
            code: std::mem::take(&mut self.code),
            ctrl: std::mem::take(&mut self.ctrl),
            local_names: std::mem::take(&mut self.local_names),
            locals_bound: std::mem::take(&mut self.locals_bound),
        });
        self.current = Some(inner);
        Ok(())
    }

    /// Ends the innermost quotation: back in the outer definition, which
    /// gets the quotation's xt as a literal. A quotation that holds one
    /// primitive and nothing else leaves the xt of that primitive.
    pub(crate) fn end_quote(&mut self) -> Res {
        if !self.ctrl.is_empty() {
            return Err(Throw::new(CONTROL_MISMATCH));
        }
        let Some(q) = self.quotes.pop() else {
            return Err(Throw::new(CONTROL_MISMATCH));
        };
        let code = std::mem::take(&mut self.code);
        let single = match code[..] {
            [Instr::Call(p)] => Some(p),
            _ => None,
        };
        let primitive = single.filter(|&p| {
            let w = &self.dict[p];
            matches!(w.kind, Kind::Prim(_)) && !w.immediate && !w.compile_only
        });
        self.current = q.current;
        self.code = q.code;
        self.ctrl = q.ctrl;
        self.local_names = q.local_names;
        self.locals_bound = q.locals_bound;
        let xt = if let Some(p) = primitive {
            self.forget_from(q.inner);
            p
        } else {
            self.count_code(q.inner, code.len())?;
            self.dict[q.inner].kind = Kind::Colon(Rc::new(code));
            q.inner
        };
        if self.current.is_some() {
            self.code.push(Instr::Lit(to_cell(xt)));
        } else {
            self.set_state(false);
            self.push(to_cell(xt));
        }
        Ok(())
    }

    /// Drops the quotations begun after the first `keep` ones, with the
    /// words they made, and puts the definition around them back.
    pub(crate) fn unwind_quotes(&mut self, keep: usize) {
        if self.quotes.len() <= keep {
            return;
        }
        let q = self.quotes.swap_remove(keep);
        self.quotes.truncate(keep);
        self.forget_from(q.inner);
        self.current = q.current;
        self.code = q.code;
        self.ctrl = q.ctrl;
        self.local_names = q.local_names;
        self.locals_bound = q.locals_bound;
    }

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
        self.cell_of(to_cell(xt), defer).map_err(|e| Throw { detail: Some(name), ..e })
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
                *t = target;
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
