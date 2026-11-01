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
        self.system_space += self.space_used();
    }

    /// The word list with this name, made if there is none yet; FORTH gives
    /// FORTH-WORDLIST. A new word list also gets a word of its name in
    /// FORTH-WORDLIST that, like FORTH, replaces the first word list of the
    /// search order, so that `ALSO NAME` searches it.
    pub fn wordlist(&mut self, name: &str) -> Cell {
        let upper = name.to_ascii_uppercase();
        if let Some(i) = self.wordlists.iter().position(|w| w.as_deref() == Some(upper.as_str())) {
            return to_cell(i) + 1;
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
        to_cell(wid)
    }

    /// Adds a word to the word list `wid` (FORTH-WORDLIST is 1). The
    /// closure runs each time the word executes; it can keep state of its
    /// own, or use `Forth::state`.
    ///
    /// # Errors
    /// Throws -24 for an unknown word list and -16 for an empty name.
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
    ///
    /// # Panics
    /// Never in practice: each state is stored under its own type, so the
    /// downcast cannot fail.
    pub fn state<T: Any + Default>(&mut self) -> &mut T {
        self.host_state
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::<T>::default())
            .downcast_mut()
            .expect("each state is stored under its own type")
    }

    /// The state of type `T`, if there is one, without making it.
    pub(crate) fn peek_state<T: Any>(&self) -> Option<&T> {
        self.host_state.get(&TypeId::of::<T>())?.downcast_ref()
    }

    /// Forgets the state of type `T`.
    pub(crate) fn forget_state<T: Any>(&mut self) {
        self.host_state.remove(&TypeId::of::<T>());
    }

    /// Sets the clock that TIME&DATE reads: seconds since 1970-01-01 UTC.
    /// By default it is the system clock; WebAssembly has none, so there a
    /// host that wants the time sets it, as the browser page does.
    pub fn set_clock(&mut self, now: impl Fn() -> u64 + 'static) {
        self.clock = Some(Box::new(now));
        self.started = self.now_micros();
    }

    /// The time of the clock, in seconds since 1970-01-01 UTC.
    pub(crate) fn now(&self) -> u64 {
        if let Some(clock) = &self.clock {
            return clock();
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
        }
        #[cfg(target_arch = "wasm32")]
        0
    }

    /// The time of the clock in microseconds since 1970-01-01 UTC: the
    /// system clock to the microsecond, or the seconds of `set_clock`.
    pub(crate) fn now_micros(&self) -> u64 {
        #[cfg(not(target_arch = "wasm32"))]
        if self.clock.is_none() {
            use std::time::{SystemTime, UNIX_EPOCH};
            let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
            return u64::try_from(d.as_micros()).unwrap_or(u64::MAX);
        }
        self.now().saturating_mul(1_000_000)
    }

    /// The number of cells on the data stack.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// The float stack, the top last.
    #[must_use]
    pub fn fstack(&self) -> &[f64] {
        &self.fstack
    }

    /// The names of the words the program defined, the latest first, each
    /// once; not those of the system or of extensions.
    #[must_use]
    pub fn program_words(&self) -> Vec<&str> {
        let mut seen = HashSet::new();
        let words = self.dict[self.fence..].iter().rev().filter(|w| !w.hidden);
        words.map(|w| w.name.as_str()).filter(|name| seen.insert(*name)).collect()
    }

    /// The data stack, the top last.
    #[must_use]
    pub fn stack(&self) -> &[Cell] {
        &self.stack
    }

    /// The output produced since the last `flush` or `take_output`, which
    /// the system then forgets; for hosts that show it themselves.
    pub fn take_output(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.out)
    }

    /// Pops a string ( c-addr u ) and returns a copy of it.
    ///
    /// # Errors
    /// Throws -4 when the stack is short, or -9 when the string is not in
    /// memory.
    pub fn pop_bytes(&mut self) -> Res<Vec<u8>> {
        let (i, n) = self.pop_str()?;
        Ok(self.mem[i..i + n].to_vec())
    }

    /// Copies `bytes` to a new block on the heap and pushes it as
    /// ( c-addr u ); the program frees it with FREE.
    ///
    /// # Errors
    /// Throws -59 when the heap is full.
    pub fn push_bytes(&mut self, bytes: &[u8]) -> Res {
        let a = self.heap.allocate(bytes.len(), self.here).ok_or(Throw::new(ALLOCATE_FAILED))?;
        self.mem[a..a + bytes.len()].copy_from_slice(bytes);
        self.push(to_cell(a));
        self.push(to_cell(bytes.len()));
        Ok(())
    }

    /// Forth memory, for the addresses that `pop_str` returns.
    #[must_use]
    pub fn memory(&self) -> &[u8] {
        &self.mem
    }

    /// Forth memory, to write in.
    pub fn memory_mut(&mut self) -> &mut [u8] {
        &mut self.mem
    }
}
