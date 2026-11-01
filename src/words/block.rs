//! The BLOCK word set and its extensions; the buffers are in blocks.rs.
//! THRU is in prelude.fs; BLK and SCR are system variables.

use crate::*;

pub(crate) fn install(f: &mut Forth) {
    f.prim("BLOCK", |f| {
        let u = f.pop()?;
        let a = f.block(u, true)?;
        f.push(a as Cell);
        Ok(())
    });
    f.prim("BUFFER", |f| {
        let u = f.pop()?;
        let a = f.block(u, false)?;
        f.push(a as Cell);
        Ok(())
    });
    f.prim("UPDATE", |f| {
        f.update();
        Ok(())
    });
    f.prim("SAVE-BUFFERS", Forth::save_buffers);
    f.prim("FLUSH", |f| {
        f.save_buffers()?;
        f.empty_buffers();
        Ok(())
    });
    f.prim("EMPTY-BUFFERS", |f| {
        f.empty_buffers();
        Ok(())
    });
    f.prim("LOAD", |f| {
        let u = f.pop()?;
        f.load(u)
    });
    f.prim("LIST", |f| {
        let u = f.pop()?;
        f.list(u)
    });
}
