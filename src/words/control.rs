//! Control-flow words.

use crate::*;

pub(crate) fn install(f: &mut Forth) {
    f.imm("IF", |f| {
        f.need_def()?;
        let p = f.emit_fwd(Instr::ZBranch(0));
        f.ctrl.push(Ctrl::Orig(p));
        Ok(())
    });
    f.imm("ELSE", |f| {
        f.need_def()?;
        let p = f.pop_orig()?;
        let q = f.emit_fwd(Instr::Branch(0));
        f.patch(p, q + 1);
        f.ctrl.push(Ctrl::Orig(q));
        Ok(())
    });
    f.imm("THEN", |f| {
        f.need_def()?;
        let p = f.pop_orig()?;
        let end = f.code.len();
        f.patch(p, end);
        Ok(())
    });
    f.imm("BEGIN", |f| {
        f.need_def()?;
        f.ctrl.push(Ctrl::Dest(f.code.len()));
        Ok(())
    });
    f.imm("UNTIL", |f| {
        f.need_def()?;
        let d = f.pop_dest()?;
        f.code.push(Instr::ZBranch(d));
        Ok(())
    });
    f.imm("AGAIN", |f| {
        f.need_def()?;
        let d = f.pop_dest()?;
        f.code.push(Instr::Branch(d));
        Ok(())
    });
    // WHILE ( dest -- orig dest ) leaves its orig under the dest, so
    // several WHILEs can share one BEGIN; REPEAT is AGAIN THEN.
    f.imm("WHILE", |f| {
        f.need_def()?;
        let d = f.pop_dest()?;
        let p = f.emit_fwd(Instr::ZBranch(0));
        f.ctrl.push(Ctrl::Orig(p));
        f.ctrl.push(Ctrl::Dest(d));
        Ok(())
    });
    f.imm("REPEAT", |f| {
        f.need_def()?;
        let d = f.pop_dest()?;
        f.code.push(Instr::Branch(d));
        let p = f.pop_orig()?;
        let end = f.code.len();
        f.patch(p, end);
        Ok(())
    });
    f.imm("AHEAD", |f| {
        f.need_def()?;
        let p = f.emit_fwd(Instr::Branch(0));
        f.ctrl.push(Ctrl::Orig(p));
        Ok(())
    });
    f.prim("CS-PICK", |f| {
        f.need_def()?;
        let i = f.cs_index()?;
        f.ctrl.push(f.ctrl[i]);
        Ok(())
    });
    f.prim("CS-ROLL", |f| {
        f.need_def()?;
        let i = f.cs_index()?;
        let item = f.ctrl.remove(i);
        f.ctrl.push(item);
        Ok(())
    });
    f.imm("CASE", |f| {
        f.need_def()?;
        f.ctrl.push(Ctrl::Case);
        Ok(())
    });
    f.imm("OF", |f| {
        f.need_def()?;
        let p = f.emit_fwd(Instr::Of(0));
        f.ctrl.push(Ctrl::Of(p));
        Ok(())
    });
    f.imm("ENDOF", |f| {
        f.need_def()?;
        let Some(Ctrl::Of(p)) = f.ctrl.pop() else {
            return Err(Throw::new(-22));
        };
        let q = f.emit_fwd(Instr::Branch(0));
        f.patch(p, q + 1);
        f.ctrl.push(Ctrl::EndOf(q));
        Ok(())
    });
    // ENDCASE drops the selector when no OF matched; every ENDOF jumps past that.
    f.imm("ENDCASE", |f| {
        f.need_def()?;
        f.code.push(Instr::Drop);
        let end = f.code.len();
        loop {
            match f.ctrl.pop() {
                Some(Ctrl::EndOf(p)) => f.patch(p, end),
                Some(Ctrl::Case) => return Ok(()),
                _ => return Err(Throw::new(-22)),
            }
        }
    });
    f.imm("DO", |f| {
        f.need_def()?;
        let p = f.emit_fwd(Instr::Do(0));
        f.ctrl.push(Ctrl::Do(p));
        Ok(())
    });
    f.imm("?DO", |f| {
        f.need_def()?;
        let p = f.emit_fwd(Instr::QDo(0));
        f.ctrl.push(Ctrl::Do(p));
        Ok(())
    });
    f.imm("LOOP", |f| f.close_loop(Instr::Loop));
    f.imm("+LOOP", |f| f.close_loop(Instr::PlusLoop));
    f.imm("LEAVE", |f| {
        f.need_def()?;
        f.code.push(Instr::Leave);
        Ok(())
    });
    f.prim("UNLOOP", |f| {
        f.loops.pop().ok_or(Throw::new(-26))?;
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
