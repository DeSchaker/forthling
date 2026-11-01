//! Functional words: the word list FUNCTIONAL, installed as the extension
//! `Functional`. `MAP FOLD SCAN FILTER` take their items from the data stack
//! and run the loop in Rust, calling the execution token once per item. When
//! the token is a primitive (`' +`, `' 1+`, or a quotation of one primitive)
//! it is called directly, without the inner interpreter. The stack effect of
//! the token is checked after every call.

use crate::*;

/// The extension: `forth.install(&Functional)`; ENVIRONMENT? knows it as
/// FORTHLING-FUNCTIONAL.
pub struct Functional;

/// The token a word applies to each item.
enum Callee {
    /// A primitive: called directly, with one step charged per call.
    Prim(Prim),
    /// Anything else: run by the inner interpreter.
    Xt(usize),
}

impl Callee {
    fn new(f: &Forth, xt: Cell, who: &str) -> Res<Self> {
        let word = usize::try_from(xt).ok().and_then(|i| f.dict.get(i).map(|w| (i, w)));
        let Some((xt, word)) = word else {
            return Err(Throw::with(
                INVALID_ARGUMENT,
                format!("{who}: invalid execution token {xt}"),
            ));
        };
        match word.kind {
            Kind::Prim(p) if !word.compile_only && !f.watching => Ok(Self::Prim(p)),
            _ => Ok(Self::Xt(xt)),
        }
    }

    /// Pushes `args`, calls the token and takes its one result. The token
    /// must leave the stack as `base` cells plus that result.
    fn apply(&self, f: &mut Forth, who: &str, base: usize, args: &[Cell]) -> Res<Cell> {
        for &a in args {
            f.push(a);
        }
        match self {
            Self::Prim(p) => {
                f.step()?;
                p(f)?;
                f.check_stack()?;
            }
            Self::Xt(xt) => f.execute(*xt)?,
        }
        if f.stack.len() != base + 1 {
            let effect = if args.len() == 1 { "x -- y" } else { "acc x -- acc'" };
            return Err(Throw::with(
                INVALID_ARGUMENT,
                format!("{who}: the token must have the stack effect ( {effect} ) with one result"),
            ));
        }
        f.pop()
    }
}

impl Forth {
    /// Pops `n xt` and takes the `n` items under them off the stack.
    fn pop_items(&mut self, who: &str) -> Res<(Callee, Vec<Cell>)> {
        let xt = self.pop()?;
        let n = self.pop()?;
        let n = usize::try_from(n)
            .ok()
            .filter(|&n| n <= self.stack.len())
            .ok_or_else(|| Throw::with(INVALID_ARGUMENT, format!("{who}: bad item count {n}")))?;
        let callee = Callee::new(self, xt, who)?;
        let at = self.stack.len() - n;
        Ok((callee, self.stack.split_off(at)))
    }

    /// Pops `addr n xt` of a block of `n` cells and checks the address range.
    fn pop_block(&mut self, who: &str) -> Res<(Callee, usize, usize)> {
        let xt = self.pop()?;
        let n = self.pop()?;
        let a = self.pop()?;
        let bytes = n
            .checked_mul(CELL as Cell)
            .ok_or_else(|| Throw::with(INVALID_ARGUMENT, format!("{who}: bad cell count {n}")))?;
        if n < 0 {
            return Err(Throw::with(INVALID_ARGUMENT, format!("{who}: bad cell count {n}")));
        }
        let a = self.check(a, bytes)?;
        let callee = Callee::new(self, xt, who)?;
        Ok((callee, a, n as usize))
    }
}

fn word(f: &mut Forth, wid: Cell, name: &str, word: impl Fn(&mut Forth) -> Res + 'static) {
    f.add_word(wid, name, word).expect("FUNCTIONAL is a word list and names are not empty");
}

