//! Defining words and the compiler words that go with them.

use crate::*;

pub(crate) fn install(f: &mut Forth) {
    f.prim(":", |f| {
        let name = f.name()?;
        f.start_def(&name)?;
        Ok(())
    });
    f.prim(":NONAME", |f| {
        let xt = f.start_def("")?;
        f.push(to_cell(xt));
        Ok(())
    });
    // Quotations: [: ... ;] compiles a nameless word and leaves its xt.
    f.imm("[:", Forth::start_quote);
    f.imm(";]", Forth::end_quote);
    f.imm(";", |f| {
        if !f.quotes.is_empty() {
            return Err(Throw::new(CONTROL_MISMATCH)); // a quotation ends with ;]
        }
        let xt = f.current.ok_or(Throw::new(CONTROL_MISMATCH))?;
        if !f.ctrl.is_empty() {
            return Err(Throw::new(CONTROL_MISMATCH));
        }
        f.count_code(xt, f.code.len())?;
        f.dict[xt].kind = Kind::Colon(Rc::new(std::mem::take(&mut f.code)));
        // A :NONAME definition is only reachable through its xt.
        f.dict[xt].hidden = f.dict[xt].name.is_empty();
        f.current = None;
        f.set_state(false);
        f.end_local_scope();
        Ok(())
    });
    f.prim("IMMEDIATE", |f| {
        f.dict.last_mut().unwrap().immediate = true;
        Ok(())
    });
    f.prim("CREATE", |f| {
        let name = f.name()?;
        f.align_here();
        let addr = to_cell(f.here);
        f.new_word(&name, Kind::Created { addr, does: None })?;
        Ok(())
    });
    f.compile_imm("DOES>", |f| {
        let at = f.code.len();
        f.code.push(Instr::SetDoes(at + 2));
        f.code.push(Instr::Exit);
        f.end_local_scope(); // the DOES> code runs in a frame of its own
        Ok(())
    });
    f.prim("CONSTANT", |f| {
        let name = f.name()?;
        let v = f.pop()?;
        f.new_word(&name, Kind::Constant(v))?;
        Ok(())
    });
    f.prim("VALUE", |f| {
        let name = f.name()?;
        let v = f.pop()?;
        f.align_here();
        let addr = f.reserve(CELL)?;
        f.set(addr, v);
        f.new_word(&name, Kind::Value(addr))?;
        Ok(())
    });
    f.prim("2VALUE", |f| {
        let name = f.name()?;
        let (x1, x2) = f.pop2()?;
        f.align_here();
        let addr = f.reserve(2 * CELL)?;
        f.set(addr, x2);
        f.set(addr + CELL, x1);
        f.new_word(&name, Kind::TwoValue(addr))?;
        Ok(())
    });
    f.prim("DEFER", |f| {
        let name = f.name()?;
        f.align_here();
        let addr = f.reserve(CELL)?;
        f.set(addr, -1); // no action yet
        f.new_word(&name, Kind::Defer(addr))?;
        Ok(())
    });
    f.prim("MARKER", |f| {
        let name = f.name()?;
        let (dict_len, here, wordlists) = (f.dict.len(), f.here, f.wordlists.len());
        let (order, current_wid) = (f.order.clone(), f.current_wid);
        f.new_word(&name, Kind::Marker { dict_len, here, wordlists, order, current_wid })?;
        Ok(())
    });
    // FORGET name: deletes the word and everything defined after it. It
    // looks in the compilation word list only.
    f.prim("FORGET", |f| {
        let name = f.name()?;
        let xt =
            f.find_in(&name, f.current_wid).ok_or_else(|| Throw::with(UNDEFINED_WORD, &name))?;
        if xt < f.fence {
            return Err(Throw::with(INVALID_FORGET, name));
        }
        f.here = f.dict[xt].here;
        f.forget_from(xt);
        Ok(())
    });
    // TO, IS and ACTION-OF parse a name; compiled, they become Store/Fetch.
    f.imm("TO", |f| {
        let name = f.name()?;
        if let Some(i) = f.find_local(&name) {
            f.code.push(Instr::ToLocal(i));
            return Ok(());
        }
        let xt = f.find_or_err(&name)?;
        if let Kind::TwoValue(addr) = f.dict[xt].kind {
            // x2 is on top and goes in the first cell, as with 2!
            f.store_to(addr)?;
            return f.store_to(addr + CELL);
        }
        if let Kind::FValue(addr) = f.dict[xt].kind {
            if f.compiling() {
                f.code.push(Instr::FStore(addr));
            } else {
                let r = f.fpop()?;
                f.fset(addr, r);
            }
            return Ok(());
        }
        let addr = f.cell_of(to_cell(xt), false).map_err(|e| Throw { detail: Some(name), ..e })?;
        f.store_to(addr)
    });
    f.imm("IS", |f| {
        let addr = f.named_cell(true)?;
        f.store_to(addr)
    });
    f.imm("ACTION-OF", |f| {
        let addr = f.named_cell(true)?;
        if f.compiling() {
            f.code.push(Instr::Fetch(addr));
        } else {
            f.push(f.get(addr));
        }
        Ok(())
    });
    f.prim("DEFER@", |f| {
        let xt = f.pop()?;
        let addr = f.cell_of(xt, true)?;
        f.push(f.get(addr));
        Ok(())
    });
    f.prim("DEFER!", |f| {
        let (action, xt) = f.pop2()?;
        let addr = f.cell_of(xt, true)?;
        f.set(addr, action);
        Ok(())
    });
    f.prim("'", |f| {
        let name = f.name()?;
        let xt = f.find_or_err(&name)?;
        f.push(to_cell(xt));
        Ok(())
    });
    f.compile_imm("[']", |f| {
        let name = f.name()?;
        let xt = f.find_or_err(&name)?;
        f.code.push(Instr::Lit(to_cell(xt)));
        Ok(())
    });
    f.prim("EXECUTE", |f| {
        let xt = f.pop()?;
        if xt < 0 {
            return Err(Throw::with(INVALID_ARGUMENT, format!("invalid execution token {xt}")));
        }
        f.invoke(to_index(xt))
    });
    f.prim("CATCH", |f| {
        let xt = f.pop()?;
        let code = f.catch(xt)?;
        f.push(code);
        Ok(())
    });
    f.prim("THROW", |f| match f.pop()? {
        0 => Ok(()),
        code => Err(Throw::new(code)),
    });
    f.prim(">BODY", |f| {
        let xt = f.pop()?;
        match usize::try_from(xt).ok().and_then(|i| f.dict.get(i)) {
            Some(Word { kind: Kind::Created { addr, .. }, .. }) => {
                let addr = *addr;
                f.push(addr);
                Ok(())
            }
            _ => Err(Throw::new(NOT_CREATED)),
        }
    });
    f.prim("CHAR", |f| {
        let (a, n) = f.parse_name();
        if n == 0 {
            return Err(Throw::new(ZERO_LENGTH_NAME));
        }
        f.push(decode(&f.mem[a..a + n]).0);
        Ok(())
    });
    f.compile_imm("[CHAR]", |f| {
        let (a, n) = f.parse_name();
        if n == 0 {
            return Err(Throw::new(ZERO_LENGTH_NAME));
        }
        f.code.push(Instr::Lit(decode(&f.mem[a..a + n]).0));
        Ok(())
    });
    f.compile_imm("LITERAL", |f| {
        let v = f.pop()?;
        f.code.push(Instr::Lit(v));
        Ok(())
    });
    f.imm("[", |f| {
        f.set_state(false);
        Ok(())
    });
    f.compile_prim("]", |f| {
        f.set_state(true);
        Ok(())
    });
    f.compile_imm("POSTPONE", |f| {
        let name = f.name()?;
        let xt = f.find_or_err(&name)?;
        f.code.push(if f.dict[xt].immediate { Instr::Call(xt) } else { Instr::Compile(xt) });
        Ok(())
    });
    f.compile_prim("COMPILE,", |f| {
        let xt = f.pop()?;
        match usize::try_from(xt).ok().filter(|&i| i < f.dict.len()) {
            Some(xt) => f.code.push(Instr::Call(xt)),
            None => {
                return Err(Throw::with(INVALID_ARGUMENT, format!("invalid execution token {xt}")));
            }
        }
        Ok(())
    });
    f.compile_imm("[COMPILE]", |f| {
        let name = f.name()?;
        let xt = f.find_or_err(&name)?;
        f.code.push(Instr::Call(xt));
        Ok(())
    });
    f.compile_imm("RECURSE", |f| {
        f.code.push(Instr::Call(f.current.unwrap()));
        Ok(())
    });
    f.compile_imm("EXIT", |f| {
        f.code.push(Instr::Exit);
        Ok(())
    });
}
