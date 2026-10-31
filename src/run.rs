//! The inner interpreter: threaded code, frames, loops and CATCH.

use crate::*;

pub(crate) const MAX_FRAMES: usize = 10_000;
pub(crate) const MAX_CATCHES: usize = 1000; // nested CATCHes; each one nests the Rust stack
pub(crate) const MAX_LOCALS: usize = 256; // per definition; ENVIRONMENT? #LOCALS

#[derive(Clone, Debug)]
pub(crate) enum Instr {
    Call(usize),
    Lit(Cell),
    FLit(f64),
    Branch(usize),
    ZBranch(usize),
    Of(usize),       // ( x1 x2 -- | x1 ): equal drops both, else jumps to the target
    Drop,            // the selector, dropped by ENDCASE
    Do(usize),       // target: just past the matching LOOP
    QDo(usize),      // like Do, but skips the loop when limit == index
    Loop(usize),     // target: start of the loop body
    PlusLoop(usize), // target: start of the loop body
    Leave,
    Print(Rc<[u8]>),
    Abort(Rc<[u8]>), // ABORT": throws -2 with this message if the flag is nonzero
    Compile(usize),  // POSTPONEd non-immediate word: compiles a call to it
    SetDoes(usize),  // gives the latest CREATEd word the code at this offset
    Store(usize),    // compiled TO and IS: stores the top of the stack here
    Fetch(usize),    // compiled ACTION-OF: pushes the cell stored here
    FStore(usize),   // compiled TO of an FVALUE: stores the top float here
    Locals(usize, usize), // new locals: this many from the stack, then this many zeroed
    Local(usize),    // pushes the frame's local with this index
    ToLocal(usize),  // compiled TO of a local
    Exit,
}

pub(crate) struct Frame {
    pub(crate) xt: usize, // the word whose code this is, for error reports
    pub(crate) code: Rc<Vec<Instr>>,
    pub(crate) pc: usize,
    pub(crate) loops: usize,  // loop stack depth on entry, restored on exit
    pub(crate) locals: usize, // where the frame's locals start in Forth::locals
}

pub(crate) struct LoopFrame {
    pub(crate) index: Cell,
    pub(crate) limit: Cell,
    pub(crate) exit: usize,
}

impl Forth {
    /// CATCH: runs `xt` and returns 0, or, when it throws, puts the stacks,
    /// the input sources and the compiler back as they were and returns the
    /// THROW code. QUIT is not an exception and passes through.
    pub(crate) fn catch(&mut self, xt: Cell) -> Res<Cell> {
        let depth = self.stack.len();
        let (rdepth, loops, frames) = (self.rstack.len(), self.loops.len(), self.frames.len());
        let (locals, fdepth) = (self.locals.len(), self.fstack.len());
        let (sources, to_in) = (self.sources.len(), self.get(IN_ADDR));
        let (current, state, ctrl) = (self.current, self.get(STATE_ADDR), self.ctrl.len());
        if self.catches == MAX_CATCHES {
            return Err(Throw::with(RSTACK_OVERFLOW, "CATCH nested too deeply"));
        }
        self.catches += 1;
        let base = self.frames.len();
        let result = self.invoke(xt as usize).and_then(|_| self.run(base));
        self.catches -= 1;
        let e = match result {
            Ok(()) => return Ok(0),
            Err(e) if e.code == QUIT => return Err(e),
            Err(e) => e,
        };
        // The depth is restored even if the stack was emptied; such cells hold 0.
        self.stack.resize(depth, 0);
        self.fstack.resize(fdepth, 0.0);
        self.rstack.truncate(rdepth);
        self.loops.truncate(loops);
        self.frames.truncate(frames);
        self.locals.truncate(locals);
        self.sources.truncate(sources);
        self.set(IN_ADDR, to_in);
        self.sync_blk();
        if current.is_none() {
            // Drop a definition that was started inside the CATCH.
            if let Some(xt) = self.current.take() {
                self.forget_from(xt);
                self.code.clear();
                self.end_local_scope();
            }
        }
        self.ctrl.truncate(ctrl);
        self.set(STATE_ADDR, state);
        Ok(e.code)
    }

    pub(crate) fn execute(&mut self, xt: usize) -> Res {
        self.step()?;
        let base = self.frames.len();
        self.invoke(xt)?;
        self.run(base)
    }

    /// Runs a primitive directly, or pushes a frame for compiled code.
    pub(crate) fn invoke(&mut self, mut xt: usize) -> Res {
        // A deferred word runs its action; follow chains of them in a loop,
        // so a DEFER that refers to itself cannot overflow the Rust stack.
        let mut hops = 0;
        while let Some(Word { kind: Kind::Defer(addr), name, .. }) = self.dict.get(xt) {
            match self.get(*addr) {
                -1 => return Err(Throw::with(UNSUPPORTED, format!("{name} has no action yet"))),
                action => xt = action as usize,
            }
            hops += 1;
            if hops > MAX_FRAMES {
                return Err(Throw::new(RSTACK_OVERFLOW));
            }
        }
        let word = self.dict.get(xt).ok_or_else(|| {
            Throw::with(INVALID_ARGUMENT, format!("invalid execution token {xt}"))
        })?;
        if word.compile_only && self.current.is_none() {
            return Err(Throw::with(COMPILE_ONLY, word.name.clone()));
        }
        match word.kind.clone() {
            Kind::Prim(f) => f(self),
            Kind::Host(f) => f(self),
            Kind::Colon(code) => self.enter(xt, code, 0),
            Kind::Constant(v) => {
                self.push(v);
                Ok(())
            }
            Kind::Created { addr, does } => {
                self.push(addr);
                match does {
                    Some((code, pc)) => self.enter(xt, code, pc),
                    None => Ok(()),
                }
            }
            Kind::Value(addr) => {
                self.push(self.get(addr));
                Ok(())
            }
            Kind::TwoValue(addr) => {
                self.push(self.get(addr + CELL));
                self.push(self.get(addr));
                Ok(())
            }
            Kind::FValue(addr) => self.fpush(self.fget(addr)),
            Kind::Defer(_) => unreachable!("resolved above"),
            Kind::Marker { dict_len, here, wordlists, order, current_wid } => {
                self.forget_from(dict_len);
                self.here = here;
                self.wordlists.truncate(wordlists);
                self.order = order;
                self.current_wid = current_wid;
                Ok(())
            }
        }
    }

