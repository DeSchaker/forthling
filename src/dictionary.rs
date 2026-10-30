//! The dictionary: words, defining them, finding them, SEE and WORDS.

use crate::*;

#[derive(Clone)]
pub(crate) enum Kind {
    Prim(Prim),
    Colon(Rc<Vec<Instr>>),
    Constant(Cell),
    Created {
        addr: Cell,
        does: Option<(Rc<Vec<Instr>>, usize)>,
    },
    Value(usize),    // address of the value's cell
    TwoValue(usize), // address of a 2VALUE's two cells, laid out as by 2!
    Defer(usize),    // address of the cell holding the xt to run
    Marker {
        dict_len: usize, // dictionary length before the marker, itself included
        here: usize,
    },
}

pub(crate) struct Word {
    pub(crate) name: String,
    pub(crate) kind: Kind,
    pub(crate) immediate: bool,
    pub(crate) hidden: bool,
}

impl Forth {
    pub(crate) fn define(&mut self, name: &str, kind: Kind) -> usize {
        self.dict.push(Word {
            name: name.to_ascii_uppercase(),
            kind,
            immediate: false,
            hidden: false,
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

    pub(crate) fn variable(&mut self, name: &str, addr: usize) {
        self.define(name, Kind::Created { addr: addr as Cell, does: None });
    }

    pub(crate) fn find(&self, name: &str) -> Option<usize> {
        let upper = name.to_ascii_uppercase();
        self.dict.iter().rposition(|w| !w.hidden && w.name == upper)
    }

    pub(crate) fn find_or_err(&self, name: &str) -> Res<usize> {
        self.find(name).ok_or_else(|| Throw::with(-13, name))
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
            Instr::Exit => "EXIT".into(),
        }
    }

    pub(crate) fn see(&mut self) -> Res {
        let name = self.name()?;
        let w = &self.dict[self.find_or_err(&name)?];
        let mut text = match &w.kind {
            Kind::Prim(_) => format!("{} is a primitive", w.name),
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

    pub(crate) fn words(&mut self) {
        let mut seen = HashSet::new();
        let mut text = String::new();
        let mut line = String::new();
        for w in self.dict.iter().rev() {
            if w.hidden || !seen.insert(w.name.as_str()) {
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
