\ The empty loop: 30 million trips of the inner interpreter; the shortest of 5 runs.
\ Run it with: forthling benches/loop.fs
ALSO BENCHMARK
: L  30000000 0 DO LOOP ;
." loop             " 5 ' L BENCH-BEST 1000 / . ." ms" CR
