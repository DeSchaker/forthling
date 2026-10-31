//! The host API: what a Rust program, or an extension, needs to add words
//! of its own and to exchange data with Forth. The stack methods `push`,
//! `pop`, `fpush`, `fpop` and `pop_str` are public too (stack.rs, float.rs,
//! memory.rs).

use crate::*;
use std::any::{Any, TypeId};

/// The code of a word added through the host API.
pub(crate) type HostWord = dyn Fn(&mut Forth) -> Res;

/// A set of words that a host program adds, usually in a word list of its
/// own, so that standard programs never meet them by accident.
///
/// ```
/// use forthling::{Cell, Extension, Forth};
///
/// struct Greet;
///
/// impl Extension for Greet {
///     fn name(&self) -> &str {
///         "FORTHLING-GREET"
///     }
///
///     fn install(&self, f: &mut Forth) {
///         let wid = f.wordlist("GREET");
///         f.add_word(wid, "HELLO", |f| {
///             let n = f.pop()?;
///             f.push(n + 1);
///             Ok(())
///         })
///         .unwrap();
///     }
/// }
///
/// let mut f = Forth::new();
/// f.install(&Greet);
/// f.interpret("S\" FORTHLING-GREET\" ENVIRONMENT? . .  ALSO GREET 41 HELLO .")?;
/// assert_eq!(f.output(), b"-1 1 42 ");
/// # Ok::<(), forthling::Throw>(())
/// ```
pub trait Extension {
    /// The name ENVIRONMENT? knows the extension by, such as
    /// "FORTHLING-SQLITE".
    fn name(&self) -> &str;

    /// The version ENVIRONMENT? gives below its true flag.
    fn version(&self) -> Cell {
        1
    }

    /// Adds the words, with `Forth::wordlist` and `Forth::add_word`.
    fn install(&self, f: &mut Forth);
}

impl Forth {
    /// Installs an extension: adds its words and makes its name known to
    /// ENVIRONMENT?. Its words then belong to the system, so FORGET leaves
    /// them alone.
    pub fn install(&mut self, extension: &dyn Extension) {
        extension.install(self);
        self.extensions.insert(extension.name().to_ascii_uppercase(), extension.version());
        self.fence = self.dict.len();
    }

    /// The word list with this name, made if there is none yet; FORTH gives
    /// FORTH-WORDLIST. A new word list also gets a word of its name in
    /// FORTH-WORDLIST that, like FORTH, replaces the first word list of the
    /// search order, so that `ALSO NAME` searches it.
    pub fn wordlist(&mut self, name: &str) -> Cell {
        let upper = name.to_ascii_uppercase();
        if let Some(i) = self.wordlists.iter().position(|w| w.as_deref() == Some(upper.as_str())) {
            return i as Cell + 1;
        }
        self.wordlists.push(Some(upper.clone()));
        let wid = self.wordlists.len();
        let xt = self.define(
            &upper,
            Kind::Host(Rc::new(move |f| {
                f.set_first(wid);
                Ok(())
            })),
        );
        self.dict[xt].wid = FORTH_WID;
        wid as Cell
    }

    /// Adds a word to the word list `wid` (FORTH-WORDLIST is 1). The
    /// closure runs each time the word executes; it can keep state of its
    /// own, or use `Forth::state`. Throws -24 for an unknown word list and
    /// -16 for an empty name.
    pub fn add_word(
        &mut self,
        wid: Cell,
        name: &str,
        word: impl Fn(&mut Forth) -> Res + 'static,
    ) -> Res {
        let wid = self.valid_wid(wid)?;
        if name.is_empty() {
            return Err(Throw::new(ZERO_LENGTH_NAME));
        }
        let xt = self.define(name, Kind::Host(Rc::new(word)));
        self.dict[xt].wid = wid;
        Ok(())
    }

    /// The state of type `T` that an extension keeps, such as its open
    /// connections; made with `T::default()` the first time it is asked for.
    pub fn state<T: Any + Default>(&mut self) -> &mut T {
        self.host_state
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::<T>::default())
            .downcast_mut()
            .expect("each state is stored under its own type")
    }

    /// The number of cells on the data stack.
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// The data stack, the top last.
    pub fn stack(&self) -> &[Cell] {
        &self.stack
    }

    /// The output produced since the last `flush` or `take_output`, which
    /// the system then forgets; for hosts that show it themselves.
    pub fn take_output(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.out)
    }

    /// Pops a string ( c-addr u ) and returns a copy of it.
    pub fn pop_bytes(&mut self) -> Res<Vec<u8>> {
        let (i, n) = self.pop_str()?;
        Ok(self.mem[i..i + n].to_vec())
    }

    /// Copies `bytes` to a new block on the heap and pushes it as
    /// ( c-addr u ); the program frees it with FREE. Throws -59 when the
    /// heap is full.
    pub fn push_bytes(&mut self, bytes: &[u8]) -> Res {
        let a = self.heap.allocate(bytes.len(), self.here).ok_or(Throw::new(ALLOCATE_FAILED))?;
        self.mem[a..a + bytes.len()].copy_from_slice(bytes);
        self.push(a as Cell);
        self.push(bytes.len() as Cell);
        Ok(())
    }

    /// Forth memory, for the addresses that `pop_str` returns.
    pub fn memory(&self) -> &[u8] {
        &self.mem
    }

    /// Forth memory, to write in.
    pub fn memory_mut(&mut self) -> &mut [u8] {
        &mut self.mem
    }
}
