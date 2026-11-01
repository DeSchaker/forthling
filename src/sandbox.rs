//! The sandbox: limits on the steps, the time and the output of each
//! evaluation, and permissions for what a Forth program may reach outside
//! its memory. They restrict the Forth program, not the host: `include`
//! called by the host still reads its file, but `INCLUDE` in the program
//! needs `Permission::Files`. Which files there are is up to the file
//! system (vfs.rs): the disk, or for `sandboxed` files in memory.

use crate::*;
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

/// The limits of each evaluation (a call of `interpret` or `include` by
/// the host); None is no limit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Limits {
    /// Instructions and words executed.
    pub steps: Option<u64>,
    /// Running time, `MS` included. Not available in WebAssembly, where it
    /// is ignored.
    pub time: Option<Duration>,
    /// Bytes of output.
    pub output: Option<usize>,
    /// Bytes outside Forth memory that the program's own definitions, code,
    /// word lists, REPLACES texts and JSON strings take. Unlike the other
    /// limits, this one counts everything since the system was made, less
    /// what MARKER and FORGET remove.
    pub space: Option<usize>,
}

impl Limits {
    /// The limits of `Forth::sandboxed`: 10 million steps, 5 seconds, 1 MiB
    /// of output and 4 MiB of space.
    pub const SANDBOX: Limits = Limits {
        steps: Some(10_000_000),
        time: Some(Duration::from_secs(5)),
        output: Some(1 << 20),
        space: Some(4 << 20),
    };
}

/// What a Forth program may do outside its memory; everything is permitted
/// unless the host says otherwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Permission {
    /// Files: the FILE words, `INCLUDE` and the like, and the block file,
    /// in the file system the host set (`set_file_system`).
    Files,
    /// The network: the extensions that connect to other machines, such as
    /// `forthling-http`, need it. `Forth::sandboxed` denies it. Whether a
    /// program reaches a given machine is up to the extension, which keeps
    /// a list of the hosts it may connect to.
    Network,
}

/// What is left of the limits in the current evaluation.
pub(crate) struct Budget {
    steps: u64,    // u64::MAX for no limit
    output: usize, // usize::MAX for no limit
    #[cfg(not(target_arch = "wasm32"))]
    deadline: Option<Instant>,
    spent: Option<&'static str>, // the limit that was reached (steps then is 0)
    start: u64,                  // steps at the start, to tell how many were used
    pub(crate) deepest: usize,   // the deepest data stack seen after a step
    depth: usize,                // evaluations in progress, nested through host words
}

impl Default for Budget {
    fn default() -> Self {
        Budget {
            steps: u64::MAX,
            output: usize::MAX,
            #[cfg(not(target_arch = "wasm32"))]
            deadline: None,
            spent: None,
            start: u64::MAX,
            deepest: 0,
            depth: 0,
        }
    }
}

impl Budget {
    /// Marks a limit as reached; every step after it throws.
    pub(crate) fn spend(&mut self, what: &'static str) {
        self.spent.get_or_insert(what);
        self.steps = 0;
    }
}

/// Bytes of work that count as one step, for the words that handle much
/// data at once (MOVE, SEARCH, the JSON words and the like).
const BYTES_PER_STEP: usize = 64;

/// What the dictionary, a word list and a REPLACES text take outside Forth
/// memory, as counted for `Limits::space`.
pub(crate) const WORD_SPACE: usize = std::mem::size_of::<Word>();
pub(crate) const INSTR_SPACE: usize = std::mem::size_of::<Instr>();
pub(crate) const WORDLIST_SPACE: usize = 32;
pub(crate) const REPLACES_SPACE: usize = 64;

/// How often the clock is read, in steps.
const CLOCK_EVERY: u64 = 4096;

fn limit_reached(what: &str) -> Throw {
    Throw::with(LIMIT_EXCEEDED, what)
}

