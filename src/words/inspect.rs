//! Words to look at the system: names and where words were defined, a
//! backtrace, `~~` and the ASSERT family, and the lists of words and files.
//! They follow the manual of Gforth; see the documentation for how they differ.

use crate::*;
use std::fmt::Write as _;
use std::io::Read as _;

pub(crate) fn install(f: &mut Forth) {
    install_names(f);
    install_where(f);
    install_trace(f);
    install_assert(f);
    install_lists(f);
}

fn install_names(f: &mut Forth) {
    // In forthling a name token and an execution token are the same number.
    f.prim("LATEST", |f| {
        f.push(to_cell(f.dict.len() - 1));
        Ok(())
    });
    f.prim("LATESTXT", |f| {
        f.push(to_cell(f.dict.len() - 1));
        Ok(())
    });
    // FIND-NAME ( c-addr u -- nt | 0 ): looks a name up in the search order.
    f.prim("FIND-NAME", |f| {
        let (i, n) = f.pop_str()?;
        let name = f.text(i, n);
        f.push(f.find(&name).map_or(0, to_cell));
        Ok(())
    });
    // NAME>LINK ( nt1 -- nt2 | 0 ): the word defined before it in its word list.
    f.prim("NAME>LINK", |f| {
        let nt = f.pop_nt()?;
        let wid = f.dict[nt].wid;
        let before = f.dict[..nt].iter().rposition(|w| w.wid == wid && !w.hidden);
        f.push(before.map_or(0, to_cell));
        Ok(())
    });
    // XT>NAME ( xt -- nt )
    f.prim("XT>NAME", |f| {
        let nt = f.pop_nt()?;
        f.push(to_cell(nt));
        Ok(())
    });
}

fn install_where(f: &mut Forth) {
    // SEE-CODE shows what SEE shows: the instructions of the word.
    f.prim("SEE-CODE", Forth::see);
    f.prim("WHERE", |f| {
        let name = f.name()?;
        let xt = f.find_or_err(&name)?;
        let text = f.where_defined(xt);
        f.emit_str(&format!("{text}\n"));
        Ok(())
    });
    // LOCATE shows the line of the source as well, when the file can be read.
    f.prim("LOCATE", |f| {
        let name = f.name()?;
        let xt = f.find_or_err(&name)?;
        let mut text = f.where_defined(xt);
        if let Some((source, line)) = f.dict[xt].origin.clone() {
            if let Some(src) = f.source_line(&source, line) {
                let _ = write!(
                    text,
                    "
{line:>5}  {src}"
                );
            }
        }
        f.emit_str(&format!("{text}\n"));
        Ok(())
    });
}

fn install_trace(f: &mut Forth) {
    // BT ( -- ): the words that are running, the innermost first.
    f.prim("BT", |f| {
        f.backtrace();
        Ok(())
    });
    // ~~ ( -- ): prints the place in the source and the data stack, where
    // it is compiled (or at once when it is interpreted).
    f.imm("~~", |f| f.trace_here(false));
    f.imm("~~BT", |f| f.trace_here(true));
}

fn install_assert(f: &mut Forth) {
    // ASSERT( code ): the code leaves a flag; false throws -2 with the place.
    // It is compiled only when ASSERT-LEVEL is at least the level of the
    // word: ASSERT0( is 0, ASSERT1( and ASSERT( 1, ASSERT2( 2, ASSERT3( 3.
    f.compile_imm("ASSERT(", |f| {
        f.assert_open(1);
        Ok(())
    });
    f.compile_imm("ASSERT0(", |f| {
        f.assert_open(0);
        Ok(())
    });
    f.compile_imm("ASSERT1(", |f| {
        f.assert_open(1);
        Ok(())
    });
    f.compile_imm("ASSERT2(", |f| {
        f.assert_open(2);
        Ok(())
    });
    f.compile_imm("ASSERT3(", |f| {
        f.assert_open(3);
        Ok(())
    });
    f.compile_imm(")", |f| {
        let Some(Ctrl::Assert) = f.ctrl.pop() else {
            return Err(Throw::with(CONTROL_MISMATCH, ") without ASSERT("));
        };
        let (source, line, column, _) = f.place(f.first_line);
        let msg = format!("assertion failed at {source}:{line}:{column}");
        let zero_equals = f.find_in("0=", FORTH_WID).expect("0= is in the prelude");
        f.code.push(Instr::Call(zero_equals));
        f.code.push(Instr::Abort(Rc::from(msg.as_bytes())));
        Ok(())
    });
}

