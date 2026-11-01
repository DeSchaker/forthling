//! Control-flow words.

use crate::*;

pub(crate) fn install(f: &mut Forth) {
    install_branches(f);
    install_case(f);
    install_loops(f);
}

fn install_branches(f: &mut Forth) {
    f.compile_imm("IF", |f| {
        let p = f.emit_fwd(Instr::ZBranch(0));
        f.ctrl.push(Ctrl::Orig(p));
        Ok(())
    });
    f.compile_imm("ELSE", |f| {
        let p = f.pop_orig()?;
        let q = f.emit_fwd(Instr::Branch(0));
        f.patch(p, q + 1);
        f.ctrl.push(Ctrl::Orig(q));
        Ok(())
    });
    f.compile_imm("THEN", |f| {
        let p = f.pop_orig()?;
        let end = f.code.len();
        f.patch(p, end);
        Ok(())
    });
    f.compile_imm("BEGIN", |f| {
        f.ctrl.push(Ctrl::Dest(f.code.len()));
        Ok(())
    });
    f.compile_imm("UNTIL", |f| {
        let d = f.pop_dest()?;
        f.code.push(Instr::ZBranch(d));
        Ok(())
    });
    f.compile_imm("AGAIN", |f| {
        let d = f.pop_dest()?;
        f.code.push(Instr::Branch(d));
        Ok(())
    });
    // WHILE ( dest -- orig dest ) leaves its orig under the dest, so
    // several WHILEs can share one BEGIN; REPEAT is AGAIN THEN.
    f.compile_imm("WHILE", |f| {
        let d = f.pop_dest()?;
        let p = f.emit_fwd(Instr::ZBranch(0));
        f.ctrl.push(Ctrl::Orig(p));
        f.ctrl.push(Ctrl::Dest(d));
        Ok(())
    });
    f.compile_imm("REPEAT", |f| {
        let d = f.pop_dest()?;
        f.code.push(Instr::Branch(d));
        let p = f.pop_orig()?;
        let end = f.code.len();
        f.patch(p, end);
        Ok(())
    });
    f.compile_imm("AHEAD", |f| {
        let p = f.emit_fwd(Instr::Branch(0));
        f.ctrl.push(Ctrl::Orig(p));
        Ok(())
    });
    f.compile_prim("CS-PICK", |f| {
        let i = f.cs_index()?;
        f.ctrl.push(f.ctrl[i]);
        Ok(())
    });
    f.compile_prim("CS-ROLL", |f| {
        let i = f.cs_index()?;
        let item = f.ctrl.remove(i);
        f.ctrl.push(item);
        Ok(())
    });
}

fn install_case(f: &mut Forth) {
    f.compile_imm("CASE", |f| {
        f.ctrl.push(Ctrl::Case);
        Ok(())
    });
    f.compile_imm("OF", |f| {
        let p = f.emit_fwd(Instr::Of(0));
        f.ctrl.push(Ctrl::Of(p));
        Ok(())
    });
    f.compile_imm("ENDOF", |f| {
        let Some(Ctrl::Of(p)) = f.ctrl.pop() else {
            return Err(Throw::new(CONTROL_MISMATCH));
        };
        let q = f.emit_fwd(Instr::Branch(0));
        f.patch(p, q + 1);
        f.ctrl.push(Ctrl::EndOf(q));
        Ok(())
    });
    // ENDCASE drops the selector when no OF matched; every ENDOF jumps past that.
    f.compile_imm("ENDCASE", |f| {
        f.code.push(Instr::Drop);
        let end = f.code.len();
        loop {
            match f.ctrl.pop() {
                Some(Ctrl::EndOf(p)) => f.patch(p, end),
                Some(Ctrl::Case) => return Ok(()),
                _ => return Err(Throw::new(CONTROL_MISMATCH)),
            }
        }
    });
}

fn install_loops(f: &mut Forth) {
    f.compile_imm("DO", |f| {
        let p = f.emit_fwd(Instr::Do(0));
        f.ctrl.push(Ctrl::Do(p));
        Ok(())
    });
    f.compile_imm("?DO", |f| {
        let p = f.emit_fwd(Instr::QDo(0));
        f.ctrl.push(Ctrl::Do(p));
        Ok(())
    });
    f.compile_imm("LOOP", |f| f.close_loop(Instr::Loop));
    f.compile_imm("+LOOP", |f| f.close_loop(Instr::PlusLoop));
    f.compile_imm("LEAVE", |f| {
        f.code.push(Instr::Leave);
        Ok(())
    });
    install_gforth_loops(f);
    f.prim("UNLOOP", |f| {
        f.loops.pop().ok_or(Throw::new(NO_LOOP))?;
        Ok(())
    });
    f.prim("I", |f| {
        let i = f.loop_index(0)?;
        f.push(i);
        Ok(())
    });
    f.prim("J", |f| {
        let j = f.loop_index(1)?;
        f.push(j);
        Ok(())
    });
}

