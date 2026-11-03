\ Recursive Fibonacci: calls, the return stack and comparisons; the shortest of 5 runs.
\ Run it with: forthling benches/fib.fs
ALSO BENCHMARK
: FIB  DUP 2 < IF EXIT THEN  DUP 1- RECURSE  SWAP 2 - RECURSE + ;
: RUN  27 FIB DROP ;
." fib              " 5 ' RUN BENCH-BEST 1000 / . ." ms" CR
