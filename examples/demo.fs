\ demo.fs - a few classic Forth examples.   Run: cargo run -- examples/demo.fs

: SQUARE  ( n -- n*n )  DUP * ;
." 7 squared = " 7 SQUARE . CR

: FAC  ( n -- n! )  DUP 1 > IF DUP 1- RECURSE * THEN ;
." 20! = " 20 FAC . CR

: FIB  ( n -- fib )  DUP 2 < IF EXIT THEN  DUP 1- RECURSE  SWAP 2 - RECURSE + ;
: FIBS  ( n -- )  0 ?DO I FIB . LOOP ;
." Fibonacci: " 15 FIBS CR

: FIZZ?  ( n -- f )  3 MOD 0= DUP IF ." Fizz" THEN ;
: BUZZ?  ( n -- f )  5 MOD 0= DUP IF ." Buzz" THEN ;
: FIZZBUZZ  ( n -- )  1+ 1 DO  I FIZZ? I BUZZ? OR  IF SPACE ELSE I . THEN  LOOP ;
15 FIZZBUZZ CR

: STARS  ( n -- )  0 ?DO [CHAR] * EMIT LOOP ;
: TRIANGLE  ( n -- )  1+ 1 DO I STARS CR LOOP ;
5 TRIANGLE

\ CREATE ... DOES> : a defining word for counters
: COUNTER  CREATE 0 , DOES> 1 OVER +! @ ;
COUNTER TICK
." ticks: " TICK . TICK . TICK . CR

." 255 in hex: " 255 HEX . DECIMAL CR
