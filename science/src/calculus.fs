\ Integration and ordinary differential equations, in the word list SCIENCE (see lib.rs).
\ These words call a function that the program passes as an execution token, which a word
\ written in Rust cannot do, so they are written in Forth. There are no float locals: each
\ word keeps its floats in a small scratch vector (so the words can call each other, and a
\ function may integrate again), and frees it at the end.

: (F<=) ( F: x y -- ) ( -- flag )  FOVER FOVER F< >R F= R> OR ;
: (S!) ( i s -- ) ( F: x -- )  FV[] F! ;
: (S@) ( i s -- ) ( F: -- x )  FV[] F@ ;

\ --- Integration: the function has the effect ( F: x -- y ) ---

\ The composite trapezoid rule with n panels.
: TRAPEZOID ( xt n -- ) ( F: a b -- r )
   {: xt n | s :}
   n 1 < IF -24 THROW THEN
   4 FV-NEW TO s                       \ 0 a, 1 h, 2 sum
   FOVER F-  n S>F F/  1 s (S!)  0 s (S!)
   0 s (S@) xt EXECUTE
   0 s (S@) n S>F 1 s (S@) F* F+ xt EXECUTE F+  2e F/  2 s (S!)
   n 1 ?DO  2 s (S@)  0 s (S@) I S>F 1 s (S@) F* F+ xt EXECUTE F+  2 s (S!)  LOOP
   2 s (S@) 1 s (S@) F*
   s FREE THROW ;

\ Simpson's rule with n panels (n is made even).
: SIMPSON ( xt n -- ) ( F: a b -- r )
   {: xt n | s :}
   n 1 < IF -24 THROW THEN
   n 1 AND IF n 1+ TO n THEN
   4 FV-NEW TO s                       \ 0 a, 1 h, 2 sum
   FOVER F-  n S>F F/  1 s (S!)  0 s (S!)
   0 s (S@) xt EXECUTE
   0 s (S@) n S>F 1 s (S@) F* F+ xt EXECUTE F+  2 s (S!)
   n 1 ?DO
      0 s (S@) I S>F 1 s (S@) F* F+ xt EXECUTE
      I 1 AND IF 4e ELSE 2e THEN F*  2 s (S@) F+  2 s (S!)
   LOOP
   2 s (S@) 1 s (S@) F* 3e F/
   s FREE THROW ;

\ The trapezoid sums T0, T1, ... that halve the panels. The scratch vector holds
\ 0 a, 1 b, 2 the latest sum, 3 the panel width.
: (T0) ( xt s -- )
   {: xt s :}
   0 s (S@) xt EXECUTE  1 s (S@) xt EXECUTE F+
   1 s (S@) 0 s (S@) F-  F* 2e F/  2 s (S!) ;
: (TNEXT) ( xt level s -- )             \ from level-1 to level
   {: xt level s | n :}
   1 level 1- LSHIFT TO n              \ the new points
   1 s (S@) 0 s (S@) F-  n 2* S>F F/  3 s (S!)
   0e  n 0 DO  0 s (S@) I 2* 1+ S>F 3 s (S@) F* F+ xt EXECUTE F+  LOOP
   3 s (S@) F*  2 s (S@) 2e F/ F+  2 s (S!) ;
: (CONVERGED?) ( k -- flag ) ( F: err r tol -- )   \ err <= tol * max(1, |r|), after 3 halvings
   FSWAP FABS 1e FMAX F*  (F<=)  SWAP 3 >= AND ;

\ The trapezoid rule, halving the panels until two sums differ by at most
\ tol * max(1, |r|); leaves the result and the difference. -264 after 20 halvings.
: TRAPEZOID-TOL ( xt -- ) ( F: a b tol -- r err )
   {: xt | s k :}
   8 FV-NEW TO s                       \ 4 previous sum, 7 tol
   7 s (S!)  1 s (S!)  0 s (S!)
   xt s (T0)
   1 TO k
   BEGIN
      2 s (S@) 4 s (S!)
      xt k s (TNEXT)
      2 s (S@) 4 s (S@) F- FABS
      FDUP 2 s (S@) 7 s (S@) k (CONVERGED?)
      IF  2 s (S@) FSWAP  TRUE
      ELSE  FDROP  k 20 >= IF -264 THROW THEN  k 1+ TO k  FALSE  THEN
   UNTIL
   s FREE THROW ;

