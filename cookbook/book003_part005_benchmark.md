# A benchmark: two ways to count primes

*Book 3, Data and tools. Needs the core only (the word list `BENCHMARK`, which needs no
permission). It also runs in the playground.*

The program: [`examples/book003_example005_benchmark.fs`](../examples/book003_example005_benchmark.fs).
Its output is beside it, in `book003_example005_benchmark.out`. Run it with:

```
cargo run --release -- examples/book003_example005_benchmark.fs
```

It counts the primes up to 5000 in two ways, trial division and the sieve of Eratosthenes,
times both, and says whether the sieve was clearly faster. It takes under 4.8 million steps
(found by lowering `--max-steps` until it stopped), because each way runs three times, for the
three timed runs. That is inside the 10 million of the playground. In the playground, pick
*Benchmark* under *Examples*.

```
trial division: found 669 primes up to 5000
the sieve:       found 669 primes up to 5000
the sieve was at least 3 times faster: yes
```

## The two ways

`PRIME?` divides a number by 2, 3, 4, and so on, and stops when the divisor squared is more
than the number: a number with no divisor up to its square root is a prime. `TRIAL` asks it
for every number from 2 to the limit and counts the yeses.

`SIEVE` keeps one byte per number, all 1 at first. For each number that is still 1 it
crosses out its multiples, starting at its square. The outer loop can stop when the square
of the number is more than the limit, and the inner loop is a `DO ... J +LOOP`, where `J` is
the number of the outer loop and so the step. At the end the bytes that are still 1 are the
primes, and a loop adds them up.

Both leave the answer in the variable `FOUND`. That is not for show: `BENCH` runs a word
with the effect `( -- )`, so a word that has a result must put it somewhere.

## Timing with BENCH-BEST

`ALSO BENCHMARK` gives two words:

- `BENCH ( xt -- microseconds )` runs a word once and gives the time it took.
- `BENCH-BEST ( n xt -- microseconds )` runs it `n` times and gives the shortest time.

The shortest of a few runs is the best measure, since a run can only be slowed down by what
else the machine is doing, never sped up. The program uses three runs:

```forth
: TIME ( xt -- microseconds )  3 SWAP BENCH-BEST ;

['] TRIAL TIME TRIAL-US !
['] SIEVE TIME SIEVE-US !
```

`[']` is used inside a definition and `'` outside. The time is wall-clock time, so a `MS`
inside the word would count.

## A test that does not depend on the clock

A time differs from run to run, so it cannot be in the `.out` file, and the test that
compares the output with that file would fail. The program therefore prints what is always
the same: the two answers, which must agree, and one yes or no:

```forth
TRIAL-US @  SIEVE-US @ 3 *  >  IF ." yes" ELSE ." no" THEN
```

The sieve is much faster than three times, so the answer is yes on a slow or a busy machine
too. If you write a test like this, choose a margin that is far below what you measured.

To see the times, change the line `FALSE VALUE SHOW-TIMES` to `TRUE VALUE SHOW-TIMES`. On the
machine this chapter was written on (a release build, three runs each, run three times) the
output was:

```
trial division 13128 us, the sieve 795 us
trial division 13371 us, the sieve 803 us
trial division 13685 us, the sieve 804 us
```

The sieve was about 16 times faster. These numbers belong to that machine; yours will differ.
In the playground a browser rounds the clock (how much depends on the browser), so time a
word that runs for milliseconds, as here, or use `BENCH-BEST` with more runs.

## Try it

- Raise `LIMIT` to 50000 and see how the gap grows: the sieve does a little work per number,
  trial division does more for each larger number. It needs more than the default limits, so
  run it as `forthling --max-steps 1000000000 --max-time 60s file.fs`. On the machine above
  the times were 305244 us and 9145 us, a factor of 33.
- Test only odd numbers in `TRIAL` (and count 2 by hand). How much of the gap does that close?
- Time `PRIME?` for a single large number with `BENCH-BEST`, putting the number in a
  `VARIABLE` and wrapping the call in a word with the effect `( -- )`.
- Use `benches/fib.fs` as a model to time your own words from the command line:
  `forthling benches/fib.fs`.