fn install_lists(f: &mut Forth) {
    f.prim("VLIST", |f| {
        f.words();
        Ok(())
    });
    // INCLUDED? ( c-addr u -- flag ): whether the file was included before.
    f.prim("INCLUDED?", |f| {
        let path = f.pop_name()?;
        let path = f.resolve(&path);
        let canonical = f.vfs.canonical(&path);
        f.push(flag(f.included.contains(&canonical)));
        Ok(())
    });
    // .INCLUDED ( -- ): the files included so far, in order.
    f.prim(".INCLUDED", |f| {
        let mut text = String::new();
        for p in &f.included {
            let _ = writeln!(text, "{p}");
        }
        f.emit_str(&text);
        Ok(())
    });
}

impl Forth {
    /// Where the program defined a word, as a line of text.
    fn where_defined(&self, xt: usize) -> String {
        let w = &self.dict[xt];
        match &w.origin {
            Some((source, line)) => format!("{} defined at {source}:{line}", w.name),
            None => format!("{} is a word of the system", w.name),
        }
    }

    /// One line of a source that is a file, if it can be read.
    fn source_line(&mut self, source: &str, line: usize) -> Option<String> {
        if !self.vfs.exists(source) {
            return None;
        }
        let read = Access { read: true, ..Access::default() };
        let mut file = self.vfs.open(source, read).ok()?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).ok()?;
        let text = String::from_utf8_lossy(&bytes);
        text.lines().nth(line.checked_sub(1)?).map(str::to_string)
    }

    /// BT: the running words, the innermost first, one on each line.
    fn backtrace(&mut self) {
        let mut words = self.context(self.first_line).words;
        // The last one is BT itself.
        if words.last().is_some_and(|w| w == "BT") {
            words.pop();
        }
        let mut text = String::new();
        for w in words.iter().rev() {
            let _ = writeln!(text, "  {w}");
        }
        if text.is_empty() {
            text = "  (no words running)\n".to_string();
        }
        self.emit_str(&format!("backtrace:\n{text}"));
    }

    /// `~~` and `~~BT`: prints the place and the stack, now or when the
    /// definition being compiled runs.
    fn trace_here(&mut self, with_backtrace: bool) -> Res {
        let (source, line, column, _) = self.place(self.first_line);
        let place: Rc<[u8]> = Rc::from(format!("{source}:{line}:{column}: ").as_bytes());
        let dot_s = self.find_in(".S", FORTH_WID).expect(".S is a primitive");
        let bt = self.find_in("BT", FORTH_WID).expect("BT is a primitive");
        if self.compiling() {
            self.code.push(Instr::Print(place));
            self.code.push(Instr::Call(dot_s));
            self.code.push(Instr::Print(Rc::from(&b"\n"[..])));
            if with_backtrace {
                self.code.push(Instr::Call(bt));
            }
        } else {
            self.emit_bytes(&place);
            self.execute(dot_s)?;
            self.emit_str("\n");
            if with_backtrace {
                self.execute(bt)?;
            }
        }
        Ok(())
    }

    fn assert_level(&self) -> Cell {
        let Some(xt) = self.find_in("ASSERT-LEVEL", FORTH_WID) else {
            return 1;
        };
        match self.dict[xt].kind {
            Kind::Created { addr, .. } => self.get(to_index(addr)),
            _ => 1,
        }
    }

    /// `ASSERT(` and the like: compiles the code up to `)` when the
    /// assertion level is high enough, else skips it.
    fn assert_open(&mut self, level: Cell) {
        if self.assert_level() >= level {
            self.ctrl.push(Ctrl::Assert);
        } else {
            self.parse(b')');
        }
    }
}
