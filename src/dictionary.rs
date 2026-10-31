//! The dictionary: words, defining them, finding them, SEE and WORDS.

use crate::*;

pub(crate) const FORTH_WID: usize = 1; // word list ids count from 1
pub(crate) const MAX_ORDER: usize = 16; // word lists in the search order

#[derive(Clone)]
pub(crate) enum Kind {
    Prim(Prim),
    Host(Rc<HostWord>), // a word added through the host API (host.rs)
    Colon(Rc<Vec<Instr>>),
    Constant(Cell),
    Created {
        addr: Cell,
        does: Option<(Rc<Vec<Instr>>, usize)>,
    },
    Value(usize),    // address of the value's cell
    TwoValue(usize), // address of a 2VALUE's two cells, laid out as by 2!
    FValue(usize),   // address of an FVALUE's float
    Defer(usize),    // address of the cell holding the xt to run
    Marker {
        dict_len: usize, // dictionary length before the marker, itself included
        here: usize,
        wordlists: usize,
        order: Vec<usize>,
        current_wid: usize,
    },
}

pub(crate) struct Word {
    pub(crate) name: String,
    pub(crate) kind: Kind,
    pub(crate) immediate: bool,
    pub(crate) compile_only: bool, // a primitive that throws -14 outside a definition
    pub(crate) hidden: bool,
    pub(crate) wid: usize,               // the word list it is in
    pub(crate) here: usize,              // HERE when it was defined, for FORGET
    pub(crate) name_addr: Option<usize>, // its name on the heap, once NAME>STRING asked
}

/// The words an empty search order still finds.
pub(crate) const ROOT_WORDS: [&str; 6] =
    ["FORTH", "FORTH-WORDLIST", "GET-ORDER", "ONLY", "ORDER", "SET-ORDER"];

impl Forth {
    pub(crate) fn define(&mut self, name: &str, kind: Kind) -> usize {
        self.dict.push(Word {
            name: name.to_ascii_uppercase(),
            kind,
            immediate: false,
            compile_only: false,
            hidden: false,
            wid: self.current_wid,
            here: self.here,
            name_addr: None,
        });
        self.dict.len() - 1
    }

    pub(crate) fn prim(&mut self, name: &str, f: Prim) {
        self.define(name, Kind::Prim(f));
    }

    pub(crate) fn imm(&mut self, name: &str, f: Prim) {
        let xt = self.define(name, Kind::Prim(f));
        self.dict[xt].immediate = true;
    }

    /// A primitive that may only run while a definition is being compiled.
    pub(crate) fn compile_prim(&mut self, name: &str, f: Prim) {
        let xt = self.define(name, Kind::Prim(f));
        self.dict[xt].compile_only = true;
    }

    /// An immediate primitive that may only run while a definition is being compiled.
    pub(crate) fn compile_imm(&mut self, name: &str, f: Prim) {
        self.compile_prim(name, f);
        self.dict.last_mut().unwrap().immediate = true;
    }

    pub(crate) fn variable(&mut self, name: &str, addr: usize) {
        self.define(name, Kind::Created { addr: addr as Cell, does: None });
    }

    /// Deletes the words from index `len` on, for MARKER and FORGET.
    pub(crate) fn forget_from(&mut self, len: usize) {
        for w in self.dict.drain(len..) {
            if let Some(a) = w.name_addr {
                self.heap.free(a);
            }
        }
    }

    /// Looks a word up in the word lists of the search order, in turn.
    /// Looks a word up in the search order. An empty search order still
    /// finds the words of ROOT_WORDS, so the system stays usable.
    pub(crate) fn find(&self, name: &str) -> Option<usize> {
        if self.order.is_empty() && ROOT_WORDS.contains(&name.to_ascii_uppercase().as_str()) {
            return self.find_in(name, FORTH_WID);
        }
        self.order.iter().find_map(|&wid| self.find_in(name, wid))
    }

    /// Looks a word up in one word list.
    pub(crate) fn find_in(&self, name: &str, wid: usize) -> Option<usize> {
        let upper = name.to_ascii_uppercase();
        self.dict.iter().rposition(|w| !w.hidden && w.wid == wid && w.name == upper)
    }

