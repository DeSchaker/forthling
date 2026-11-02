//! Benchmarks of the interpreter, with no dependencies: `cargo bench`.
//!
//! Each case runs a few times and prints the shortest time, which is the least disturbed
//! by the machine. To compare a change with the previous commit, run this in a checkout of
//! each (a `git worktree` does) and put the two tables side by side. `cargo bench -- fib`
//! runs only the cases whose name holds `fib`; `FORTHLING_BENCH_RUNS=10` sets the runs.

use forthling::Forth;
use std::fmt::Write;
use std::time::{Duration, Instant};

const RUNS: usize = 5;

/// The empty loop: 30 million trips of the inner interpreter.
const LOOP: &str = ": L 30000000 0 DO LOOP ; L";

/// Recursive Fibonacci: calls, the return stack and comparisons.
const FIB: &str = ": FIB DUP 2 < IF EXIT THEN DUP 1- RECURSE SWAP 2 - RECURSE + ; 27 FIB DROP";

/// `n` colon definitions, as a file would have them.
fn definitions(n: usize) -> String {
    let mut src = String::new();
    for i in 0..n {
        writeln!(src, ": W{i} {i} DUP + SWAP DROP ;").unwrap();
    }
    src
}

/// `n` constants.
fn constants(n: usize) -> String {
    let mut src = String::new();
    for i in 0..n {
        writeln!(src, "{i} CONSTANT C{i}").unwrap();
    }
    src
}

/// `n` lines of numbers and a word of the system, to be found among many definitions.
fn number_lookups(n: usize) -> String {
    "1 2 + DROP\n".repeat(n)
}

/// `n` lines that name a word defined long before, the first of `constants`.
fn name_lookups(n: usize) -> String {
    "C0 DROP\n".repeat(n)
}

/// What a case does to a system that is ready, and how it is measured.
struct Case {
    name: &'static str,
    /// Run before the clock starts, on the new system.
    setup: String,
    /// Run with the clock.
    work: String,
}

fn cases() -> Vec<Case> {
    let case = |name, setup: String, work: String| Case { name, setup, work };
    vec![
        case("loop", String::new(), LOOP.into()),
        case("fib", String::new(), FIB.into()),
        case("define-5000", String::new(), definitions(5000)),
        case("define-20000", String::new(), definitions(20000)),
        case("constants-20000", String::new(), constants(20000)),
        case("lookup-numbers", constants(20000), number_lookups(20000)),
        case("lookup-names", constants(20000), name_lookups(20000)),
    ]
}

fn shortest(runs: usize, mut once: impl FnMut() -> Duration) -> Duration {
    (0..runs).map(|_| once()).min().expect("at least one run")
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn main() {
    // Cargo passes `--bench`; the other arguments, if any, select cases by name.
    let filter: Vec<String> = std::env::args().skip(1).filter(|a| !a.starts_with('-')).collect();
    let wanted = |name: &str| filter.is_empty() || filter.iter().any(|f| name.contains(f.as_str()));
    let runs = std::env::var("FORTHLING_BENCH_RUNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or(RUNS);

    println!("forthling benchmarks (shortest of {runs} runs, in ms)");
    if wanted("start") {
        // A new system: the words are installed and the prelude is compiled.
        let t = shortest(runs, || {
            let t = Instant::now();
            let mut f = Forth::new();
            f.interpret("1 DROP").expect("start");
            t.elapsed()
        });
        println!("{:<18}{:>10.1}", "start", ms(t));
    }
    for case in cases().iter().filter(|c| wanted(c.name)) {
        let t = shortest(runs, || {
            let mut f = Forth::new();
            f.interpret(&case.setup).expect("setup");
            let t = Instant::now();
            f.interpret(&case.work).expect(case.name);
            t.elapsed()
        });
        println!("{:<18}{:>10.1}", case.name, ms(t));
    }
}