impl Forth {
    /// A system for programs that are not trusted: files only in memory
    /// (an empty `MemoryFs` of at most `MemoryFs::SANDBOX_LIMIT` bytes; give
    /// it files of your own with `set_file_system`), no terminal input
    /// (`set_input` gives it some), no network (`Permission::Network`), and
    /// the limits of `Limits::SANDBOX`.
    #[must_use]
    pub fn sandboxed() -> Self {
        let mut f = Forth::new();
        f.set_limits(Limits::SANDBOX);
        f.permit(Permission::Network, false);
        f.vfs = Box::new(MemoryFs::with_limit(MemoryFs::SANDBOX_LIMIT));
        f.set_input(Box::new(io::Cursor::new(Vec::new())));
        f
    }

    /// Sets the limits of each evaluation from now on. When one is reached,
    /// the program throws -258 with `steps`, `time` or `output` as detail,
    /// and every step after that throws again, so CATCH cannot go on.
    pub fn set_limits(&mut self, limits: Limits) {
        self.limits = limits;
    }

    /// The space outside Forth memory that the program's definitions, code,
    /// word lists, REPLACES texts and JSON strings take now, in bytes (see
    /// `Limits::space`); the words of the system and of extensions are not
    /// counted.
    #[must_use]
    pub fn space(&self) -> usize {
        self.space_used()
    }

    /// The steps the current or last evaluation took (see `Limits::steps`),
    /// with or without a limit.
    #[must_use]
    pub fn steps_used(&self) -> u64 {
        self.budget.start - self.budget.steps
    }

    /// The deepest the data stack was in the current or last evaluation, as
    /// seen after each step.
    #[must_use]
    pub fn max_depth(&self) -> usize {
        self.budget.deepest
    }

    /// The limits of each evaluation.
    #[must_use]
    pub fn limits(&self) -> Limits {
        self.limits
    }

    /// Permits or forbids something; what is forbidden throws -259, or gives
    /// -259 as the ior of a word that has one.
    pub fn permit(&mut self, permission: Permission, permitted: bool) {
        if permitted {
            self.denied.remove(&permission);
        } else {
            self.denied.insert(permission);
        }
    }

    /// Whether something is permitted; for extensions that reach outside.
    #[must_use]
    pub fn permits(&self, permission: Permission) -> bool {
        !self.denied.contains(&permission)
    }

    /// Throws -259 unless the permission is there; `what` names the attempt.
    pub(crate) fn need(&self, permission: Permission, what: impl FnOnce() -> String) -> Res {
        if self.permits(permission) { Ok(()) } else { Err(Throw::with(NOT_PERMITTED, what())) }
    }

    /// Runs an evaluation of the host. The outermost one starts with the
    /// full limits; one started from inside (by a host word) shares them.
    pub(crate) fn evaluation(&mut self, run: impl FnOnce(&mut Self) -> Res) -> Res {
        if self.budget.depth == 0 {
            let steps = self.limits.steps.unwrap_or(u64::MAX);
            self.budget = Budget {
                steps,
                output: self.limits.output.unwrap_or(usize::MAX),
                #[cfg(not(target_arch = "wasm32"))]
                deadline: self.limits.time.map(|t| Instant::now() + t),
                spent: None,
                start: steps,
                deepest: self.stack.len(),
                depth: 0,
            };
        }
        self.budget.depth += 1;
        let result = run(self);
        self.budget.depth -= 1;
        if self.budget.depth == 0 {
            self.end_stepping();
        }
        // Output past the limit is dropped; the evaluation still fails.
        match self.budget.spent {
            Some(what) if result.is_ok() => Err(limit_reached(what)),
            _ => result,
        }
    }

    /// Counts a step, and throws when a limit is reached. The fast path is
    /// one test: only every `CLOCK_EVERY` steps, and at 0, is there more to do.
    #[inline]
    pub(crate) fn step(&mut self) -> Res {
        if self.budget.steps % CLOCK_EVERY == 0 {
            return self.step_slowly();
        }
        self.budget.steps -= 1;
        Ok(())
    }

    #[cold]
    #[inline(never)]
    fn step_slowly(&mut self) -> Res {
        let b = &mut self.budget;
        // A limit that is reached sets steps to 0 too, so this test suffices.
        if b.steps == 0 {
            let what = *b.spent.get_or_insert("steps");
            return Err(limit_reached(what));
        }
        b.steps -= 1;
        self.check_clock()?;
        // Code being compiled grows outside Forth memory too.
        self.check_space()
    }

