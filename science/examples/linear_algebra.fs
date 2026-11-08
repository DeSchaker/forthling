\ An example of the SCIENCE words; run it with
\   cargo run --manifest-path science/Cargo.toml -- science/examples/linear_algebra.fs

ALSO SCIENCE
4 SET-PRECISION

: .VECTOR ( v -- )  {: v :}  v FV-LEN 0 ?DO  I v FV[] F@ F.  LOOP  CR ;
: .MATRIX ( m -- )  {: m :}
   m MAT-ROWS 0 ?DO  m MAT-COLS 0 ?DO  J I m MAT[] F@ F.  LOOP  CR  LOOP ;

\ Three equations with three unknowns:
\    2x +  y -  z =   8
\   -3x -  y + 2z = -11
\   -2x +  y + 2z =  -3
\ The matrix of the coefficients, row by row, and the right-hand sides.
2e  1e -1e
-3e -1e  2e
-2e  1e  2e   3 3 MAT-FROM-STACK  CONSTANT A
8e -11e -3e   3 FV-FROM-STACK     CONSTANT B

." x y z: "  B A MAT-SOLVE  DUP .VECTOR  FREE THROW
." determinant: "  A MAT-DET F.  CR
." inverse:" CR  A MAT-INV  DUP .MATRIX  FREE THROW

\ A factorization is made once and used for many right-hand sides.
A MAT-LU  DROP  CONSTANT PIV  CONSTANT LU
." two more right-hand sides: "
1e 0e 0e 3 FV-FROM-STACK  LU PIV LU-SOLVE  DUP .VECTOR  FREE THROW
." and: "
0e 0e 1e 3 FV-FROM-STACK  LU PIV LU-SOLVE  DUP .VECTOR  FREE THROW

\ The Hilbert matrix is badly conditioned: the condition number grows fast, and
\ solving H x = (H times a vector of ones) gives less and less of the ones.
: ERROR ( x v -- ) ( F: -- r )  \ the largest distance of an element of x to 1
   {: x :}  0e  x FV-LEN 0 ?DO  I x FV[] F@ 1e F- FABS FMAX  LOOP ;
: HILBERT-TEST ( n -- )  {: n :}
   n MAT-HILBERT  {: h :}
   n FV-NEW {: ones :}  1e ones FV-FILL
   ones h SWAP MAT*V {: b :}
   b h MAT-SOLVE {: x :}
   n . 2 SPACES  h MAT-COND FS.  2 SPACES  x ERROR FS.  CR
   h FREE THROW  ones FREE THROW  b FREE THROW  x FREE THROW ;
." n   condition number   largest error" CR
: HILBERT-TESTS ( -- )  10 3 DO  I HILBERT-TEST  LOOP ;
HILBERT-TESTS
