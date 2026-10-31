//! The MEMORY word set; the heap itself is in heap.rs.

use crate::*;

pub(crate) fn install(f: &mut Forth) {
    f.prim("ALLOCATE", |f| {
        let n = f.pop()?;
        f.charge(f.heap.blocks() * 16)?; // first fit looks through the blocks
        match usize::try_from(n).ok().and_then(|n| f.heap.allocate(n, f.here)) {
            Some(a) => {
                f.push(a as Cell);
                f.push(0);
            }
            None => {
                f.push(0);
                f.push(ALLOCATE_FAILED);
            }
        }
        Ok(())
    });
    f.prim("FREE", |f| {
        let a = f.pop()?;
        let freed = usize::try_from(a).is_ok_and(|a| f.heap.free(a));
        f.push(if freed { 0 } else { FREE_FAILED });
        Ok(())
    });
    f.prim("RESIZE", |f| {
        let (a, n) = f.pop2()?;
        f.charge(f.heap.blocks() * 16 + n.max(0) as usize)?;
        let new = match (usize::try_from(a), usize::try_from(n)) {
            (Ok(a), Ok(n)) => f.resize(a, n),
            _ => None,
        };
        match new {
            Some(new) => {
                f.push(new as Cell);
                f.push(0);
            }
            None => {
                f.push(a);
                f.push(RESIZE_FAILED);
            }
        }
        Ok(())
    });
}
