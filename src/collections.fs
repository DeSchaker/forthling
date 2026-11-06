\ Vectors and dictionaries, in the word list COLLECTIONS (see collections.rs).
\ Everything lives in ALLOCATEd blocks, so it counts towards the memory limit.
\
\ A vector is a cell that holds 0 or the address of a header: count, capacity and the
\ address of the block of cells with the elements.
: (VEC) ( v -- h )  \ the header; made when it is not there yet
   DUP @ ?DUP IF NIP EXIT THEN
   {: v | h d :}
   4 CELLS ALLOCATE THROW TO d
   3 CELLS ALLOCATE IF d FREE DROP -59 THROW THEN TO h
   0 h !  4 h CELL+ !  d h 2 CELLS + !  h v !  h ;
: VEC# ( v -- n )  @ DUP IF @ THEN ;
: VEC-DATA ( v -- addr n )  \ the elements as a block of cells; do not keep it past the next VEC+!
   DUP @ 0= IF 0 EXIT THEN
   @ DUP 2 CELLS + @ SWAP @ ;
: (VEC-GROW) ( h -- )  \ room for one more element
   {: h :}
   h @ h CELL+ @ < IF EXIT THEN
   h CELL+ @ 2*  h 2 CELLS + @ OVER CELLS RESIZE THROW
   h 2 CELLS + !  h CELL+ ! ;
: VEC+! ( x v -- )  \ adds x as the new last element
   (VEC) {: x h :}
   h (VEC-GROW)
   x  h 2 CELLS + @  h @ CELLS +  !
   1 h +! ;
: (VEC-AT) ( i v -- addr )  \ the address of element i; -24 when there is no such element
   {: i v :}
   i 0< i v VEC# 1- > OR IF -24 THROW THEN
   v @ 2 CELLS + @ i CELLS + ;
: VEC@ ( i v -- x )  (VEC-AT) @ ;
: VEC! ( x i v -- )  (VEC-AT) ! ;
: VEC-POP ( v -- x )  \ removes and gives the last element; -24 when there is none
   {: v :}
   v VEC# 1- DUP 0< IF -24 THROW THEN
   DUP v (VEC-AT) @ SWAP  v @ ! ;
: VEC-CLEAR ( v -- )  @ ?DUP IF 0 SWAP ! THEN ;
: VEC-FREE ( v -- )
   DUP @ ?DUP IF  DUP 2 CELLS + @ FREE THROW  FREE THROW  THEN
   0 SWAP ! ;
: VEC-SORT ( v xt -- )  \ xt is ( a b -- flag ), true when a goes before b
   >R VEC-DATA R> SORT-CELLS ;
: VEC-VARIABLE ( "name" -- )  CREATE 0 , ;

\ A dictionary maps text keys to cells and keeps the keys in the order they were added.
\ It is a cell that holds 0 or the address of a header of five cells: the number of
\ keys, the entries used (deleted ones included), the capacity in entries, the address
\ of the entries and the address of the index. An entry is three cells: the address of
\ a copy of the key (0 once deleted), its length, and the value. The index is a table
\ of 2 * capacity cells that hold an entry number plus one (0 for a free place).
: (HASH) ( c-addr u -- x )  \ FNV-1a
   -3750763034362895579 ROT ROT  OVER + SWAP
   ?DO  I C@ XOR 1099511628211 *  LOOP ;
: (DICT) ( d -- h )
   DUP @ ?DUP IF NIP EXIT THEN
   {: d | h e ni :}
   12 CELLS ALLOCATE THROW TO e
   8 CELLS ALLOCATE IF e FREE DROP -59 THROW THEN TO ni
   5 CELLS ALLOCATE IF e FREE DROP ni FREE DROP -59 THROW THEN TO h
   ni 8 CELLS ERASE
   0 h !  0 h CELL+ !  4 h 2 CELLS + !  e h 3 CELLS + !  ni h 4 CELLS + !
   h d !  h ;
: (KEY=) ( c-addr u e -- flag )  \ does the entry have this key?
   DUP @ 0= IF DROP 2DROP FALSE EXIT THEN
   DUP @ SWAP CELL+ @ COMPARE 0= ;
