//! Timing words: the word list BENCHMARK, installed as the extension `Benchmark`.
//! A script opens it with `ALSO BENCHMARK`. The programs in `benches/*.fs` use it,
//! and so can an agent that wants to measure its own code.

use crate::*;

/// The extension: `forth.install(&Benchmark)`; ENVIRONMENT? knows it as
/// FORTHLING-BENCHMARK.
pub struct Benchmark;

impl Extension for Benchmark {
    fn name(&self) -> &'static str {
        "FORTHLING-BENCHMARK"
    }

    fn install(&self, f: &mut Forth) {
        let wid = f.wordlist("BENCHMARK");
        f.add_word(wid, "BENCH", bench).expect("BENCHMARK is a word list");
        f.add_word(wid, "BENCH-BEST", bench_best).expect("BENCHMARK is a word list");
    }
}

fn pop_xt(f: &mut Forth, word: &str) -> Res<usize> {
    let xt = f.pop()?;
    usize::try_from(xt).ok().filter(|&i| i < f.dict.len()).ok_or_else(|| {
        Throw::with(INVALID_ARGUMENT, format!("{word}: invalid execution token {xt}"))
    })
}

/// Runs xt once; the time it took, in microseconds.
fn time(f: &mut Forth, xt: usize) -> Res<u64> {
    let start = f.now_micros();
    f.execute(xt)?;
    Ok(f.now_micros().saturating_sub(start))
}

/// `BENCH ( xt -- microseconds )`: runs xt once and gives the time it took.
fn bench(f: &mut Forth) -> Res {
    let xt = pop_xt(f, "BENCH")?;
    let t = time(f, xt)?;
    f.push(to_cell(usize::try_from(t).unwrap_or(usize::MAX)));
    Ok(())
}

/// `BENCH-BEST ( n xt -- microseconds )`: runs xt n times and gives the shortest
/// time, which is the least disturbed by the machine.
fn bench_best(f: &mut Forth) -> Res {
    let xt = pop_xt(f, "BENCH-BEST")?;
    let n = f.pop()?;
    if n < 1 {
        return Err(Throw::with(INVALID_ARGUMENT, format!("BENCH-BEST: {n} runs")));
    }
    let mut best = u64::MAX;
    for _ in 0..n {
        best = best.min(time(f, xt)?);
    }
    f.push(to_cell(usize::try_from(best).unwrap_or(usize::MAX)));
    Ok(())
}
