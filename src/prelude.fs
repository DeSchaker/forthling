\ Words that are easier to express in Forth itself; Forth::new loads them.
-1 CONSTANT TRUE   0 CONSTANT FALSE   32 CONSTANT BL
: NIP     SWAP DROP ;
: TUCK    SWAP OVER ;
: -ROT    ROT ROT ;
: ?DUP    DUP IF DUP THEN ;
: 2DUP    OVER OVER ;
: 2DROP   DROP DROP ;
: 2SWAP   ROT >R ROT R> ;
: 2OVER   3 PICK 3 PICK ;
\ Division is floored (see README).
: */MOD   >R M* R> FM/MOD ;
: */      */MOD NIP ;
: /MOD    >R S>D R> FM/MOD ;
: /       /MOD NIP ;
: MOD     /MOD DROP ;
: 1+  1 + ;   : 1-  1 - ;   : 2*  2 * ;
: 0=  0 = ;   : 0<  0 < ;   : 0>  0 > ;   : <>  = 0= ;
: NEGATE  0 SWAP - ;
: ABS     DUP 0< IF NEGATE THEN ;
: MIN     2DUP > IF SWAP THEN DROP ;
: MAX     2DUP < IF SWAP THEN DROP ;
: +!      DUP @ ROT + SWAP ! ;
: CELLS  8 * ;   : CELL+  8 + ;   : CHARS ;   : CHAR+  1+ ;
: ALIGNED  7 + -8 AND ;
: ALIGN    HERE ALIGNED HERE - ALLOT ;
: VARIABLE  CREATE 0 , ;
: CR      10 EMIT ;
: SPACE   BL EMIT ;
: SPACES  BEGIN DUP 0> WHILE SPACE 1- REPEAT DROP ;
: DECIMAL 10 BASE ! ;
: HEX     16 BASE ! ;
: #S      BEGIN # 2DUP OR 0= UNTIL ;
: SIGN    0< IF [CHAR] - HOLD THEN ;
: HOLDS   BEGIN DUP WHILE 1- 2DUP + C@ HOLD REPEAT 2DROP ;
: U.R     >R 0 <# #S #> R> OVER - SPACES TYPE ;
: .R      >R DUP ABS 0 <# #S ROT SIGN #> R> OVER - SPACES TYPE ;
: U.      0 U.R SPACE ;
: .       0 .R SPACE ;
: 0<>     0= 0= ;
: U>      SWAP U< ;
: WITHIN  OVER - >R - R> U< ;
: ERASE   0 FILL ;
: BUFFER: CREATE ALLOT ;
: 2>R     SWAP >R >R ;
: 2R>     R> R> SWAP ;
: 2R@     R> R@ OVER >R SWAP ;
: ?       @ . ;
: /STRING ROT OVER + ROT ROT - ;
: BLANK   BL FILL ;
: -TRAILING  BEGIN DUP WHILE 2DUP + 1- C@ BL <> IF EXIT THEN 1- REPEAT ;
: 2CONSTANT  CREATE , , DOES> 2@ ;
: 2VARIABLE  CREATE 2 CELLS ALLOT ;
: 2LITERAL   SWAP POSTPONE LITERAL POSTPONE LITERAL ; IMMEDIATE
: 2ROT    2>R 2SWAP 2R> 2SWAP ;
: D.R     >R DUP >R DABS <# #S R> SIGN #> R> OVER - SPACES TYPE ;
: D.      0 D.R SPACE ;
\ Structures (FACILITY EXT). A structure name gives its size; a field adds its offset.
: BEGIN-STRUCTURE  CREATE HERE 0 , 0 DOES> @ ;
: END-STRUCTURE    SWAP ! ;
: +FIELD   CREATE OVER , + DOES> @ + ;
: FIELD:   ALIGNED 1 CELLS +FIELD ;
: CFIELD:  1 CHARS +FIELD ;
\ Keyboard events. The terminal delivers characters only, so EKEY is XKEY and
\ the special keys, numbered above the Unicode range, never occur. Only the
\ ASCII keys are chars; all keys are xchars (XCHAR EXT).
: EKEY   XKEY ;
: EKEY?  KEY? ;
: XKEY?  KEY? ;
: EMIT?  TRUE ;
: EKEY>CHAR   DUP $80 U< ;
: EKEY>XCHAR  DUP $110000 U< ;
: EKEY>FKEY  DUP $110000 U< 0= ;
$110000 CONSTANT K-UP      $110001 CONSTANT K-DOWN    $110002 CONSTANT K-RIGHT
$110003 CONSTANT K-LEFT    $110004 CONSTANT K-HOME    $110005 CONSTANT K-END
$110006 CONSTANT K-PRIOR   $110007 CONSTANT K-NEXT    $110008 CONSTANT K-INSERT
$110009 CONSTANT K-DELETE
$110011 CONSTANT K-F1   $110012 CONSTANT K-F2   $110013 CONSTANT K-F3   $110014 CONSTANT K-F4
$110015 CONSTANT K-F5   $110016 CONSTANT K-F6   $110017 CONSTANT K-F7   $110018 CONSTANT K-F8
$110019 CONSTANT K-F9   $11001A CONSTANT K-F10  $11001B CONSTANT K-F11  $11001C CONSTANT K-F12
$1000000 CONSTANT K-SHIFT-MASK   $2000000 CONSTANT K-CTRL-MASK   $4000000 CONSTANT K-ALT-MASK
\ FILE: the access methods, as R_O and W_O in files.rs. BIN changes nothing.
0 CONSTANT R/O   1 CONSTANT W/O   2 CONSTANT R/W
: BIN  4 OR ;
\ BLOCK EXT
: THRU    1+ SWAP ?DO I LOAD LOOP ;
\ FLOATING: the sizes and alignment of floats. A float and a DFLOAT are
\ 8 bytes, aligned like a cell; an SFLOAT is 4.
: FLOATS     8 * ;
: FLOAT+     8 + ;
: FALIGNED   7 + -8 AND ;
: FALIGN     ALIGN ;
: SFLOATS    4 * ;
: SFLOAT+    4 + ;
: SFALIGNED  3 + -4 AND ;
: SFALIGN    HERE SFALIGNED HERE - ALLOT ;
: DFLOATS    FLOATS ;
: DFLOAT+    FLOAT+ ;
: DFALIGNED  FALIGNED ;
: DFALIGN    FALIGN ;
: DF@  F@ ;
: DF!  F! ;
: FVARIABLE  FALIGN CREATE 1 FLOATS ALLOT ;
: FCONSTANT  FALIGN CREATE HERE 1 FLOATS ALLOT F! DOES> F@ ;
: FFIELD:    FALIGNED 1 FLOATS +FIELD ;
: DFFIELD:   DFALIGNED 1 DFLOATS +FIELD ;
: SFFIELD:   SFALIGNED 1 SFLOATS +FIELD ;
\ Words that Gforth code expects (the standard and the manual describe them).
\ Stack and comparison.
: 2NIP    ROT DROP ROT DROP ;
: 2TUCK   2SWAP 2OVER ;
: THIRD   2 PICK ;
: FOURTH  3 PICK ;
: UNDER+  ROT + SWAP ;
: <=      > 0= ;   : >=   < 0= ;   : 0<=  0> 0= ;   : 0>=  0< 0= ;
: U<=     U> 0= ;  : U>=  U< 0= ;
: D<>     D= 0= ;  : D>   2SWAP D< ;  : D0<>  D0= 0= ;
: UMIN    2DUP U> IF SWAP THEN DROP ;
: UMAX    2DUP U< IF SWAP THEN DROP ;
: U/MOD   0 SWAP UM/MOD ;
: UMOD    U/MOD DROP ;
\ Memory and strings.
: BOUNDS  OVER + SWAP ;
: CELL-   8 - ;
: PLACE   2DUP C! 1+ SWAP MOVE ;
: STR=    COMPARE 0= ;
: STR<    COMPARE 0< ;
: STRING-PREFIX?  ( c-addr1 u1 c-addr2 u2 -- flag )
   ROT OVER U< IF 2DROP DROP FALSE EXIT THEN  TUCK COMPARE 0= ;