/// The counted loops of Gforth, built on `?DO` and `+LOOP`. `+DO`, `U+DO`,
/// `-DO`, `U-DO` and `FOR` first make the loop parameters such that `?DO`
/// skips the loop when it must not run; `-LOOP` and `NEXT` count down and
/// leave the loop when the index reaches the limit.
#[allow(clippy::cast_sign_loss, reason = "U+DO and U-DO compare the loop parameters as unsigned")]
fn install_gforth_loops(f: &mut Forth) {
    // ( limit start -- limit start' ): start becomes the limit, so that ?DO skips, unless
    // the loop must run.
    f.prim("(+DO)", |f| skip_unless(f, |start, limit| start < limit));
    f.prim("(U+DO)", |f| skip_unless(f, |start, limit| (start as u64) < (limit as u64)));
    f.prim("(-DO)", |f| skip_unless(f, |start, limit| start > limit));
    f.prim("(U-DO)", |f| skip_unless(f, |start, limit| (start as u64) > (limit as u64)));
    // FOR runs n+1 times, with the index counting from n down to 0.
    f.prim("(FOR)", |f| {
        let n = f.pop()?;
        f.push(-1);
        f.push(n.max(-1));
        Ok(())
    });
    // ( u -- done ): the index goes down by u; the loop is done when it reaches the
    // limit, that is, when the border between limit+1 and limit is crossed.
    f.prim("(-LOOP)", |f| {
        let u = f.pop()?;
        let lf = f.loops.last_mut().ok_or(Throw::new(NO_LOOP))?;
        let step = u.wrapping_neg();
        let before = lf.index.wrapping_sub(lf.limit.wrapping_add(1));
        let after = before.wrapping_add(step);
        lf.index = lf.index.wrapping_add(step);
        let done = (before ^ after) & (before ^ step) < 0;
        if done {
            f.loops.pop();
        }
        f.push(flag(done));
        Ok(())
    });
    f.compile_imm("+DO", |f| {
        open_loop(f, "(+DO)");
        Ok(())
    });
    f.compile_imm("U+DO", |f| {
        open_loop(f, "(U+DO)");
        Ok(())
    });
    f.compile_imm("-DO", |f| {
        open_loop(f, "(-DO)");
        Ok(())
    });
    f.compile_imm("U-DO", |f| {
        open_loop(f, "(U-DO)");
        Ok(())
    });
    f.compile_imm("FOR", |f| {
        open_loop(f, "(FOR)");
        Ok(())
    });
    f.compile_imm("-LOOP", |f| close_down(f, None));
    f.compile_imm("NEXT", |f| close_down(f, Some(1)));
    f.compile_imm("?LEAVE", |f| {
        let p = f.emit_fwd(Instr::ZBranch(0));
        f.code.push(Instr::Leave);
        let end = f.code.len();
        f.patch(p, end);
        Ok(())
    });
    f.compile_imm("?EXIT", |f| {
        let p = f.emit_fwd(Instr::ZBranch(0));
        f.code.push(Instr::Exit);
        let end = f.code.len();
        f.patch(p, end);
        Ok(())
    });
}

fn skip_unless(f: &mut Forth, runs: fn(Cell, Cell) -> bool) -> Res {
    let (limit, start) = f.pop2()?;
    f.push(limit);
    f.push(if runs(start, limit) { start } else { limit });
    Ok(())
}

/// Compiles a call to the primitive that prepares the parameters, then `?DO`.
fn open_loop(f: &mut Forth, prep: &str) {
    let xt = f.find_in(prep, FORTH_WID).expect("defined in install_gforth_loops");
    f.code.push(Instr::Call(xt));
    let p = f.emit_fwd(Instr::QDo(0));
    f.ctrl.push(Ctrl::Do(p));
}

/// Closes a loop that counts down: the step is on the stack, or `step`.
fn close_down(f: &mut Forth, step: Option<Cell>) -> Res {
    let p = f.pop_do()?;
    let xt = f.find_in("(-LOOP)", FORTH_WID).expect("defined in install_gforth_loops");
    if let Some(n) = step {
        f.code.push(Instr::Lit(n));
    }
    f.code.push(Instr::Call(xt));
    f.code.push(Instr::ZBranch(p + 1));
    let end = f.code.len();
    f.patch(p, end);
    Ok(())
}
