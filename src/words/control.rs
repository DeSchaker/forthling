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