\ Romberg's method: the trapezoid sums with Richardson extrapolation. The vector e
\ holds the latest row of the table; the scratch vector holds 4 the best value
\ so far, 5 the power of 4, 6 the best value of the row before.
: (ROMBERG-ROW) ( k e s -- )
   {: k e s :}
   2 s (S@) 4 s (S!)
   4e 5 s (S!)
   k 0 ?DO
      I e FV[] F@
      4 s (S@) I e FV[] F!
      FDUP 6 s (S!)
      4 s (S@) 5 s (S@) F*  FSWAP F-  5 s (S@) 1e F- F/  4 s (S!)
      5 s (S@) 4e F*  5 s (S!)
   LOOP
   4 s (S@) k e FV[] F! ;
: ROMBERG ( xt -- ) ( F: a b tol -- r err )
   {: xt | s e k :}
   8 FV-NEW TO s                       \ 7 tol
   21 FV-NEW TO e
   7 s (S!)  1 s (S!)  0 s (S!)
   xt s (T0)  0 e s (ROMBERG-ROW)
   1 TO k
   BEGIN
      xt k s (TNEXT)  k e s (ROMBERG-ROW)
      4 s (S@) 6 s (S@) F- FABS
      FDUP 4 s (S@) 7 s (S@) k (CONVERGED?)
      IF  4 s (S@) FSWAP  TRUE
      ELSE  FDROP  k 20 >= IF -264 THROW THEN  k 1+ TO k  FALSE  THEN
   UNTIL
   s FREE THROW  e FREE THROW ;

\ Gauss-Legendre with 5 points on [a, b].
0.5384693101056831e FCONSTANT (GL-X1)
0.906179845938664e  FCONSTANT (GL-X2)
0.4786286704993665e FCONSTANT (GL-W1)
0.2369268850561891e FCONSTANT (GL-W2)
: (G5PAIR) ( xt v -- ) ( F: x w -- r )   \ w * (f(m - c x) + f(m + c x))
   {: xt v :}
   3 v (S!)  1 v (S@) F*  4 v (S!)
   0 v (S@) 4 v (S@) F- xt EXECUTE
   0 v (S@) 4 v (S@) F+ xt EXECUTE F+
   3 v (S@) F* ;
: (G5) ( xt -- ) ( F: a b -- r )
   {: xt | v :}
   5 FV-NEW TO v                       \ 0 midpoint, 1 half width
   FOVER F-  2e F/  1 v (S!)           \ F: a
   1 v (S@) F+  0 v (S!)
   0 v (S@) xt EXECUTE  128e 225e F/ F*
   (GL-X1) (GL-W1) xt v (G5PAIR) F+
   (GL-X2) (GL-W2) xt v (G5PAIR) F+
   1 v (S@) F*
   v FREE THROW ;

\ Adaptive Gauss-Legendre. A row of the matrix m is an interval: 0 a, 1 b, 2 its
\ integral (two 5-point rules on the halves), 3 the difference with one rule on all
\ of it, which is the error estimate (usually far above the true error).
500 CONSTANT (INTERVALS)
: (ROW!) ( xt m r -- ) ( F: a b -- )
   {: xt m r | v :}
   4 FV-NEW TO v                       \ 0 a, 1 b, 2 midpoint, 3 coarse value
   1 v (S!)  0 v (S!)
   0 v (S@) r 0 m MAT[] F!  1 v (S@) r 1 m MAT[] F!
   0 v (S@) 1 v (S@) F+ 2e F/ 2 v (S!)
   0 v (S@) 1 v (S@) xt (G5)  3 v (S!)
   0 v (S@) 2 v (S@) xt (G5)  2 v (S@) 1 v (S@) xt (G5) F+
   FDUP r 2 m MAT[] F!
   3 v (S@) F- FABS  r 3 m MAT[] F!
   v FREE THROW ;
: (TOTALS) ( m n s -- )                 \ the exact sums of the integrals and the errors
   {: m n s :}
   0e 1 s (S!)  0e 2 s (S!)
   n 0 ?DO
      1 s (S@) I 2 m MAT[] F@ F+ 1 s (S!)
      2 s (S@) I 3 m MAT[] F@ F+ 2 s (S!)
   LOOP ;
: (WORST) ( m n s -- i )                \ the interval with the largest error
   {: m n s | w :}
   -1e 3 s (S!)  0 TO w
   n 0 ?DO
      I 3 m MAT[] F@
      3 s (S@) FOVER F< IF  3 s (S!)  I TO w  ELSE  FDROP  THEN
   LOOP
   w ;
: (ENOUGH?) ( s -- flag )               \ the errors add up to at most tol * max(1, |sum|)
   {: s :}  2 s (S@) 1 s (S@) FABS 1e FMAX 0 s (S@) F* (F<=) ;
