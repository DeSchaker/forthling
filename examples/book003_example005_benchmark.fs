\ book003_example005_benchmark.fs - time two ways to count primes with BENCH-BEST.
\ Run: cargo run --release -- examples/book003_example005_benchmark.fs
\ Needs the BENCHMARK words, which need no permission; they also work in the playground.
\
\ How it works:
\   1. Two words count the primes up to LIMIT: trial division and the sieve of Eratosthenes.
\   2. BENCH-BEST runs a word several times and gives the shortest time in microseconds.
\      The word must have the effect ( -- ), so each one leaves its answer in a variable.
\   3. A time differs from run to run, so it cannot be in the .out file. The program prints
\      what is always the same: the answers, and whether the sieve was clearly faster.
\      Set SHOW-TIMES to true to see the times as well.

ALSO BENCHMARK

5000 CONSTANT LIMIT
FALSE VALUE SHOW-TIMES
VARIABLE FOUND

\ ---- way 1: trial division ----

: PRIME? ( n -- flag )                  \ divide by 2, 3, 4, ... until d*d is more than n
  DUP 2 < IF DROP FALSE EXIT THEN
  2 BEGIN                               ( n d )
    2DUP DUP * < IF 2DROP TRUE EXIT THEN
    2DUP MOD 0= IF 2DROP FALSE EXIT THEN
    1+
  AGAIN ;
: TRIAL ( -- )
  0  LIMIT 1+ 2 DO  I PRIME? IF 1+ THEN  LOOP  FOUND ! ;

\ ---- way 2: the sieve ----

CREATE FLAGS  LIMIT 1+ ALLOT            \ one byte per number: 1 while it may be a prime
: SIEVE ( -- )
  FLAGS LIMIT 1+ 1 FILL
  0 FLAGS C!  0 FLAGS 1+ C!
  LIMIT 1+ 2 DO
    I I * LIMIT > IF LEAVE THEN
    FLAGS I + C@ IF                     \ I is a prime: cross out its multiples
      LIMIT 1+ I I * DO  0 FLAGS I + C!  J +LOOP
    THEN
  LOOP
  0  LIMIT 1+ 0 DO  FLAGS I + C@ +  LOOP  FOUND ! ;

\ ---- the measurement ----

: TIME ( xt -- microseconds )  3 SWAP BENCH-BEST ;      \ the shortest of 3 runs

VARIABLE TRIAL-US   VARIABLE SIEVE-US

: REPORT ( c-addr u -- )                \ the name of a way, and what it found
  TYPE ."  found " FOUND @ . ." primes up to " LIMIT . CR ;

: COMPARE-WAYS ( -- )
  ['] TRIAL TIME TRIAL-US !    S" trial division:" REPORT
  ['] SIEVE TIME SIEVE-US !    S" the sieve:      " REPORT
  SHOW-TIMES IF
    ." trial division " TRIAL-US @ . ." us, the sieve " SIEVE-US @ . ." us" CR
  THEN
  ." the sieve was at least 3 times faster: "
  TRIAL-US @  SIEVE-US @ 3 *  >  IF ." yes" ELSE ." no" THEN CR ;

COMPARE-WAYS
