//! The LOCALS word set: (LOCAL), plus {: and LOCALS| from LOCALS EXT.
//!
//! Locals live on their own stack, `Forth::locals`; each frame knows where
//! its locals start. While compiling, `local_names` holds the names of the
//! current definition's locals, which are found before any word.

use crate::*;

pub(crate) fn install(f: &mut Forth) {
    f.compile_prim("(LOCAL)", |f| {
        let (a, n) = f.pop2()?;
        if n == 0 {
            return f.end_stack_order_locals();
        }
        let name = f.bytes(a, n)?;
        f.declare_local(&String::from_utf8_lossy(&name))
    });
    // {: args | vals -- outputs :}
    f.compile_imm("{:", |f| {
        let (mut vals, mut part) = (0, 0);
        loop {
            let Some(word) = f.word() else {
                if f.refill()? {
                    continue;
                }
                return Err(Throw::with(CONTROL_MISMATCH, "{: without :}"));
            };
            match (word.as_str(), part) {
                (":}", _) => break,
                ("|", 0) => part = 1,
                ("--", 0 | 1) => part = 2,
                (_, 2) => {} // the outputs are a comment
                ("|" | "--", _) => return Err(Throw::with(CONTROL_MISMATCH, word)),
                (name, _) => {
                    f.declare_local(name)?;
                    if part == 1 {
                        vals += 1;
                    }
                }
            }
        }
        f.end_locals(vals)
    });
    // LOCALS| a b c |, which works like (LOCAL)
    f.compile_imm("LOCALS|", |f| loop {
        match f.word() {
            Some(w) if w == "|" => return f.end_stack_order_locals(),
            Some(w) => f.declare_local(&w)?,
            None => return Err(Throw::with(CONTROL_MISMATCH, "LOCALS| without |")),
        }
    });
}

impl Forth {
    /// Adds a local to the current definition; it gets its value from the
    /// stack when the declarations end.
    pub(crate) fn declare_local(&mut self, name: &str) -> Res {
        if self.local_names.len() == MAX_LOCALS {
            return Err(Throw::with(UNSUPPORTED, "too many locals"));
        }
        self.local_names.push(name.to_ascii_uppercase());
        Ok(())
    }

    /// Ends a group of declarations: compiles code that takes the new
    /// locals from the stack, except the last `zeroed` ones, which start at 0.
    pub(crate) fn end_locals(&mut self, zeroed: usize) -> Res {
        let take = self.local_names.len() - self.locals_bound - zeroed;
        self.code.push(Instr::Locals(take, zeroed));
        self.locals_bound = self.local_names.len();
        Ok(())
    }

    /// Ends declarations made by (LOCAL), where the first local gets the top
    /// of the stack, unlike with {:.
    pub(crate) fn end_stack_order_locals(&mut self) -> Res {
        self.local_names[self.locals_bound..].reverse();
        self.end_locals(0)
    }

    /// The index of a local of the definition being compiled; the latest
    /// one wins when names repeat.
    pub(crate) fn find_local(&self, name: &str) -> Option<usize> {
        if self.current.is_none() || !self.compiling() {
            return None;
        }
        let upper = name.to_ascii_uppercase();
        self.local_names.iter().rposition(|n| *n == upper)
    }

    /// Forgets the locals of the definition being compiled.
    pub(crate) fn end_local_scope(&mut self) {
        self.local_names.clear();
        self.locals_bound = 0;
    }
}
