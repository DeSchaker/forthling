\ Many definitions, and many lookups among them. The source text is made first, so
\ that only compiling and finding words is timed; each case runs once.
\ Run it with: forthling benches/definitions.fs
ALSO BENCHMARK
ALSO DYNAMIC-STRINGS

5000 CONSTANT N

\ The text of N lines of Forth, each one a case.
: DEFINITIONS-TEXT N 0 DO ." : W" I 0 .R ."  " I 0 .R ."  DUP + SWAP DROP ;" CR LOOP ;
: CONSTANTS-TEXT   N 0 DO I 0 .R ."  CONSTANT C" I 0 .R CR LOOP ;
: NUMBERS-TEXT     N 0 DO ." 1 2 + DROP" CR LOOP ;   \ numbers and a word of the system
: NAMES-TEXT       N 0 DO ." C0 DROP" CR LOOP ;      \ a word defined long before

\ An allocated copy of the text that xt types.
: KEEP ( xt -- c-addr u )  $TMP DUP ALLOCATE THROW SWAP 2DUP 2>R CMOVE 2R> ;

2VARIABLE TEXT
: MAKE ( xt -- )  KEEP TEXT 2! ;
: COMPILE-TEXT  TEXT 2@ EVALUATE ;
: RUN-CASE ( xt -- )  \ makes the text of xt, and times its compiling
   MAKE  ['] COMPILE-TEXT BENCH 1000 /  . ." ms" CR  TEXT 2@ DROP FREE THROW ;

." define-5000     "  ' DEFINITIONS-TEXT RUN-CASE
." constants-5000   " ' CONSTANTS-TEXT   RUN-CASE
." lookup-numbers   " ' NUMBERS-TEXT     RUN-CASE
." lookup-names     " ' NAMES-TEXT       RUN-CASE
