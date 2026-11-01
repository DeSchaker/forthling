//! Functional words: the word list FUNCTIONAL, installed as the extension
//! `Functional`. `MAP FOLD SCAN FILTER` (and `ANY? FIND SUM` and the rest)
//! take their items from the data stack
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
        self.run(f, args)?;
        if f.stack.len() != base + 1 {
            let effect = if args.len() == 1 { "x -- y" } else { "acc x -- acc'" };
            return Err(Throw::with(
                INVALID_ARGUMENT,
                format!("{who}: the token must have the stack effect ( {effect} ) with one result"),
            ));
        }
        f.pop()
    }

    /// Pushes `args` and calls the token once.
    fn run(&self, f: &mut Forth, args: &[Cell]) -> Res {
        for &a in args {
            f.push(a);
        }
        match self {
            Self::Prim(p) => {
                f.step()?;
                p(f)?;
                f.check_stack()
            }
            Self::Xt(xt) => f.execute(*xt),
        }
    }

    /// Calls a token of the effect `( x -- y1 .. yk k )` and takes its `k`
    /// results; the stack must be back at `base` cells afterwards.
    fn apply_many(&self, f: &mut Forth, who: &str, base: usize, x: Cell) -> Res<Vec<Cell>> {
        self.run(f, &[x])?;
        let bad = || {
            Throw::with(
                INVALID_ARGUMENT,
                format!("{who}: the token must have the stack effect ( x -- y1 .. yk k )"),
            )
        };
        if f.stack.len() <= base {
            return Err(bad());
        }
        let k = usize::try_from(f.pop()?).map_err(|_| bad())?;
        if f.stack.len() != base + k {
            return Err(bad());
        }
        Ok(f.stack.split_off(base))
    }
}

impl Forth {
    /// Pops `n xt` and takes the `n` items under them off the stack.
    fn pop_items(&mut self, who: &str) -> Res<(Callee, Vec<Cell>)> {
        let xt = self.pop()?;
        let n = self.pop()?;
        let n = self.item_count(n, who)?;
        let callee = Callee::new(self, xt, who)?;
        let at = self.stack.len() - n;
        Ok((callee, self.stack.split_off(at)))
    }

    /// Pops `n` and takes the `n` items under it off the stack.
    fn pop_list(&mut self, who: &str) -> Res<Vec<Cell>> {
        let n = self.pop()?;
        let n = self.item_count(n, who)?;
        let at = self.stack.len() - n;
        Ok(self.stack.split_off(at))
    }

    /// Pops `m n` and the `m` items under them; gives them and `min(m, n)`.
    fn pop_split(&mut self, who: &str) -> Res<(Vec<Cell>, usize)> {
        let n = self.pop()?;
        let n = usize::try_from(n)
            .map_err(|_| Throw::with(INVALID_ARGUMENT, format!("{who}: bad item count {n}")))?;
        let items = self.pop_list(who)?;
        let k = n.min(items.len());
        Ok((items, k))
    }

    /// Checks a count `n` of items that are on the stack.
    fn item_count(&self, n: Cell, who: &str) -> Res<usize> {
        usize::try_from(n)
            .ok()
            .filter(|&n| n <= self.stack.len())
            .ok_or_else(|| Throw::with(INVALID_ARGUMENT, format!("{who}: bad item count {n}")))
    }

    /// Puts results on the stack, checking that they fit.
    fn push_all(&mut self, items: &[Cell]) -> Res {
        if items.len() >= STACK_CELLS - self.stack.len() {
            return Err(Throw::new(STACK_OVERFLOW));
        }
        self.stack.extend(items);
        Ok(())
    }

    /// Pops `addr n xt` of a block of `n` cells and checks the address range.
    fn pop_block(&mut self, who: &str) -> Res<(Callee, usize, usize)> {
        let xt = self.pop()?;
        let (a, n) = self.pop_cells(who)?;
        let callee = Callee::new(self, xt, who)?;
        Ok((callee, a, n))
    }

