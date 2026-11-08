\ Root finding for a function that the program passes as an execution token, in the word
\ list SCIENCE (see lib.rs). The function has the effect ( F: x -- y ). Like calculus.fs
\ the words keep their floats in a scratch vector, so a function may find a root again.
\ A bracket that does not hold a root, a NaN, or a method that does not settle gives -264.

\ The sign of the float on top: -1, 0 or 1; -264 for a NaN.
: (SIGN) ( F: x -- ) ( -- n )
   FDUP FDUP F= 0= IF -264 THROW THEN
   FDUP F0< IF FDROP -1 EXIT THEN
   F0= IF 0 ELSE 1 THEN ;

\ Bisection: halves [a, b] until it is no wider than tol. f(a) and f(b) must differ in
\ sign. The scratch vector holds 0 a, 1 b, 2 tol, 3 the midpoint.
: BISECT ( xt -- ) ( F: a b tol -- r )
   {: xt | s sa sb sm :}
   4 FV-NEW TO s
   2 s (S!)  1 s (S!)  0 s (S!)
   0 s (S@) xt EXECUTE (SIGN) TO sa
   1 s (S@) xt EXECUTE (SIGN) TO sb
   sb 0= IF 1 s (S@) s FREE THROW EXIT THEN
   sa 0= IF 0 s (S@) s FREE THROW EXIT THEN
   sa sb * 0 >= IF -264 THROW THEN
   BEGIN
      0 s (S@) 1 s (S@) F+ 2e F/ 3 s (S!)
      1 s (S@) 0 s (S@) F- FABS 2 s (S@) (F<=)
      3 s (S@) 0 s (S@) F=  3 s (S@) 1 s (S@) F=  OR OR
      DUP 0= IF
         DROP
         3 s (S@) xt EXECUTE (SIGN) TO sm
         sm 0= IF TRUE
         ELSE
            sm sa * 0< IF 3 s (S@) 1 s (S!) ELSE 3 s (S@) 0 s (S!) sm TO sa THEN
            FALSE
         THEN
      THEN
   UNTIL
   3 s (S@) s FREE THROW ;

\ Regula falsi with the Illinois rule: the secant through the ends of the bracket, and
\ the value at the end that stays is halved when it stays twice. Ends when the bracket is no
\ wider than tol, so a function this steep needs many steps (-264 after 200; BISECT is the
\ word for it). The scratch vector holds 0 a, 1 b, 2 tol, 3 f(a), 4 f(b), 5 c, 6 f(c).
: REGULA-FALSI ( xt -- ) ( F: a b tol -- r )
   {: xt | s side k n :}
   8 FV-NEW TO s
   2 s (S!)  1 s (S!)  0 s (S!)
   0 s (S@) xt EXECUTE 3 s (S!)
   1 s (S@) xt EXECUTE 4 s (S!)
   4 s (S@) (SIGN) 0= IF 1 s (S@) s FREE THROW EXIT THEN
   3 s (S@) (SIGN) 0= IF 0 s (S@) s FREE THROW EXIT THEN
   3 s (S@) (SIGN) 4 s (S@) (SIGN) * 0 > IF -264 THROW THEN
   0 TO side  0 TO k
   BEGIN
      0 s (S@) 4 s (S@) F*  1 s (S@) 3 s (S@) F*  F-  4 s (S@) 3 s (S@) F-  F/  5 s (S!)
      5 s (S@) xt EXECUTE FDUP 6 s (S!) (SIGN) TO n
      n 0= IF TRUE
      ELSE
         n 4 s (S@) (SIGN) = IF
            5 s (S@) 1 s (S!)  6 s (S@) 4 s (S!)
            side -1 = IF 3 s (S@) 2e F/ 3 s (S!) THEN
            -1 TO side
         ELSE
            5 s (S@) 0 s (S!)  6 s (S@) 3 s (S!)
            side 1 = IF 4 s (S@) 2e F/ 4 s (S!) THEN
            1 TO side
         THEN
         1 s (S@) 0 s (S@) F- FABS 2 s (S@) (F<=)
         k 1+ TO k
         DUP 0= k 200 >= AND IF -264 THROW THEN
      THEN
   UNTIL
   5 s (S@) s FREE THROW ;

\ Newton's method from x0 with the function and its derivative (both
\ ( F: x -- y )): x - f/f' until the step is at most tol * max(1, |x|). -264 when the
\ derivative is 0 or after 100 steps. The scratch vector holds 0 x, 1 tol, 2 the step.
: NEWTON ( xt dxt -- ) ( F: x0 tol -- r )
   {: xt dxt | s k :}
   4 FV-NEW TO s
   1 s (S!)  0 s (S!)
   0 TO k
   BEGIN
      0 s (S@) dxt EXECUTE  FDUP 0e F= IF -264 THROW THEN
      0 s (S@) xt EXECUTE FSWAP F/  2 s (S!)
      0 s (S@) 2 s (S@) F-  0 s (S!)
      2 s (S@) FABS  0 s (S@) FABS 1e FMAX 1 s (S@) F*  (F<=)
      k 1+ TO k
      DUP 0= k 100 >= AND IF -264 THROW THEN
   UNTIL
   0 s (S@) s FREE THROW ;
