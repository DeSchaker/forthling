\ book003_example004_cli_tool.fs - a small command-line tool: show files with numbered lines.
\ Run: cargo run --release -- examples/book003_example004_cli_tool.fs
\ Real use: forthling book003_example004_cli_tool.fs -- -n notes.txt more.txt
\ Needs the OPERATING-SYSTEM words (NEXT-ARG), so it does not run in the playground.
\
\ How it works:
\   1. NEXT-ARG takes the next argument of the command line: a word, or 0 0 if there is none.
\      The arguments are what follows the script. Put `--` before them, so that the forthling
\      program does not read -n as an option of its own.
\   2. MAIN is a loop over the arguments: an option sets a flag, any other word is a file name.
\   3. Without arguments the program prints a usage text and then shows the tool at work on a
\      file it writes itself. DEFER/IS lets the same MAIN read the real command line or a
\      string that stands in for it.

ALSO OPERATING-SYSTEM

VARIABLE NUMBERING   \ -n: put a line number before each line
VARIABLE COUNTING    \ -c: only count the lines of each file

\ ---- where the arguments come from ----

DEFER ARGUMENT ( -- c-addr u )     \ the next argument; u is 0 when there is none
' NEXT-ARG IS ARGUMENT

\ A stand-in for the command line: a string of words that FAKE-ARG hands out one by one.
VARIABLE FAKE-A   VARIABLE FAKE-U
: SKIP ( c-addr u char skip? -- c-addr' u' )          \ skip? true: skip char; false: skip the rest
  {: char skip? :}
  BEGIN DUP IF OVER C@ char = skip? = ELSE 0 THEN WHILE 1 /STRING REPEAT ;
: FAKE-ARG ( -- c-addr u )
  FAKE-A @ FAKE-U @  BL -1 SKIP               ( word-a u )
  2DUP BL 0 SKIP                              ( word-a u rest-a rest-u )
  FAKE-U !  FAKE-A !  DROP                    ( word-a )
  DUP FAKE-A @ SWAP - ;                       ( word-a length )
: FAKE-LINE ( c-addr u -- )  FAKE-U ! FAKE-A !  ['] FAKE-ARG IS ARGUMENT ;

\ ---- the tool ----

: USAGE ( -- )
  ." usage: lines [-n] [-c] file ..." CR
  ."   -n  number the lines" CR
  ."   -c  only count the lines" CR ;

CREATE LBUF 256 ALLOT

: .NO ( n -- )  S>D <# #S #> DUP 3 SWAP - 0 MAX SPACES TYPE ." : " ;

: SHOW-FILE ( c-addr u -- )
  2DUP R/O OPEN-FILE IF  DROP ." lines: cannot read " TYPE CR EXIT  THEN
  {: name-a name-u fid :}
  0 {: n :}
  BEGIN  LBUF 256 fid READ-LINE THROW  WHILE
    n 1+ TO n
    COUNTING @ 0= IF
      NUMBERING @ IF n .NO THEN  LBUF SWAP TYPE CR
    ELSE DROP THEN
  REPEAT  DROP
  fid CLOSE-FILE THROW
  COUNTING @ IF name-a name-u TYPE ." : " n . CR THEN ;

: MAIN ( -- )
  0 NUMBERING !  0 COUNTING !
  BEGIN  ARGUMENT DUP WHILE
    2DUP S" -n" COMPARE 0= IF  2DROP -1 NUMBERING !  ELSE
    2DUP S" -c" COMPARE 0= IF  2DROP -1 COUNTING !   ELSE
      SHOW-FILE
    THEN THEN
  REPEAT  2DROP ;

\ ---- run ----

\ With no arguments: usage, then a demonstration on a file that the program writes itself.
: DEMO ( -- )
  S" demo.txt" W/O CREATE-FILE THROW {: fid :}
  S\" first line\nsecond line\nthird line\n" fid WRITE-FILE THROW
  fid CLOSE-FILE THROW
  USAGE CR
  ." $ lines -n demo.txt" CR   S" -n demo.txt" FAKE-LINE MAIN CR
  ." $ lines -c demo.txt" CR   S" -c demo.txt" FAKE-LINE MAIN CR
  ." $ lines missing.txt" CR   S" missing.txt" FAKE-LINE MAIN
  S" demo.txt" DELETE-FILE THROW ;

: RUN ( -- )  ARGC @ 1 = IF  DEMO  ELSE  MAIN  THEN ;
RUN
