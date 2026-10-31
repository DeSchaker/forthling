\ Tests for the XCHAR word set and its extensions, in UTF-8.
\ The Forth-2012 test suite has none, so these are forthling's own. They
\ need tester.fr. The examples: A is $41 (1 byte), é is $E9 (2 bytes),
\ € is $20AC (3 bytes), 中 is $4E2D (3 bytes, 2 columns wide) and
\ 😀 is $1F600 (4 bytes, 2 columns wide).

DECIMAL

TESTING XC-SIZE XC!+ XC@+ XCHAR+ XC,

T{ $41 XC-SIZE -> 1 }T
T{ $E9 XC-SIZE -> 2 }T
T{ $7FF XC-SIZE -> 2 }T
T{ $800 XC-SIZE -> 3 }T
T{ $20AC XC-SIZE -> 3 }T
T{ $1F600 XC-SIZE -> 4 }T
T{ $10FFFF XC-SIZE -> 4 }T

CREATE XBUF 16 ALLOT
: XBYTES ( n -- c1 .. cn ) XBUF SWAP 0 ?DO DUP C@ SWAP CHAR+ LOOP DROP ;

T{ $41 XBUF XC!+ XBUF - -> 1 }T
T{ 1 XBYTES -> $41 }T
T{ $E9 XBUF XC!+ XBUF - -> 2 }T
T{ 2 XBYTES -> $C3 $A9 }T
T{ $20AC XBUF XC!+ XBUF - -> 3 }T
T{ 3 XBYTES -> $E2 $82 $AC }T
T{ $1F600 XBUF XC!+ XBUF - -> 4 }T
T{ 4 XBYTES -> $F0 $9F $98 $80 }T

T{ XBUF XC@+ SWAP XBUF - -> $1F600 4 }T
T{ $E9 XBUF XC!+ $20AC SWAP XC!+ $41 SWAP XC!+ XBUF - -> 6 }T
T{ XBUF XC@+ SWAP XC@+ SWAP XC@+ SWAP XBUF - -> $E9 $20AC $41 6 }T
T{ XBUF XCHAR+ XCHAR+ XCHAR+ XBUF - -> 6 }T

\ A byte that does not start a complete xchar is an xchar of its own.
T{ $A9 XBUF C! XBUF XC@+ SWAP XBUF - -> $A9 1 }T
T{ $C3 XBUF C! $41 XBUF CHAR+ C! XBUF XC@+ SWAP XBUF - -> $C3 1 }T

T{ HERE $20AC XC, HERE SWAP - -> 3 }T
T{ HERE 3 - XC@+ NIP -> $20AC }T
ALIGN

TESTING XC!+? X-SIZE +X/STRING X\STRING- XCHAR- -TRAILING-GARBAGE

T{ $20AC XBUF 3 XC!+? ROT XBUF - -ROT -> 3 0 <TRUE> }T
T{ $20AC XBUF 2 XC!+? ROT XBUF - -ROT -> 0 2 <FALSE> }T
T{ $41 XBUF 0 XC!+? ROT XBUF - -ROT -> 0 0 <FALSE> }T

: S-MIXED ( -- c-addr u ) S" Aé€😀" ;
T{ S-MIXED NIP -> 10 }T
T{ S-MIXED X-SIZE -> 1 }T
T{ S-MIXED 1 /STRING X-SIZE -> 2 }T
T{ S-MIXED 3 /STRING X-SIZE -> 3 }T
T{ S-MIXED 6 /STRING X-SIZE -> 4 }T
T{ S-MIXED DROP 0 X-SIZE -> 0 }T

T{ S-MIXED +X/STRING NIP -> 9 }T
T{ S-MIXED +X/STRING +X/STRING NIP -> 7 }T
T{ S-MIXED +X/STRING +X/STRING +X/STRING DROP XC@+ NIP -> $1F600 }T
T{ S-MIXED X\STRING- NIP -> 6 }T
T{ S-MIXED X\STRING- X\STRING- NIP -> 3 }T
T{ S-MIXED X\STRING- X\STRING- X\STRING- X\STRING- NIP -> 0 }T
T{ S-MIXED DROP DUP 10 + XCHAR- SWAP - -> 6 }T
T{ S-MIXED DROP DUP 6 + XCHAR- SWAP - -> 3 }T
T{ S-MIXED DROP DUP 1 + XCHAR- SWAP - -> 0 }T

\ Cutting the string inside 😀 or € leaves an incomplete xchar at the end.
T{ S-MIXED DROP 8 -TRAILING-GARBAGE NIP -> 6 }T
T{ S-MIXED DROP 5 -TRAILING-GARBAGE NIP -> 3 }T
T{ S-MIXED -TRAILING-GARBAGE NIP -> 10 }T
T{ S-MIXED DROP 1 -TRAILING-GARBAGE NIP -> 1 }T
T{ S-MIXED DROP 0 -TRAILING-GARBAGE NIP -> 0 }T

TESTING XC-WIDTH X-WIDTH

T{ $41 XC-WIDTH -> 1 }T
T{ $E9 XC-WIDTH -> 1 }T
T{ $4E2D XC-WIDTH -> 2 }T
T{ $1F600 XC-WIDTH -> 2 }T
T{ $301 XC-WIDTH -> 0 }T
T{ 7 XC-WIDTH -> 0 }T
T{ S-MIXED X-WIDTH -> 5 }T
T{ S" 中文" X-WIDTH -> 4 }T
T{ S" " X-WIDTH -> 0 }T

TESTING CHAR [CHAR] 'x' PARSE XHOLD

T{ CHAR A -> $41 }T
T{ CHAR é -> $E9 }T
T{ CHAR €uro -> $20AC }T
T{ CHAR 😀 -> $1F600 }T
: GC1 [CHAR] € ;
: GC2 [CHAR] 中 ;
T{ GC1 GC2 -> $20AC $4E2D }T
T{ 'é' '€' '😀' -> $E9 $20AC $1F600 }T
: GC3 '中' ;
T{ GC3 -> $4E2D }T

T{ $20AC PARSE ab€ NIP -> 2 }T
T{ $20AC PARSE abc€ S" abc" COMPARE -> 0 }T
: GP1 [CHAR] é PARSE ;
T{ GP1 café S" caf" COMPARE -> 0 }T

T{ <# $20AC XHOLD $E9 XHOLD 0 0 #> S" é€" COMPARE -> 0 }T
T{ <# 12 0 # # $20AC XHOLD #> S" €12" COMPARE -> 0 }T

TESTING XEMIT EKEY>XCHAR EKEY>CHAR ENVIRONMENT?

: GX1 $E9 XEMIT $20AC XEMIT $1F600 XEMIT ;
T{ GX1 -> }T
T{ $41 EKEY>XCHAR -> $41 <TRUE> }T
T{ $20AC EKEY>XCHAR -> $20AC <TRUE> }T
T{ K-UP EKEY>XCHAR -> K-UP <FALSE> }T
T{ $41 EKEY>CHAR -> $41 <TRUE> }T
T{ $E9 EKEY>CHAR -> $E9 <FALSE> }T

T{ S" XCHAR-ENCODING" ENVIRONMENT? >R S" UTF-8" COMPARE R> -> 0 <TRUE> }T
T{ S" MAX-XCHAR" ENVIRONMENT? -> $10FFFF <TRUE> }T
T{ S" XCHAR-MAXMEM" ENVIRONMENT? -> 4 <TRUE> }T

CR .( End of XCHAR word tests) CR
