//! Vectors, dictionaries and sorting: the word list COLLECTIONS, installed as the
//! extension `Collections`. The vectors and dictionaries are written in Forth
//! (collections.fs); they live in `ALLOCATE`d blocks, so they count towards the memory
//! limit. Only `SORT-CELLS`, which calls an execution token, is in Rust.

use crate::*;

/// The extension: `forth.install(&Collections)`; ENVIRONMENT? knows it as
/// FORTHLING-COLLECTIONS.
pub struct Collections;

impl Extension for Collections {
    fn name(&self) -> &'static str {
        "FORTHLING-COLLECTIONS"
    }

    fn install(&self, f: &mut Forth) {
        let wid = f.wordlist("COLLECTIONS");
        f.add_word(wid, "SORT-CELLS", sort_cells).expect("COLLECTIONS is a word list");
        // Compile collections.fs into the word list, and leave the search order
        // and the compilation word list as they were.
        let (order, current) = (f.order.clone(), f.current_wid);
        f.interpret("ALSO COLLECTIONS DEFINITIONS").expect("COLLECTIONS is a word list");
        let result = f.interpret(include_str!("collections.fs"));
        f.order = order;
        f.current_wid = current;
        result.expect("collections.fs must compile");
    }
}

/// `SORT-CELLS ( addr n xt -- )`: sorts n cells in place. xt is `( a b -- flag )`, true
/// when a goes before b. The sort is stable (a merge sort). When xt throws, the cells
/// stay as they were.
fn sort_cells(f: &mut Forth) -> Res {
    let (call, addr, count) = f.pop_block("SORT-CELLS")?;
    let base = f.stack.len();
    let mut from: Vec<Cell> = (0..count).map(|i| f.get(addr + i * CELL)).collect();
    let mut to = from.clone();
    let mut width = 1;
    while width < count {
        for start in (0..count).step_by(2 * width) {
            let mid = (start + width).min(count);
            let end = (start + 2 * width).min(count);
            let (mut left, mut right, mut out) = (start, mid, start);
            while left < mid && right < end {
                let (first, second) = (from[left], from[right]);
                // The right item goes first only when it is strictly before the left one.
                if call.apply_with(f, "SORT-CELLS", "a b -- flag", base, &[second, first])? == 0 {
                    to[out] = first;
                    left += 1;
                } else {
                    to[out] = second;
                    right += 1;
                }
                out += 1;
            }
            to[out..out + (mid - left)].copy_from_slice(&from[left..mid]);
            out += mid - left;
            to[out..out + (end - right)].copy_from_slice(&from[right..end]);
        }
        std::mem::swap(&mut from, &mut to);
        width *= 2;
    }
    for (i, x) in from.into_iter().enumerate() {
        f.set(addr + i * CELL, x);
    }
    Ok(())
}