    pub(crate) fn find_or_err(&self, name: &str) -> Res<usize> {
        self.find(name).ok_or_else(|| Throw::with(UNDEFINED_WORD, name))
    }

    pub(crate) fn show(&self, instr: &Instr) -> String {
        match instr {
            Instr::Call(xt) => self.dict[*xt].name.clone(),
            Instr::Lit(n) => n.to_string(),
            Instr::Branch(t) => format!("BRANCH {t}"),
            Instr::ZBranch(t) => format!("0BRANCH {t}"),
            Instr::Of(t) => format!("(OF) {t}"),
            Instr::Drop => "DROP".into(),
            Instr::Do(t) => format!("(DO) {t}"),
            Instr::QDo(t) => format!("(?DO) {t}"),
            Instr::Loop(t) => format!("(LOOP) {t}"),
            Instr::PlusLoop(t) => format!("(+LOOP) {t}"),
            Instr::Leave => "LEAVE".into(),
            Instr::Print(s) => format!(".\" {}\"", String::from_utf8_lossy(s)),
            Instr::Abort(s) => format!("ABORT\" {}\"", String::from_utf8_lossy(s)),
            Instr::Compile(xt) => format!("POSTPONE {}", self.dict[*xt].name),
            Instr::SetDoes(_) => "DOES>".into(),
            Instr::Store(addr) => format!("{addr} !"),
            Instr::Fetch(addr) => format!("{addr} @"),
            Instr::FLit(r) => format!("{r:E}"),
            Instr::FStore(addr) => format!("{addr} F!"),
            Instr::Locals(take, zeroed) => format!("(LOCALS) {take} {zeroed}"),
            Instr::Local(i) => format!("LOCAL#{i}"),
            Instr::ToLocal(i) => format!("TO LOCAL#{i}"),
            Instr::Exit => "EXIT".into(),
        }
    }

    pub(crate) fn see(&mut self) -> Res {
        let name = self.name()?;
        let w = &self.dict[self.find_or_err(&name)?];
        let mut text = match &w.kind {
            Kind::Prim(_) if w.compile_only => format!("{} is a compile-only primitive", w.name),
            Kind::Prim(_) => format!("{} is a primitive", w.name),
            Kind::Host(_) => format!("{} is a host word", w.name),
            Kind::Constant(v) => format!("{v} CONSTANT {}", w.name),
            Kind::Created { addr, does } => format!(
                "CREATE {} ( address {addr} ){}",
                w.name,
                if does.is_some() { " with DOES>" } else { "" }
            ),
            Kind::Colon(code) => {
                let mut s = format!(": {}\n", w.name);
                for (i, instr) in code.iter().enumerate() {
                    s += &format!("{i:>4}  {}\n", self.show(instr));
                }
                s + ";"
            }
            Kind::Value(addr) => format!("{} VALUE {}", self.get(*addr), w.name),
            Kind::FValue(addr) => format!("{:E} FVALUE {}", self.fget(*addr), w.name),
            Kind::TwoValue(addr) => {
                format!("{} {} 2VALUE {}", self.get(*addr + CELL), self.get(*addr), w.name)
            }
            Kind::Defer(addr) => match self.dict.get(self.get(*addr) as usize) {
                Some(action) => format!("DEFER {} IS {}", w.name, action.name),
                None => format!("DEFER {}", w.name),
            },
            Kind::Marker { .. } => format!("MARKER {}", w.name),
        };
        if w.immediate {
            text += " IMMEDIATE";
        }
        text.push('\n');
        self.emit_str(&text);
        Ok(())
    }

    /// WORDS: the words of the first word list in the search order.
    pub(crate) fn words(&mut self) {
        let mut seen = HashSet::new();
        let mut text = String::new();
        let mut line = String::new();
        let first = self.order.first().copied();
        for w in self.dict.iter().rev() {
            if w.hidden || Some(w.wid) != first || !seen.insert(w.name.as_str()) {
                continue;
            }
            if line.len() + w.name.len() > 72 {
                text += line.trim_end();
                text.push('\n');
                line.clear();
            }
            line += &w.name;
            line.push(' ');
        }
        text += line.trim_end();
        text.push('\n');
        self.emit_str(&text);
    }
}
