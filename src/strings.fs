\ Dynamic strings and string arrays, in the word list DYNAMIC-STRINGS (see strings.rs).
\ A dynamic string is a cell that holds 0 (the empty string) or the address of a
\ heap block: a cell with the length, then the characters.
: $INIT ( a -- )  0 SWAP ! ;
: $FREE ( a -- )  DUP @ ?DUP IF FREE THROW THEN $INIT ;
: $@LEN ( a -- u )  @ DUP IF @ THEN ;
: $@ ( a -- c-addr u )  @ ?DUP IF DUP CELL+ SWAP @ ELSE 0 0 THEN ;
: $!LEN ( u a -- )  \ new length; characters that are new are zero
   {: n a | old :}
   n 0< IF -24 THROW THEN
   n 0= IF a $FREE EXIT THEN
   a $@LEN TO old
   a @ IF a @ n CELL+ RESIZE ELSE n CELL+ ALLOCATE THEN THROW
   DUP a !  n OVER !
   old n < IF CELL+ old + n old - ERASE ELSE DROP THEN ;
: $+!LEN ( u a -- c-addr )  \ room for u more characters; c-addr is the new space
   {: n a | old :}
   a $@LEN TO old  old n + a $!LEN  a $@ DROP old + ;
: $! ( c-addr u a -- )
   {: src n a | new :}
   n 0= IF a $FREE EXIT THEN
   n CELL+ ALLOCATE THROW TO new
   n new !  src new CELL+ n MOVE
   a $FREE  new a ! ;
\ The string to insert may lie inside the string it goes into, so it is copied first.
: (INS) ( tmp u a pos -- )
   {: tmp n a pos | len :}
   a $@LEN TO len  pos 0 MAX len MIN TO pos
   n a $+!LEN DROP
   a $@ DROP pos +  DUP n +  len pos -  MOVE
   tmp  a $@ DROP pos +  n MOVE ;
: $INS ( c-addr u a pos -- )
   {: src n a pos | tmp :}
   n 1 < IF EXIT THEN
   n ALLOCATE THROW TO tmp  src tmp n MOVE
   tmp n a pos ['] (INS) CATCH  tmp FREE THROW  THROW ;
: $+! ( c-addr u a -- )  DUP $@LEN $INS ;
: $DEL ( a pos u -- )
   {: a pos n | len :}
   a $@LEN TO len
   pos 0 MAX len MIN TO pos   n 0 MAX len pos - MIN TO n
   n 0= IF EXIT THEN
   a $@ DROP pos +  DUP n +  SWAP  len pos - n -  MOVE
   len n - a $!LEN ;
: $SPLIT ( c-addr u char -- c-addr u1 c-addr2 u2 )
   {: src n ch | i :}
   0 TO i
   BEGIN i n < IF src i + C@ ch <> ELSE FALSE THEN WHILE i 1+ TO i REPEAT
   i n < IF src i  src i + 1+  n i - 1-  ELSE src n  src n +  0 THEN ;
: $ITER ( ... a char xt -- ... )  \ xt is ( ... c-addr u -- ... ), once per part
   {: a ch xt | p len u1 :}
   a $@ TO len TO p
   len 0= IF EXIT THEN
   BEGIN
      p len ch $SPLIT 2>R  TO u1 DROP
      p u1 xt EXECUTE
      len u1 >
      2R> TO len TO p
   0= UNTIL ;

: $EXEC ( xt a -- )  \ appends what xt prints
   >R $TMP R> $+! ;
: $SLURP ( fid a -- )  >R SLURP-FID  2DUP R> $!  DROP FREE THROW ;
: $SLURP-FILE ( c-addr u a -- )  >R SLURP-FILE  2DUP R> $!  DROP FREE THROW ;
: $VARIABLE ( "name" -- )  CREATE 0 , ;
\ String arrays: a dynamic string that holds cells, each one a dynamic string.
: $[]# ( a -- n )  $@LEN 1 CELLS / ;
: $[] ( n a -- a' )  \ the n-th string; the array grows when n is past the end
   {: n a :}
   n 0< IF -24 THROW THEN
   n 1+ CELLS a $@LEN > IF n 1+ CELLS a $!LEN THEN
   a $@ DROP n CELLS + ;
: $[]! ( c-addr u n a -- )  $[] $! ;
: $[]@ ( n a -- c-addr u )  \ the empty string when n is past the end; the array stays
   OVER 1+ CELLS OVER $@LEN > IF 2DROP 0 0 EXIT THEN  $[] $@ ;
: $[]+! ( c-addr u a -- )  \ adds the string as a new last element
   DUP $[]# SWAP $[] $! ;
: $[]FREE ( a -- )
   {: a :}  a $[]# 0 ?DO I a $[] $FREE LOOP  a $FREE ;
: $[]MAP ( a xt -- )  \ xt is ( c-addr u -- ), once per element
   {: a xt :}  a $[]# 0 ?DO I a $[] $@ xt EXECUTE LOOP ;
: $[]SLURP ( fid a -- )  \ the lines of the file; an old content is freed
   {: fid a | src n p len u1 :}
   a $[]FREE
   fid SLURP-FID TO n TO src
   src TO p  n TO len
   BEGIN len 0> WHILE
      p len 10 $SPLIT 2>R  TO u1 DROP
      p u1  DUP IF 2DUP + 1- C@ 13 = IF 1- THEN THEN  a $[]+!
      2R> TO len TO p
   REPEAT
   src FREE THROW ;
: $[]SLURP-FILE ( c-addr u a -- )
   >R R/O OPEN-FILE THROW  DUP R> $[]SLURP  CLOSE-FILE THROW ;
: $[]VARIABLE ( "name" -- )  CREATE 0 , ;
