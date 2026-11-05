\ Tolerances for the float cells of T{ ... -> ... }T, as in ttester.fs of the
\ Forth-2012 test suite: SET-NEAR compares with them, SET-EXACT (the default)
\ asks for identical floats. A float is near when it is within REL-NEAR
\ relatively or within ABS-NEAR absolutely.
FVARIABLE REL-NEAR  1E-12 REL-NEAR F!
FVARIABLE ABS-NEAR  0E    ABS-NEAR F!
