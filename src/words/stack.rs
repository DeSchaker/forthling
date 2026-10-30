//! Stack, arithmetic and logic words.

use crate::*;

pub(crate) fn install(f: &mut Forth) {
    f.prim("DUP", |f| {
        let a = f.pop()?;
        f.push(a);
        f.push(a);
        Ok(())
    });
    f.prim("DROP", |f| f.pop().map(|_| ()));
    f.prim("SWAP", |f| {
        let (a, b) = f.pop2()?;
        f.push(b);
        f.push(a);
        Ok(())
    });
    f.prim("OVER", |f| {
        let (a, b) = f.pop2()?;
        f.push(a);
        f.push(b);
        f.push(a);
        Ok(())
    });
    f.prim("ROT", |f| {
        let c = f.pop()?;
        let (a, b) = f.pop2()?;
        f.push(b);
        f.push(c);
        f.push(a);
        Ok(())
    });
    f.prim("PICK", |f| {
        let n = f.pop()?;
        let len = f.stack.len() as Cell;
        if n < 0 || n >= len {
            return Err(Throw::new(-4));
        }
        f.push(f.stack[(len - 1 - n) as usize]);
        Ok(())
    });
    f.prim("ROLL", |f| {
        let n = f.pop()?;
        let len = f.stack.len() as Cell;
        if n < 0 || n >= len {
            return Err(Throw::new(-4));
        }
        let x = f.stack.remove((len - 1 - n) as usize);
        f.push(x);
        Ok(())
    });
    f.prim("DEPTH", |f| {
        f.push(f.stack.len() as Cell);
        Ok(())
    });
    f.prim(">R", |f| {
        let a = f.pop()?;
        if f.rstack.len() >= RSTACK_CELLS {
            return Err(Throw::new(-5));
        }
        f.rstack.push(a);
        Ok(())
    });
    f.prim("R>", |f| {
        let a = f.rstack.pop().ok_or(Throw::new(-6))?;
        f.push(a);
        Ok(())
    });
    f.prim("R@", |f| {
        let a = *f.rstack.last().ok_or(Throw::new(-6))?;
        f.push(a);
        Ok(())
    });

    // arithmetic and logic
    f.prim("+", |f| f.binop(|a, b| a.wrapping_add(b)));
    f.prim("-", |f| f.binop(|a, b| a.wrapping_sub(b)));
    f.prim("*", |f| f.binop(|a, b| a.wrapping_mul(b)));
    // / MOD /MOD */ */MOD are defined in the prelude via FM/MOD.
}
