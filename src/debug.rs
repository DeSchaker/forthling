//! The debugger: BREAK (in the word list DEBUG) stops the program and hands
//! it to a function of the host, which looks at the state and says how to
//! go on: to the next word (step), to the next word at the same depth
//! (over), out of the current word (out), or on to the next BREAK
//! (continue). The program waits while that function runs, so a host that
//! asks a person, such as the forthling program, gives a debugger in the
//! terminal; a host that answers at once, such as the MCP server, records
//! what the program looked like at each BREAK.

use crate::*;

/// Why the program stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    /// BREAK ran.
    Break,
    /// The next word after a step, an over or an out.
    Step,
}

/// How the program goes on after a pause.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resume {
    /// Run on, to the next BREAK.
    Continue,
    /// Stop at the next word, also one that the current word calls.
    Step,
    /// Stop at the next word that is nested no deeper than this one.
    Over,
    /// Stop at the next word that is nested less deep than this one: once
    /// the word that this one is in has finished.
    Out,
}

/// What the program looked like when it stopped.
#[derive(Clone, Debug, PartialEq)]
pub struct Pause {
    /// Why it stopped.
    pub reason: Reason,
    /// The word about to run, or BREAK.
    pub word: String,
    /// How deep that word is nested, as in a trace: 0 for a word of the
    /// input, 1 for a word that one of those calls, and so on.
    pub depth: usize,
    /// The stacks, the words that are running, and where in the input; as
    /// for an error.
    pub context: ErrorContext,
}

/// The function of the host that a pause calls (see `Forth::set_debugger`).
pub type DebugHook = dyn FnMut(&mut Forth, &Pause) -> Res<Resume>;

/// Where to stop next.
#[derive(Clone, Copy)]
pub(crate) enum Stop {
    Any,           // at the next word
    AtMost(usize), // at the next word nested this deep or less
    Below(usize),  // at the next word nested less deep than this
}

#[derive(Default)]
pub(crate) struct Debugging {
    hook: Option<Box<DebugHook>>,
    pub(crate) stop: Option<Stop>,
}

impl Forth {
    /// Sets the function that a pause calls, or none. Without one, BREAK
    /// does nothing. The function gets the system, to read its stacks and
    /// memory, and what the pause looks like; it returns how to go on, or
    /// an error, which the program then throws (as -28, user interrupt, for
    /// a person who quits). Time spent in it does not count towards the time
    /// limit.
    pub fn set_debugger(&mut self, hook: Option<Box<DebugHook>>) {
        self.debug.hook = hook;
        self.debug.stop = None;
        self.update_watching();
    }

    /// Stops at `word`, if a step, an over or an out asked for it.
    pub(crate) fn step_into(&mut self, word: String) -> Res {
        let depth = self.frames.len();
        let reached = match self.debug.stop {
            None => false,
            Some(Stop::Any) => true,
            Some(Stop::AtMost(d)) => depth <= d,
            Some(Stop::Below(d)) => depth < d,
        };
        if reached { self.pause(Reason::Step, word) } else { Ok(()) }
    }

    /// Hands the program to the host's function, and sets up where to stop
    /// next from its answer.
    pub(crate) fn pause(&mut self, reason: Reason, word: String) -> Res {
        let Some(mut hook) = self.debug.hook.take() else {
            return Ok(());
        };
        let depth = self.frames.len();
        let pause = Pause { reason, word, depth, context: self.context(self.first_line) };
        #[cfg(not(target_arch = "wasm32"))]
        let started = std::time::Instant::now();
        let answer = hook(self, &pause);
        #[cfg(not(target_arch = "wasm32"))]
        self.extend_deadline(started.elapsed());
        // The function may have set another one.
        if self.debug.hook.is_none() {
            self.debug.hook = Some(hook);
        }
        self.debug.stop = match answer {
            Ok(Resume::Continue) | Err(_) => None,
            Ok(Resume::Step) => Some(Stop::Any),
            Ok(Resume::Over) => Some(Stop::AtMost(depth)),
            Ok(Resume::Out) => Some(Stop::Below(depth)),
        };
        self.update_watching();
        answer.map(|_| ())
    }

    /// The evaluation is over: nothing is left to step to.
    pub(crate) fn end_stepping(&mut self) {
        if self.debug.stop.take().is_some() {
            self.update_watching();
        }
    }
}