: INTEGRATE ( xt -- ) ( F: a b tol -- r err )
   {: xt | m n s worst :}
   (INTERVALS) 4 MAT-NEW TO m
   6 FV-NEW TO s                       \ 0 tol, 1 sum, 2 error sum, 3 largest error, 4 midpoint, 5 b
   0 s (S!)
   xt m 0 (ROW!)  1 TO n  m n s (TOTALS)
   BEGIN
      \ The sums are kept up to date by adding and subtracting; a result is
      \ confirmed with the exact sums.
      s (ENOUGH?) DUP IF  DROP m n s (TOTALS)  s (ENOUGH?)  THEN
      DUP 0= IF
         n (INTERVALS) >= IF -264 THROW THEN
         m n s (WORST) TO worst
         1 s (S@) worst 2 m MAT[] F@ F-  1 s (S!)
         2 s (S@) worst 3 m MAT[] F@ F-  2 s (S!)
         worst 0 m MAT[] F@  worst 1 m MAT[] F@
         FOVER FOVER F+ 2e F/  4 s (S!)
         5 s (S!)
         4 s (S@) xt m worst (ROW!)
         4 s (S@) 5 s (S@) xt m n (ROW!)
         1 s (S@) worst 2 m MAT[] F@ F+  n 2 m MAT[] F@ F+  1 s (S!)
         2 s (S@) worst 3 m MAT[] F@ F+  n 3 m MAT[] F@ F+  2 s (S!)
         n 1+ TO n
      THEN
   UNTIL
   1 s (S@) 2 s (S@)
   m FREE THROW  s FREE THROW ;

\ --- Ordinary differential equations ---
\ The state y is a float vector; the derivative is a word with the effect
\ ( y dy -- ) ( F: t -- ) that fills dy with y'(t). The solvers change y in place.

\ out = y + h * (sum over j < cols of tab[r][j] * k_j), or without y when y is 0.
: (COMB) ( n y ks tab r cols out -- ) ( F: h -- )
   {: n y ks tab r cols out :}
   n 0 ?DO
      0e
      cols 0 ?DO  r I tab MAT[] F@  I CELLS ks + @  J SWAP FV[] F@  F*  F+  LOOP
      FOVER F*
      y IF I y FV[] F@ ELSE 0e THEN F+
      I out FV[] F!
   LOOP
   FDROP ;

\ out = y + c * k.
: (LC) ( y k out -- ) ( F: c -- )
   {: y k o :}
   y FV-LEN 0 ?DO  FDUP I k FV[] F@ F*  I y FV[] F@ F+  I o FV[] F!  LOOP
   FDROP ;

\ Classical Runge-Kutta of order 4, with a fixed number of steps from t0 to t1.
: RK4 ( y xt steps -- ) ( F: t0 t1 -- )
   {: y xt steps | n s k1 k2 k3 k4 yt :}
   steps 1 < IF -24 THROW THEN
   y FV-LEN TO n
   n FV-NEW TO k1  n FV-NEW TO k2  n FV-NEW TO k3  n FV-NEW TO k4  n FV-NEW TO yt
   3 FV-NEW TO s                       \ 0 t, 1 h, 2 t0
   FOVER F-  steps S>F F/  1 s (S!)  2 s (S!)
   steps 0 ?DO
      2 s (S@) I S>F 1 s (S@) F* F+  0 s (S!)
      0 s (S@)  y k1 xt EXECUTE
      1 s (S@) 2e F/  y k1 yt (LC)
      0 s (S@) 1 s (S@) 2e F/ F+  yt k2 xt EXECUTE
      1 s (S@) 2e F/  y k2 yt (LC)
      0 s (S@) 1 s (S@) 2e F/ F+  yt k3 xt EXECUTE
      1 s (S@)  y k3 yt (LC)
      0 s (S@) 1 s (S@) F+  yt k4 xt EXECUTE
      n 0 ?DO
         I k1 FV[] F@  I k2 FV[] F@ 2e F* F+  I k3 FV[] F@ 2e F* F+  I k4 FV[] F@ F+
         1 s (S@) 6e F/ F*  I y FV[] F@ F+  I y FV[] F!
      LOOP
   LOOP
   k1 FREE THROW  k2 FREE THROW  k3 FREE THROW  k4 FREE THROW  yt FREE THROW  s FREE THROW ;