impl Extension for Functional {
    fn name(&self) -> &'static str {
        "FORTHLING-FUNCTIONAL"
    }

    fn install(&self, f: &mut Forth) {
        let wid = f.wordlist("FUNCTIONAL");
        // MAP ( x1 .. xn n xt -- y1 .. yn ): xt is ( x -- y ).
        word(f, wid, "MAP", |f| {
            let (call, items) = f.pop_items("MAP")?;
            let base = f.stack.len();
            let mut out = Vec::with_capacity(items.len());
            for x in items {
                out.push(call.apply(f, "MAP", base, &[x])?);
            }
            f.stack.extend(out);
            Ok(())
        });
        // FOLD ( x1 .. xn n xt -- r ): xt is ( acc x -- acc' ); x1 is the
        // first acc, so n must be at least 1.
        word(f, wid, "FOLD", |f| {
            let (call, items) = f.pop_items("FOLD")?;
            let base = f.stack.len();
            let mut items = items.into_iter();
            let Some(mut acc) = items.next() else {
                return Err(Throw::with(INVALID_ARGUMENT, "FOLD: no items, so no start value"));
            };
            for x in items {
                acc = call.apply(f, "FOLD", base, &[acc, x])?;
            }
            f.push(acc);
            Ok(())
        });
        // SCAN ( x1 .. xn n xt -- r1 .. rn ): like FOLD, keeping every
        // partial result; r1 is x1.
        word(f, wid, "SCAN", |f| {
            let (call, items) = f.pop_items("SCAN")?;
            let base = f.stack.len();
            let mut out = Vec::with_capacity(items.len());
            let mut items = items.into_iter();
            if let Some(mut acc) = items.next() {
                out.push(acc);
                for x in items {
                    acc = call.apply(f, "SCAN", base, &[acc, x])?;
                    out.push(acc);
                }
            }
            f.stack.extend(out);
            Ok(())
        });
        // FILTER ( x1 .. xn n xt -- y1 .. ym m ): xt is ( x -- flag ).
        word(f, wid, "FILTER", |f| {
            let (call, items) = f.pop_items("FILTER")?;
            let base = f.stack.len();
            let mut out = Vec::new();
            for x in items {
                if call.apply(f, "FILTER", base, &[x])? != 0 {
                    out.push(x);
                }
            }
            f.stack.extend(&out);
            f.push(out.len() as Cell);
            Ok(())
        });
        // MAP-CELLS ( addr n xt -- ): xt is ( x -- y ); every cell is
        // replaced by its result.
        word(f, wid, "MAP-CELLS", |f| {
            let (call, addr, count) = f.pop_block("MAP-CELLS")?;
            let base = f.stack.len();
            for i in 0..count {
                let at = addr + i * CELL;
                let x = f.get(at);
                let res = call.apply(f, "MAP-CELLS", base, &[x])?;
                f.set(at, res);
            }
            Ok(())
        });
        // FOLD-CELLS ( addr n xt -- r ): xt is ( acc x -- acc' ); the first
        // cell is the first acc, so n must be at least 1.
        word(f, wid, "FOLD-CELLS", |f| {
            let (call, addr, count) = f.pop_block("FOLD-CELLS")?;
            let base = f.stack.len();
            if count == 0 {
                return Err(Throw::with(
                    INVALID_ARGUMENT,
                    "FOLD-CELLS: no cells, so no start value",
                ));
            }
            let mut acc = f.get(addr);
            for i in 1..count {
                let x = f.get(addr + i * CELL);
                acc = call.apply(f, "FOLD-CELLS", base, &[acc, x])?;
            }
            f.push(acc);
            Ok(())
        });
        // SCAN-CELLS ( addr n xt -- ): like FOLD-CELLS; cell i becomes the
        // partial result over cells 0..=i, so the first cell stays.
        word(f, wid, "SCAN-CELLS", |f| {
            let (call, addr, count) = f.pop_block("SCAN-CELLS")?;
            let base = f.stack.len();
            if count > 0 {
                let mut acc = f.get(addr);
                for i in 1..count {
                    let at = addr + i * CELL;
                    let x = f.get(at);
                    acc = call.apply(f, "SCAN-CELLS", base, &[acc, x])?;
                    f.set(at, acc);
                }
            }
            Ok(())
        });
        // FILTER-CELLS ( addr n xt -- m ): xt is ( x -- flag ); the cells
        // that pass move to the front, m is how many.
        word(f, wid, "FILTER-CELLS", |f| {
            let (call, addr, count) = f.pop_block("FILTER-CELLS")?;
            let base = f.stack.len();
            let mut kept = 0;
            for i in 0..count {
                let x = f.get(addr + i * CELL);
                if call.apply(f, "FILTER-CELLS", base, &[x])? != 0 {
                    f.set(addr + kept * CELL, x);
                    kept += 1;
                }
            }
            f.push(kept as Cell);
            Ok(())
        });
        // RANGE ( a b -- a .. b-1 n ): the numbers from a up to b.
        word(f, wid, "RANGE", |f| {
            let (a, b) = f.pop2()?;
            let n = i128::from(b) - i128::from(a);
            if n > 0 && n >= (STACK_CELLS - f.stack.len()) as i128 {
                return Err(Throw::new(STACK_OVERFLOW));
            }
            let n = n.max(0) as Cell;
            f.stack.extend(a..a + n);
            f.push(n);
            Ok(())
        });
    }
}