    /// Counts work of `bytes` bytes as steps, one per 64 bytes, and reads the
    /// clock when that passes a multiple of 4096 steps; throws -258 when a
    /// limit is reached. A word that handles much data at once calls it, so
    /// that the limits hold for it too; an extension as well.
    pub fn charge(&mut self, bytes: usize) -> Res {
        let cost = (bytes / BYTES_PER_STEP) as u64;
        if cost == 0 {
            return Ok(());
        }
        let b = &mut self.budget;
        if b.steps < cost {
            let what = *b.spent.get_or_insert("steps");
            b.steps = 0;
            return Err(limit_reached(what));
        }
        let before = b.steps;
        b.steps -= cost;
        if before / CLOCK_EVERY != b.steps / CLOCK_EVERY {
            self.check_clock()?;
        }
        Ok(())
    }

    /// The space outside Forth memory that the program takes now (see
    /// `Limits::space`).
    pub(crate) fn space_used(&self) -> usize {
        let total = self.space
            + self.code.len() * INSTR_SPACE
            + self.wordlists.len() * WORDLIST_SPACE
            + self.json_space();
        total.saturating_sub(self.system_space)
    }

    /// Throws -258 with `space` when the program takes more than its limit.
    pub(crate) fn check_space(&mut self) -> Res {
        match self.limits.space {
            Some(limit) if self.space_used() > limit => {
                self.budget.spend("space");
                Err(limit_reached("space"))
            }
            _ => Ok(()),
        }
    }

    /// Throws -258 when the time of the evaluation is up; for an extension
    /// that has just done something that took long.
    pub fn check_time(&mut self) -> Res {
        self.check_clock()
    }

    /// When the time of the current evaluation is up, if it has a limit; for
    /// an extension that runs something long outside Forth (such as SQL), so
    /// that it can stop in time. Not available in WebAssembly.
    #[cfg(not(target_arch = "wasm32"))]
    #[must_use]
    pub fn deadline(&self) -> Option<Instant> {
        self.budget.deadline
    }

    /// Moves the time limit of the evaluation on by `time`, the time that
    /// a pause of the debugger took.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn extend_deadline(&mut self, time: Duration) {
        if let Some(deadline) = &mut self.budget.deadline {
            *deadline += time;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn check_clock(&mut self) -> Res {
        match self.budget.deadline {
            Some(deadline) if Instant::now() >= deadline => {
                self.budget.spend("time");
                Err(limit_reached("time"))
            }
            _ => Ok(()),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn check_clock(&mut self) -> Res {
        Ok(())
    }

    /// MS: sleeps, unless that would pass the time limit.
    pub(crate) fn sleep(&mut self, time: Duration) -> Res {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(deadline) = self.budget.deadline {
            if Instant::now() + time > deadline {
                self.budget.spend("time");
                return Err(limit_reached("time"));
            }
        }
        // WebAssembly cannot sleep: there MS does nothing.
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::sleep(time);
        #[cfg(target_arch = "wasm32")]
        let _ = time;
        Ok(())
    }

    /// Appends to the output, as far as the output limit allows.
    pub(crate) fn emit_bytes(&mut self, bytes: &[u8]) {
        let n = self.output_room(bytes.len());
        self.out.extend_from_slice(&bytes[..n]);
    }

    /// Appends n bytes of Forth memory from a to the output (TYPE).
    pub(crate) fn emit_memory(&mut self, a: usize, n: usize) {
        let n = self.output_room(n);
        self.out.extend_from_slice(&self.mem[a..a + n]);
    }

    /// How many of n bytes fit in the output limit; uses them up.
    fn output_room(&mut self, n: usize) -> usize {
        let b = &mut self.budget;
        if n > b.output {
            let room = b.output;
            b.output = 0;
            b.spend("output");
            room
        } else {
            b.output -= n;
            n
        }
    }
}