\ Fehlberg's method of orders 4 and 5. Rows 0-4 of the table give the coefficients of
\ stages 2-6 (columns 0-4) and the step fraction c (column 5); row 5 gives the weights
\ of the 5th order solution, row 6 the weights of the difference between the orders.
: (TROW) ( m r -- ) ( F: x0 ... x5 -- )
   {: m r :}  6 0 DO  r 5 I - m MAT[] F!  LOOP ;
: (RKF-TABLE) ( -- m )
   7 6 MAT-NEW {: m :}
   1e 4e F/  0e 0e 0e 0e  1e 4e F/            m 0 (TROW)
   3e 32e F/  9e 32e F/  0e 0e 0e  3e 8e F/   m 1 (TROW)
   1932e 2197e F/  -7200e 2197e F/  7296e 2197e F/  0e 0e  12e 13e F/   m 2 (TROW)
   439e 216e F/  -8e  3680e 513e F/  -845e 4104e F/  0e  1e   m 3 (TROW)
   -8e 27e F/  2e  -3544e 2565e F/  1859e 4104e F/  -11e 40e F/  1e 2e F/   m 4 (TROW)
   16e 135e F/  0e  6656e 12825e F/  28561e 56430e F/  -9e 50e F/  2e 55e F/   m 5 (TROW)
   1e 360e F/  0e  -128e 4275e F/  -2197e 75240e F/  1e 50e F/  2e 55e F/   m 6 (TROW)
   m ;

\ The step factor for a scaled error en: 0.9 en^-1/5 between 1/5 and 5; a NaN gives 1/5.
: (FACTOR) ( F: en -- f )
   FDUP FDUP F= 0= IF FDROP 0.2e EXIT THEN
   FDUP 0e F= IF FDROP 5e EXIT THEN
   -0.2e F** 0.9e F* 0.2e FMAX 5e FMIN ;

\ Adaptive Runge-Kutta-Fehlberg from t0 to t1. The error of a step, per component, is
\ kept below tol * (1 + |y|); -264 when the step gets too small or after 100000 steps.
: ODE-RKF ( y xt -- ) ( F: t0 t1 tol -- )
   {: y xt | n s ks tab y5 er yt final ok steps :}
   y FV-LEN TO n
   6 FV-NEW TO s                       \ 0 t, 1 t1, 2 h, 3 tol, 4 the step, 5 scaled error
   3 s (S!)  1 s (S!)  0 s (S!)
   1 s (S@) 0 s (S@) F-  100e F/  2 s (S!)
   (RKF-TABLE) TO tab
   6 CELLS ALLOCATE THROW TO ks
   6 0 DO  n FV-NEW  I CELLS ks + !  LOOP
   n FV-NEW TO y5  n FV-NEW TO er  n FV-NEW TO yt
   0 TO steps
   BEGIN
      steps 100000 >= IF -264 THROW THEN  steps 1+ TO steps
      1 s (S@) 0 s (S@) F-
      FDUP FABS 2 s (S@) FABS (F<=)
      IF  4 s (S!)  TRUE TO final  ELSE  FDROP  2 s (S@) 4 s (S!)  FALSE TO final  THEN
      0 s (S@)  y ks @ xt EXECUTE
      5 0 DO
         4 s (S@)  n y ks tab I I 1+ yt (COMB)
         0 s (S@)  I 5 tab MAT[] F@  4 s (S@) F*  F+
         yt  I 1+ CELLS ks + @  xt EXECUTE
      LOOP
      4 s (S@)  n y ks tab 5 6 y5 (COMB)
      4 s (S@)  n 0 ks tab 6 6 er (COMB)
      0e
      n 0 ?DO
         I er FV[] F@ FABS
         I y FV[] F@ FABS  I y5 FV[] F@ FABS FMAX  1e F+  3 s (S@) F*
         F/ FMAX
      LOOP
      FDUP 5 s (S!)
      1e (F<=) TO ok
      ok IF
         n 0 ?DO  I y5 FV[] F@  I y FV[] F!  LOOP
         final IF  1 s (S@) 0 s (S!)  ELSE  0 s (S@) 4 s (S@) F+  0 s (S!)  THEN
      THEN
      5 s (S@) (FACTOR)  4 s (S@) F*  2 s (S!)
      ok final AND 0= IF
         2 s (S@) FABS  0 s (S@) FABS 1e FMAX  1e-15 F*  F<
         IF  -264 THROW  THEN
      THEN
      ok final AND
   UNTIL
   6 0 DO  I CELLS ks + @ FREE THROW  LOOP
   ks FREE THROW  tab FREE THROW  y5 FREE THROW  er FREE THROW  yt FREE THROW  s FREE THROW ;