    pub(crate) fn enter(&mut self, xt: usize, code: Rc<Vec<Instr>>, pc: usize) -> Res {
        if self.frames.len() >= MAX_FRAMES {
            return Err(Throw::new(RSTACK_OVERFLOW));
        }
        let (loops, locals) = (self.loops.len(), self.locals.len());
        self.frames.push(Frame { xt, code, pc, loops, locals });
        Ok(())
    }

    pub(crate) fn leave_frame(&mut self) {
        if let Some(f) = self.frames.pop() {
            self.loops.truncate(f.loops);
            self.locals.truncate(f.locals);
        }
    }

    pub(crate) fn jump(&mut self, pc: usize) {
        if let Some(f) = self.frames.last_mut() {
            f.pc = pc;
        }
    }

    pub(crate) fn run(&mut self, base: usize) -> Res {
        while self.frames.len() > base {
            self.step()?;
            self.check_stack()?;
            let frame = self.frames.last_mut().unwrap();
            let Some(instr) = frame.code.get(frame.pc).cloned() else {
                self.leave_frame();
                continue;
            };
            frame.pc += 1;
            match instr {
                Instr::Call(xt) => self.invoke(xt)?,
                Instr::Lit(n) => self.push(n),
                Instr::Branch(t) => self.jump(t),
                Instr::ZBranch(t) => {
                    if self.pop()? == 0 {
                        self.jump(t);
                    }
                }
                Instr::Of(t) => {
                    let (x1, x2) = self.pop2()?;
                    if x1 != x2 {
                        self.push(x1);
                        self.jump(t);
                    }
                }
                Instr::Drop => {
                    self.pop()?;
                }
                Instr::Do(exit) => {
                    let (limit, index) = self.pop2()?;
                    self.loops.push(LoopFrame { index, limit, exit });
                }
                Instr::QDo(exit) => {
                    let (limit, index) = self.pop2()?;
                    if index == limit {
                        self.jump(exit);
                    } else {
                        self.loops.push(LoopFrame { index, limit, exit });
                    }
                }
                Instr::Loop(start) => self.loop_step(start, 1)?,
                Instr::PlusLoop(start) => {
                    let step = self.pop()?;
                    self.loop_step(start, step)?;
                }
                Instr::Leave => {
                    let lf = self.loops.pop().ok_or(Throw::new(NO_LOOP))?;
                    self.jump(lf.exit);
                }
                Instr::Print(s) => self.emit_bytes(&s),
                Instr::Abort(s) => self.abort_if(&s)?,
                Instr::Compile(xt) => {
                    self.need_def()?;
                    self.code.push(Instr::Call(xt));
                }
                Instr::SetDoes(pc) => {
                    let code = self.frames.last().unwrap().code.clone();
                    match self.dict.last_mut() {
                        Some(Word { kind: Kind::Created { does, .. }, .. }) => {
                            *does = Some((code, pc))
                        }
                        _ => return Err(Throw::with(UNSUPPORTED, "DOES> without CREATE")),
                    }
                }
                Instr::Store(addr) => {
                    let v = self.pop()?;
                    self.set(addr, v);
                }
                Instr::Fetch(addr) => self.push(self.get(addr)),
                Instr::FLit(r) => self.fpush(r)?,
                Instr::FStore(addr) => {
                    let r = self.fpop()?;
                    self.fset(addr, r);
                }
                Instr::Locals(take, zeroed) => {
                    let start =
                        self.stack.len().checked_sub(take).ok_or(Throw::new(STACK_UNDERFLOW))?;
                    self.locals.extend(self.stack.drain(start..));
                    self.locals.extend(std::iter::repeat_n(0, zeroed));
                }
                Instr::Local(i) => {
                    let base = self.frames.last().unwrap().locals;
                    self.push(self.locals[base + i]);
                }
                Instr::ToLocal(i) => {
                    let v = self.pop()?;
                    let base = self.frames.last().unwrap().locals;
                    self.locals[base + i] = v;
                }
                Instr::Exit => self.leave_frame(),
            }
        }
        Ok(())
    }

    /// Advances the innermost loop; it ends when the index crosses the
    /// boundary between limit-1 and limit, in either direction.
    pub(crate) fn loop_step(&mut self, start: usize, step: Cell) -> Res {
        let lf = self.loops.last_mut().ok_or(Throw::new(NO_LOOP))?;
        let before = lf.index.wrapping_sub(lf.limit);
        let after = before.wrapping_add(step);
        lf.index = lf.index.wrapping_add(step);
        if (before ^ after) & (before ^ step) < 0 {
            self.loops.pop();
        } else {
            self.jump(start);
        }
        Ok(())
    }
}