: STRING-SUFFIX?  {: a1 u1 a2 u2 :}
   u2 u1 U> IF FALSE EXIT THEN  a1 u1 + u2 -  u2  a2 u2 COMPARE 0= ;
: SAFE/STRING  0 MAX OVER MIN /STRING ;
: TOUPPER  DUP [CHAR] a 123 WITHIN IF 32 - THEN ;
: INSERT  {: a1 u1 a2 u2 :}  a2 a2 u1 + u2 MOVE  a1 a2 u1 MOVE ;
: DELETE  {: a u n :}  n 0 MAX u MIN TO n  a n + a u n - MOVE  a u + n -  n BLANK ;
\ S+ gives a new allocated string; APPEND resizes an allocated one.
: S+      {: a1 u1 a2 u2 :}
   u1 u2 + 1 MAX ALLOCATE THROW  a1 OVER u1 MOVE  a2 OVER u1 + u2 MOVE  u1 u2 + ;
: APPEND  {: a1 u1 a2 u2 :}
   a1 u1 u2 + 1 MAX RESIZE THROW  a2 OVER u1 + u2 MOVE  u1 u2 + ;
\ Small helpers.
: ON      TRUE SWAP ! ;
: OFF     FALSE SWAP ! ;
: NOOP ;
: PERFORM @ EXECUTE ;
SYNONYM ENDIF THEN
SYNONYM \G \
: UVALUE  0 VALUE ;
: VOCABULARY  WORDLIST CREATE ,
   DOES> @ >R GET-ORDER NIP R> SWAP SET-ORDER ;
\ Output and numbers.
: UD.R    >R <# #S #> R> OVER - SPACES TYPE ;
: UD.     0 UD.R SPACE ;
: DEC.    BASE @ >R DECIMAL . R> BASE ! ;
: HEX.    BASE @ >R HEX [CHAR] $ EMIT U. R> BASE ! ;
: FNIP    FSWAP FDROP ;
: FTUCK   FSWAP FOVER ;
: F-ROT   FROT FROT ;
: F2*     FDUP F+ ;
: F2/     0.5E F* ;
3.141592653589793E FCONSTANT PI
\ Fixed-width memory and files.
: L,      HERE 4 ALLOT L! ;
: W,      HERE 2 ALLOT W! ;
: SLURP-FID  ( fid -- c-addr u )
   >R 0 0 R@ REPOSITION-FILE THROW  R@ FILE-SIZE THROW DROP
   DUP 1 MAX ALLOCATE THROW SWAP  2DUP R> READ-FILE THROW NIP ;
: SLURP-FILE ( c-addr u -- c-addr u )
   R/O OPEN-FILE THROW >R  R@ SLURP-FID  R> CLOSE-FILE THROW ;
VARIABLE ASSERT-LEVEL   1 ASSERT-LEVEL !
