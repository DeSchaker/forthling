\ book003_example002_word_count.fs - count the words of a text and print the most common ones.
\ Run: cargo run --release -- examples/book003_example002_word_count.fs
\ Needs the core only (the word lists COLLECTIONS and STRINGS); it also runs in the playground.
\
\ How it works:
\   1. LOWER changes a text in place, so the program works on an ALLOCATEd copy of it.
\   2. A word is a run of letters and apostrophes. Everything else only separates words.
\   3. A dictionary maps each word to its count: DICT@ and DICT! add 1 for every word.
\   4. DICT-EACH copies the words and the counts into vectors. SORT-CELLS then puts the
\      numbers 0..n-1 in order of count (high first), and for the same count in
\      alphabetical order. The program prints the top of that list.

ALSO COLLECTIONS ALSO STRINGS

: TEXT ( -- c-addr u )
  S" The quick brown fox jumps over the lazy dog. The dog sleeps; the fox doesn't. A quick fox is a happy fox, and a lazy dog is a lazy dog." ;

DICT-VARIABLE COUNTS
VEC-VARIABLE ADDRS   VEC-VARIABLE LENS   VEC-VARIABLE NUMS   VEC-VARIABLE ORDER
VARIABLE TOTAL

: LETTER? ( char -- flag )  DUP [CHAR] a [CHAR] z 1+ WITHIN  SWAP [CHAR] ' = OR ;

\ Add 1 to the count of a word.
: COUNT-WORD ( c-addr u -- )
  1 TOTAL +!
  2DUP COUNTS DICT@ IF 1+ ELSE 1 THEN  ROT ROT COUNTS DICT! ;

\ Walk over the text: skip what is not a letter, take a run of letters as one word.
: COUNT-WORDS ( c-addr u -- )  {: a u :}
  0 {: i :}  0 {: j :}
  BEGIN i u < WHILE
    a i + C@ LETTER? IF
      i TO j
      BEGIN  i u <  IF a i + C@ LETTER? ELSE 0 THEN  WHILE  i 1+ TO i  REPEAT
      a j +  i j -  COUNT-WORD
    ELSE  i 1+ TO i  THEN
  REPEAT ;

\ DICT-EACH calls this for each word with its count: keep all three.
: KEEP ( c-addr u n -- )
  NUMS VEC+!  LENS VEC+!  ADDRS VEC+!
  ORDER VEC# ORDER VEC+! ;                   \ the number of this entry, from 0

: WORD@ ( i -- c-addr u )  DUP ADDRS VEC@ SWAP LENS VEC@ ;
: NUM@  ( i -- n )  NUMS VEC@ ;

\ i goes before j: more often, or as often and earlier in the alphabet.
: BEFORE? ( i j -- flag )  {: i j :}
  i NUM@ j NUM@ = IF  i WORD@ j WORD@ COMPARE 0<  ELSE  i NUM@ j NUM@ >  THEN ;

: .ENTRY ( i -- )  DUP NUM@ 3 .R SPACE  WORD@ TYPE ;

: TOP ( n -- )
  ORDER VEC# MIN 0 ?DO  I ORDER VEC@ .ENTRY CR  LOOP ;

: MAIN ( -- )
  TEXT {: ta tu :}
  tu ALLOCATE THROW {: buf :}
  ta buf tu CMOVE   buf tu LOWER COUNT-WORDS   buf FREE THROW
  COUNTS ['] KEEP DICT-EACH
  ORDER VEC-DATA ['] BEFORE? SORT-CELLS
  TOTAL @ . ." words, " COUNTS DICT# . ." different. The top 5:" CR
  5 TOP ;
MAIN
