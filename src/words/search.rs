//! The SEARCH word set and its extensions, plus the TOOLS EXT words that
//! work on word lists and name tokens.
//!
//! A wid is the number of a word list, starting at 1 for FORTH-WORDLIST.
//! A name token is the same number as the execution token.

use crate::*;

pub(crate) fn install(f: &mut Forth) {
    install_search(f);
    install_search_ext(f);
    install_tools(f);
}

fn install_search(f: &mut Forth) {
    f.prim("FORTH-WORDLIST", |f| {
        f.push(to_cell(FORTH_WID));
        Ok(())
    });
    f.prim("WORDLIST", |f| {
        f.wordlists.push(None);
        if let Err(e) = f.check_space() {
            f.wordlists.pop();
            return Err(e);
        }
        f.push(to_cell(f.wordlists.len()));
        Ok(())
    });
    f.prim("GET-CURRENT", |f| {
        f.push(to_cell(f.current_wid));
        Ok(())
    });
    f.prim("SET-CURRENT", |f| {
        f.current_wid = f.pop_wid()?;
        Ok(())
    });
    f.prim("GET-ORDER", |f| {
        for i in (0..f.order.len()).rev() {
            f.push(to_cell(f.order[i]));
        }
        f.push(to_cell(f.order.len()));
        Ok(())
    });
    f.prim("SET-ORDER", |f| {
        let n = f.pop()?;
        if n == -1 {
            f.order = vec![FORTH_WID];
            return Ok(());
        }
        let n = usize::try_from(n).map_err(|_| Throw::new(INVALID_ARGUMENT))?;
        if n > MAX_ORDER {
            return Err(Throw::new(ORDER_OVERFLOW));
        }
        let mut order = Vec::with_capacity(n);
        for _ in 0..n {
            order.push(f.pop_wid()?);
        }
        f.order = order;
        Ok(())
    });
    f.prim("DEFINITIONS", |f| {
        f.current_wid = *f.order.first().ok_or(Throw::new(ORDER_UNDERFLOW))?;
        Ok(())
    });
    f.prim("SEARCH-WORDLIST", |f| {
        let wid = f.pop_wid()?;
        let (i, n) = f.pop_str()?;
        let name = f.text(i, n);
        match f.find_in(&name, wid) {
            Some(xt) => {
                f.push(to_cell(xt));
                f.push(if f.dict[xt].immediate { 1 } else { -1 });
            }
            None => f.push(0),
        }
        Ok(())
    });
}

fn install_search_ext(f: &mut Forth) {
    // SEARCH EXT
    f.prim("FORTH", |f| {
        f.set_first(FORTH_WID);
        Ok(())
    });
    f.prim("ALSO", |f| {
        let first = *f.order.first().ok_or(Throw::new(ORDER_UNDERFLOW))?;
        if f.order.len() == MAX_ORDER {
            return Err(Throw::new(ORDER_OVERFLOW));
        }
        f.order.insert(0, first);
        Ok(())
    });
    f.prim("ONLY", |f| {
        f.order = vec![FORTH_WID];
        Ok(())
    });
    f.prim("PREVIOUS", |f| {
        if f.order.is_empty() {
            return Err(Throw::new(ORDER_UNDERFLOW));
        }
        f.order.remove(0);
        Ok(())
    });
    f.prim("ORDER", |f| {
        let names: Vec<String> = f.order.iter().map(|&wid| f.wordlist_name(wid)).collect();
        let text = format!(
            "Search order: {}\nDefinitions: {}\n",
            names.join(" "),
            f.wordlist_name(f.current_wid)
        );
        f.emit_str(&text);
        Ok(())
    });
}

fn install_tools(f: &mut Forth) {
    // TOOLS EXT
    f.prim("TRAVERSE-WORDLIST", |f| {
        let wid = f.pop_wid()?;
        let xt = f.pop()?;
        let xt = usize::try_from(xt).map_err(|_| Throw::with(INVALID_ARGUMENT, xt.to_string()))?;
        let mut nt = f.dict.len();
        // The xt may shorten the dictionary (a marker, FORGET); go on below its new end.
        while let Some(i) =
            f.dict[..nt.min(f.dict.len())].iter().rposition(|w| w.wid == wid && !w.hidden)
        {
            nt = i;
            f.push(to_cell(nt));
            f.execute(xt)?;
            if f.pop()? == 0 {
                break;
            }
        }
        Ok(())
    });
    f.prim("NAME>STRING", |f| {
        let nt = f.pop_nt()?;
        let a = f.name_in_memory(nt)?;
        f.push(to_cell(a));
        f.push(to_cell(f.dict[nt].name.len()));
        Ok(())
    });
    f.prim("NAME>INTERPRET", |f| {
        let nt = f.pop_nt()?;
        f.push(to_cell(nt));
        Ok(())
    });
    // Compiling a word is executing it if it is immediate, else COMPILE, it.
    f.prim("NAME>COMPILE", |f| {
        let nt = f.pop_nt()?;
        f.push(to_cell(nt));
        let action = if f.dict[nt].immediate { "EXECUTE" } else { "COMPILE," };
        f.push(to_cell(f.find_in(action, FORTH_WID).unwrap()));
        Ok(())
    });
}

impl Forth {
    /// Pops a word list id and checks it.
    pub(crate) fn pop_wid(&mut self) -> Res<usize> {
        let wid = self.pop()?;
        self.valid_wid(wid)
    }

    pub(crate) fn valid_wid(&self, wid: Cell) -> Res<usize> {
        match usize::try_from(wid) {
            Ok(w) if (1..=self.wordlists.len()).contains(&w) => Ok(w),
            _ => Err(Throw::with(INVALID_ARGUMENT, format!("invalid word list {wid}"))),
        }
    }

    pub(crate) fn pop_nt(&mut self) -> Res<usize> {
        let nt = self.pop()?;
        match usize::try_from(nt) {
            Ok(i) if i < self.dict.len() => Ok(i),
            _ => Err(Throw::with(INVALID_ARGUMENT, format!("invalid name token {nt}"))),
        }
    }

    /// Replaces the first word list of the search order.
    pub(crate) fn set_first(&mut self, wid: usize) {
        match self.order.first_mut() {
            Some(first) => *first = wid,
            None => self.order.push(wid),
        }
    }

    fn wordlist_name(&self, wid: usize) -> String {
        self.wordlists[wid - 1].clone().unwrap_or_else(|| format!("#{wid}"))
    }

    /// The name of a word in Forth memory, for NAME>STRING. It is copied to
    /// the heap the first time it is asked for.
    fn name_in_memory(&mut self, nt: usize) -> Res<usize> {
        if let Some(a) = self.dict[nt].name_addr {
            return Ok(a);
        }
        let name = self.dict[nt].name.clone().into_bytes();
        let a = self.heap.allocate(name.len(), self.here).ok_or(Throw::new(DICTIONARY_OVERFLOW))?;
        self.mem[a..a + name.len()].copy_from_slice(&name);
        self.dict[nt].name_addr = Some(a);
        Ok(a)
    }
}