: (PROBE) ( c-addr u h -- ipos e )  \ the index place of the key, and its entry (0: not there)
   {: s n h | mask i :}
   h 2 CELLS + @ 2* 1- TO mask
   s n (HASH) mask AND TO i
   BEGIN
      h 4 CELLS + @ i CELLS +
      DUP @ 0= IF 0 EXIT THEN
      DUP @ 1- 3 * CELLS h 3 CELLS + @ +
      DUP s n ROT (KEY=) IF EXIT THEN
      2DROP  i 1+ mask AND TO i
   AGAIN ;
: (IX-ADD) ( ni mask x k -- )  \ puts entry k in the first free place from hash x
   {: ni mask x k :}
   x mask AND
   BEGIN DUP CELLS ni + @ WHILE 1+ mask AND REPEAT
   CELLS ni + k 1+ SWAP ! ;
: (REHASH) ( h nc -- )  \ new blocks for nc entries; the deleted entries are dropped
   {: h nc | ne ni old k :}
   nc 3 * CELLS ALLOCATE THROW TO ne
   nc 2* CELLS ALLOCATE IF ne FREE DROP -59 THROW THEN TO ni
   ni nc 2* CELLS ERASE
   0 TO k
   h CELL+ @ 0 ?DO
      h 3 CELLS + @ I 3 * CELLS + TO old
      old @ IF
         old ne k 3 * CELLS + 3 CELLS MOVE
         ni  nc 2* 1-  old @ old CELL+ @ (HASH)  k  (IX-ADD)
         k 1+ TO k
      THEN
   LOOP
   h 3 CELLS + @ FREE THROW  h 4 CELLS + @ FREE THROW
   ne h 3 CELLS + !  ni h 4 CELLS + !  nc h 2 CELLS + !  k h CELL+ ! ;
: DICT-VARIABLE ( "name" -- )  CREATE 0 , ;
: DICT# ( d -- n )  @ DUP IF @ THEN ;
: DICT! ( x c-addr u d -- )  \ the key is copied; an old value of the key is replaced
   (DICT) {: x s n h | ipos e k key :}
   s n h (PROBE) TO e TO ipos
   e IF x e 2 CELLS + ! EXIT THEN
   h CELL+ @ h 2 CELLS + @ = IF
      h  h @ 2* h 2 CELLS + @ < IF h 2 CELLS + @ ELSE h 2 CELLS + @ 2* THEN  (REHASH)
      s n h (PROBE) DROP TO ipos
   THEN
   n 1 MAX ALLOCATE THROW TO key
   s key n MOVE
   h CELL+ @ TO k
   h 3 CELLS + @ k 3 * CELLS + TO e
   key e !  n e CELL+ !  x e 2 CELLS + !
   k 1+ ipos !
   1 h CELL+ +!  1 h +! ;
: DICT@ ( c-addr u d -- x true | false )
   DUP @ 0= IF DROP 2DROP FALSE EXIT THEN
   @ (PROBE) NIP ?DUP IF 2 CELLS + @ TRUE ELSE FALSE THEN ;
: DICT-DELETE ( c-addr u d -- flag )  \ true when the key was there
   DUP @ 0= IF DROP 2DROP FALSE EXIT THEN
   @ {: s n h | e :}
   s n h (PROBE) NIP TO e
   e 0= IF FALSE EXIT THEN
   e @ FREE THROW  0 e !  -1 h +!  TRUE ;
: DICT-EACH ( d xt -- )  \ xt is ( c-addr u x -- ), once per key in the order added; xt must not add or delete keys
   {: d xt | h i e :}
   0 TO i
   BEGIN
      d @ TO h
      h IF i h CELL+ @ < ELSE FALSE THEN
   WHILE
      h 3 CELLS + @ i 3 * CELLS + TO e
      e @ IF  e @ e CELL+ @ e 2 CELLS + @ xt EXECUTE  THEN
      i 1+ TO i
   REPEAT ;
: DICT-FREE ( d -- )
   {: d | h :}
   d @ TO h
   h 0= IF EXIT THEN
   h CELL+ @ 0 ?DO  h 3 CELLS + @ I 3 * CELLS + @ ?DUP IF FREE THROW THEN  LOOP
   h 3 CELLS + @ FREE THROW  h 4 CELLS + @ FREE THROW  h FREE THROW
   0 d ! ;
