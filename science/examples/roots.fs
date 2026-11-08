\ An example of roots, polynomials and interpolation with the SCIENCE words; run it with
\   cargo run --manifest-path science/Cargo.toml -- science/examples/roots.fs

ALSO SCIENCE
6 SET-PRECISION

\ The roots of x^2 - 3x + 2 and of x^3 - 6x^2 + 11x - 6, from the formulas. The count
\ comes on the data stack, the roots (smallest deepest) on the float stack.
: SHOW-ROOTS ( n -- )  0 ?DO F. LOOP CR ;
." quadratic: "  1e -3e 2e QUADRATIC  SHOW-ROOTS
." cubic: "  1e -6e 11e -6e CUBIC  SHOW-ROOTS
." no real roots of x^2 + 1: "  1e 0e 1e QUADRATIC . CR

\ A polynomial is a vector of coefficients, the constant term first. Laguerre's method finds
\ a root near a start; deflating by that root leaves a polynomial for the others.
\ Here x^4 - 10x^3 + 35x^2 - 50x + 24 = (x-1)(x-2)(x-3)(x-4).
24e -50e 35e -10e 1e 5 FV-FROM-STACK CONSTANT P
: ROOTS ( c -- )  {: c :}
   c FV-LEN 1- 0 ?DO
      0e 1e-12 c POLY-LAGUERRE  FDUP F.
      c POLY-DEFLATE TO c
   LOOP  CR ;
." roots by Laguerre and deflation: "  P ROOTS
." p(2.5) = "  2.5e P POLY-EVAL F.  CR

\ Orthogonal polynomials: T_5(0.3) = cos(5 acos 0.3), and P_3 and H_3 at 0.3.
." T5(0.3) = "  5 0.3e CHEBYSHEV-T F.  CR
." P3(0.3) = "  3 0.3e LEGENDRE F.  CR
." H3(0.3) = "  3 0.3e HERMITE F.  CR

\ Root finding for a function: the word passed as an execution token has the effect
\ ( F: x -- y ). Here cos(x) = x, a root of cos(x) - x, in [0, 1].
: F ( F: x -- y )  FDUP FCOS FSWAP F- ;
: DF ( F: x -- y )  FSIN FNEGATE 1e F- ;
." bisection: "      0e 1e 1e-12 ' F BISECT F.  CR
." regula falsi: "   0e 1e 1e-12 ' F REGULA-FALSI F.  CR
." Newton: "         1e 1e-12 ' F ' DF NEWTON F.  CR

\ Interpolation: nine points of sin on [0, 2]. AITKEN also gives an estimate of the error.
9 FV-NEW CONSTANT XS
9 FV-NEW CONSTANT YS
: FILL  9 0 DO  I S>F 0.25e F*  FDUP I XS FV[] F!  FSIN I YS FV[] F!  LOOP ;
FILL
." sin(0.9) = "  0.9e FSIN F.  CR
." Lagrange: "   0.9e XS YS LAGRANGE F.  CR
." Aitken: "      0.9e XS YS AITKEN  FDROP F.  CR
XS YS NEWTON-DD CONSTANT DD
." Newton's form: "  0.9e XS DD NEWTON-EVAL F.  CR

\ A rational function does better where a function has a pole: 1 / (1 - x^2) near x = 1.
5 FV-NEW CONSTANT RX
5 FV-NEW CONSTANT RY
: FILL2  5 0 DO  I S>F 0.2e F*  FDUP I RX FV[] F!  FDUP F* 1e FSWAP F- 1e FSWAP F/  I RY FV[] F!  LOOP ;
FILL2
." 1/(1-0.9^2) = "  1e 1e 0.9e 0.9e F* F- F/ F.  CR
." polynomial: "  0.9e RX RY LAGRANGE F.  CR
." rational: "    0.9e RX RY RATIONAL-INTERP FDROP F.  CR