    /// Pops `addr n` of a block of `n` cells and checks the address range.
    fn pop_cells(&mut self, who: &str) -> Res<(usize, usize)> {
        let n = self.pop()?;
        let a = self.pop()?;
        let bytes = n
            .checked_mul(to_cell(CELL))
            .ok_or_else(|| Throw::with(INVALID_ARGUMENT, format!("{who}: bad cell count {n}")))?;
        if n < 0 {
            return Err(Throw::with(INVALID_ARGUMENT, format!("{who}: bad cell count {n}")));
        }
        let a = self.check(a, bytes)?;
        Ok((a, to_index(n)))
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
            f.push(to_cell(out.len()));
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
            f.push(to_cell(kept));
            Ok(())
        });
        // ANY? ( x1 .. xn n xt -- flag ): xt is ( x -- flag ); stops at the
        // first item that passes.
        word(f, wid, "ANY?", |f| {
            let (call, items) = f.pop_items("ANY?")?;
            let base = f.stack.len();
            let mut found = false;
            for x in items {
                if call.apply(f, "ANY?", base, &[x])? != 0 {
                    found = true;
                    break;
                }
            }
            f.push(-Cell::from(found));
            Ok(())
        });
        // ALL? ( x1 .. xn n xt -- flag ): stops at the first item that fails;
        // true for n = 0.
        word(f, wid, "ALL?", |f| {
            let (call, items) = f.pop_items("ALL?")?;
            let base = f.stack.len();
            let mut all = true;
            for x in items {
                if call.apply(f, "ALL?", base, &[x])? == 0 {
                    all = false;
                    break;
                }
            }
            f.push(-Cell::from(all));
            Ok(())
        });
        // FIND ( x1 .. xn n xt -- x true | false ): the first item that passes.
        word(f, wid, "FIND", |f| {
            let (call, items) = f.pop_items("FIND")?;
            let base = f.stack.len();
            for x in items {
                if call.apply(f, "FIND", base, &[x])? != 0 {
                    f.push(x);
                    f.push(-1);
                    return Ok(());
                }
            }
            f.push(0);
            Ok(())
        });
        // COUNT-IF ( x1 .. xn n xt -- m ): how many items pass.
        word(f, wid, "COUNT-IF", |f| {
            let (call, items) = f.pop_items("COUNT-IF")?;
            let base = f.stack.len();
            let mut m = 0;
            for x in items {
                if call.apply(f, "COUNT-IF", base, &[x])? != 0 {
                    m += 1;
                }
            }
            f.push(m);
            Ok(())
        });
        // FLAT-MAP ( x1 .. xn n xt -- y1 .. ym m ): xt is ( x -- y1 .. yk k );
        // the results of all items are joined.
        word(f, wid, "FLAT-MAP", |f| {
            let (call, items) = f.pop_items("FLAT-MAP")?;
            let base = f.stack.len();
            let mut out = Vec::new();
            for x in items {
                out.extend(call.apply_many(f, "FLAT-MAP", base, x)?);
                if out.len() >= STACK_CELLS {
                    return Err(Throw::new(STACK_OVERFLOW));
                }
            }
            f.push_all(&out)?;
            f.push(to_cell(out.len()));
            Ok(())
        });
        // ITERATE ( x n xt -- x1 .. xn ): x1 is x, every next item is xt of
        // the one before; xt is ( x -- y ).
        word(f, wid, "ITERATE", |f| {
            let xt = f.pop()?;
            let n = f.pop()?;
            let mut x = f.pop()?;
            let call = Callee::new(f, xt, "ITERATE")?;
            let n = usize::try_from(n).map_err(|_| {
                Throw::with(INVALID_ARGUMENT, format!("ITERATE: bad item count {n}"))
            })?;
            if n >= STACK_CELLS - f.stack.len() {
                return Err(Throw::new(STACK_OVERFLOW));
            }
            let base = f.stack.len();
            let mut out = Vec::with_capacity(n);
            for i in 0..n {
                out.push(x);
                if i + 1 < n {
                    x = call.apply(f, "ITERATE", base, &[x])?;
                }
            }
            f.stack.extend(out);
            Ok(())
        });
        // REVERSE ( x1 .. xn n -- xn .. x1 )
        word(f, wid, "REVERSE", |f| {
            let mut items = f.pop_list("REVERSE")?;
            items.reverse();
            f.stack.extend(items);
            Ok(())
        });
        // TAKE ( x1 .. xm m n -- x1 .. xk k ): the first n items, k = min(m, n).
        word(f, wid, "TAKE", |f| {
            let (items, k) = f.pop_split("TAKE")?;
            f.stack.extend(&items[..k]);
            f.push(to_cell(k));
            Ok(())
        });
        // DROP-N ( x1 .. xm m n -- xk+1 .. xm m-k ): without the first n items.
        word(f, wid, "DROP-N", |f| {
            let (items, k) = f.pop_split("DROP-N")?;
            f.stack.extend(&items[k..]);
            f.push(to_cell(items.len() - k));
            Ok(())
        });
        // ZIP ( a1 .. an b1 .. bn n -- a1 b1 .. an bn 2n )
        word(f, wid, "ZIP", |f| {
            let n = f.pop()?;
            let n = usize::try_from(n)
                .ok()
                .filter(|&n| n <= f.stack.len() / 2)
                .ok_or_else(|| Throw::with(INVALID_ARGUMENT, format!("ZIP: bad item count {n}")))?;
            let at = f.stack.len() - 2 * n;
            let items = f.stack.split_off(at);
            let (a, b) = items.split_at(n);
            for (&x, &y) in a.iter().zip(b) {
                f.stack.push(x);
                f.stack.push(y);
            }
            f.push(to_cell(2 * n));
            Ok(())
        });
        // SUM, PRODUCT ( x1 .. xn n -- r ): wrapping; 0 and 1 for n = 0.
        word(f, wid, "SUM", |f| {
            let items = f.pop_list("SUM")?;
            f.push(items.into_iter().fold(0, Cell::wrapping_add));
            Ok(())
        });
        word(f, wid, "PRODUCT", |f| {
            let items = f.pop_list("PRODUCT")?;
            f.push(items.into_iter().fold(1, Cell::wrapping_mul));
            Ok(())
        });
        // MIN-OF, MAX-OF ( x1 .. xn n -- r ): n must be at least 1.
        word(f, wid, "MIN-OF", |f| {
            let items = f.pop_list("MIN-OF")?;
            let Some(r) = items.into_iter().min() else {
                return Err(Throw::with(INVALID_ARGUMENT, "MIN-OF: no items"));
            };
            f.push(r);
            Ok(())
        });
        word(f, wid, "MAX-OF", |f| {
            let items = f.pop_list("MAX-OF")?;
            let Some(r) = items.into_iter().max() else {
                return Err(Throw::with(INVALID_ARGUMENT, "MAX-OF: no items"));
            };
            f.push(r);
            Ok(())
        });
        // ANY?-CELLS, ALL?-CELLS ( addr n xt -- flag ), FIND-CELLS
        // ( addr n xt -- x true | false ) and COUNT-IF-CELLS ( addr n xt -- m )
        // are the same on a block of cells; they leave the block as it is.
        word(f, wid, "ANY?-CELLS", |f| {
            let (call, addr, count) = f.pop_block("ANY?-CELLS")?;
            let base = f.stack.len();
            let mut found = false;
            for i in 0..count {
                let x = f.get(addr + i * CELL);
                if call.apply(f, "ANY?-CELLS", base, &[x])? != 0 {
                    found = true;
                    break;
                }
            }
            f.push(-Cell::from(found));
            Ok(())
        });
        word(f, wid, "ALL?-CELLS", |f| {
            let (call, addr, count) = f.pop_block("ALL?-CELLS")?;
            let base = f.stack.len();
            let mut all = true;
            for i in 0..count {
                let x = f.get(addr + i * CELL);
                if call.apply(f, "ALL?-CELLS", base, &[x])? == 0 {
                    all = false;
                    break;
                }
            }
            f.push(-Cell::from(all));
            Ok(())
        });
        word(f, wid, "FIND-CELLS", |f| {
            let (call, addr, count) = f.pop_block("FIND-CELLS")?;
            let base = f.stack.len();
            for i in 0..count {
                let x = f.get(addr + i * CELL);
                if call.apply(f, "FIND-CELLS", base, &[x])? != 0 {
                    f.push(x);
                    f.push(-1);
                    return Ok(());
                }
            }
            f.push(0);
            Ok(())
        });
        word(f, wid, "COUNT-IF-CELLS", |f| {
            let (call, addr, count) = f.pop_block("COUNT-IF-CELLS")?;
            let base = f.stack.len();
            let mut m = 0;
            for i in 0..count {
                let x = f.get(addr + i * CELL);
                if call.apply(f, "COUNT-IF-CELLS", base, &[x])? != 0 {
                    m += 1;
                }
            }
            f.push(m);
            Ok(())
        });
        // REVERSE-CELLS ( addr n -- ): in place.
        word(f, wid, "REVERSE-CELLS", |f| {
            let (addr, count) = f.pop_cells("REVERSE-CELLS")?;
            for i in 0..count / 2 {
                let (lo, hi) = (addr + i * CELL, addr + (count - 1 - i) * CELL);
                let (x, y) = (f.get(lo), f.get(hi));
                f.set(lo, y);
                f.set(hi, x);
            }
            Ok(())
        });
        // SUM-CELLS, PRODUCT-CELLS ( addr n -- r ): wrapping; 0 and 1 for n = 0.
        word(f, wid, "SUM-CELLS", |f| {
            let (addr, count) = f.pop_cells("SUM-CELLS")?;
            let r = (0..count).fold(0, |acc, i| Cell::wrapping_add(acc, f.get(addr + i * CELL)));
            f.push(r);
            Ok(())
        });
        word(f, wid, "PRODUCT-CELLS", |f| {
            let (addr, count) = f.pop_cells("PRODUCT-CELLS")?;
            let r = (0..count).fold(1, |acc, i| Cell::wrapping_mul(acc, f.get(addr + i * CELL)));
            f.push(r);
            Ok(())
        });
        // RANGE ( a b -- a .. b-1 n ): the numbers from a up to b.
        word(f, wid, "RANGE", |f| {
            let (a, b) = f.pop2()?;
            let n = i128::from(b) - i128::from(a);
            if n > 0 && n >= (STACK_CELLS - f.stack.len()) as i128 {
                return Err(Throw::new(STACK_OVERFLOW));
            }
            #[allow(
                clippy::cast_possible_truncation,
                reason = "n is at most STACK_CELLS here, checked above"
            )]
            let n = n.max(0) as Cell;
            f.stack.extend(a..a + n);
            f.push(n);
            Ok(())
        });
    }
}
