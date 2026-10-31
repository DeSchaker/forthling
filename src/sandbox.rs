//! The sandbox: limits on the steps, the time and the output of each
//! evaluation, and permissions for what a Forth program may reach outside
//! its memory. They restrict the Forth program, not the host: `include`
//! called by the host still reads its file, but `INCLUDE` in the program
//! needs `Permission::Files`.

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
}

impl Limits {
    /// The limits of `Forth::sandboxed`: 10 million steps, 5 seconds and
    /// 1 MiB of output.
    pub const SANDBOX: Limits = Limits {
        steps: Some(10_000_000),
        time: Some(Duration::from_secs(5)),
        output: Some(1 << 20),
    };
}

/// What a Forth program may do outside its memory; everything is permitted
/// unless the host says otherwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Permission {
    /// Files: the FILE words, `INCLUDE` and the like, and the block file.
    Files,
}

/// What is left of the limits in the current evaluation.
pub(crate) struct Budget {
    steps: u64,    // u64::MAX for no limit
    output: usize, // usize::MAX for no limit
    #[cfg(not(target_arch = "wasm32"))]
    deadline: Option<Instant>,
    spent: Option<&'static str>, // the limit that was reached (steps then is 0)
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
            depth: 0,
        }
    }
}

impl Budget {
    /// Marks a limit as reached; every step after it throws.
    fn spend(&mut self, what: &'static str) {
        self.spent.get_or_insert(what);
        self.steps = 0;
    }
}

/// How often the clock is read, in steps.
const CLOCK_EVERY: u64 = 4096;

fn limit_reached(what: &str) -> Throw {
    Throw::with(LIMIT_EXCEEDED, what)
}

impl Forth {
    /// A system for programs that are not trusted: no files, no terminal
    /// input (`set_input` gives it some), and the limits of
    /// `Limits::SANDBOX`.
    pub fn sandboxed() -> Self {
        let mut f = Forth::new();
        f.set_limits(Limits::SANDBOX);
        f.permit(Permission::Files, false);
        f.set_input(Box::new(io::Cursor::new(Vec::new())));
        f
    }

    /// Sets the limits of each evaluation from now on. When one is reached,
    /// the program throws -258 with `steps`, `time` or `output` as detail,
    /// and every step after that throws again, so CATCH cannot go on.
    pub fn set_limits(&mut self, limits: Limits) {
        self.limits = limits;
    }

    /// The limits of each evaluation.
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
            self.budget = Budget {
                steps: self.limits.steps.unwrap_or(u64::MAX),
                output: self.limits.output.unwrap_or(usize::MAX),
                #[cfg(not(target_arch = "wasm32"))]
                deadline: self.limits.time.map(|t| Instant::now() + t),
                spent: None,
                depth: 0,
            };
        }
        self.budget.depth += 1;
        let result = run(self);
        self.budget.depth -= 1;
        // Output past the limit is dropped; the evaluation still fails.
        match self.budget.spent {
            Some(what) if result.is_ok() => Err(limit_reached(what)),
            _ => result,
        }
    }

    /// Counts a step, and throws when a limit is reached. The fast path is
    /// one test: only every CLOCK_EVERY steps, and at 0, is there more to do.
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
        self.check_clock()
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
        std::thread::sleep(time);
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
