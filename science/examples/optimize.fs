\ An example of minimization and curve fitting with the SCIENCE words; run it with
\   cargo run --manifest-path science/Cargo.toml -- science/examples/optimize.fs

ALSO SCIENCE
6 SET-PRECISION

\ Nelder-Mead needs no derivative. The function takes a vector of the n unknowns and leaves
\ a float. Rosenbrock's valley, f = (1-x)^2 + 100 (y-x^2)^2, has its minimum 0 at (1, 1).
: ROSENBROCK ( v -- ) ( F: -- f )
   {: v :}
   1e 0 v FV[] F@ F-  FDUP F*
   1 v FV[] F@  0 v FV[] F@ FDUP F*  F-  FDUP F*  100e F*  F+ ;
2 FV-NEW CONSTANT X
-1.2e 0 X FV[] F!   1e 1 X FV[] F!
\ The step 0.5 sizes the first simplex; the tolerance 1e-12 is on the values.
0.5e 1e-12 X ' ROSENBROCK NELDER-MEAD
." Rosenbrock: value " F.  ." at " 0 X FV[] F@ F.  1 X FV[] F@ F.  CR

\ Curve fitting: y = a e^(b t) through eight points, found from a poor start. The data are
\ 2 e^(-t/2) with 0.01 added at the even points and taken off at the odd ones.
8 FV-NEW CONSTANT TS
8 FV-NEW CONSTANT YS
: DATA  8 0 DO
      I S>F  FDUP I TS FV[] F!
      -0.5e F* FEXP 2e F*
      I 2 MOD IF -0.01e ELSE 0.01e THEN F+  I YS FV[] F!
   LOOP ;
DATA

\ The word for LM-FIT fills r with the model minus the data, for the parameters p.
: RESIDUALS ( p r -- )
   {: p r :}
   8 0 DO
      0 p FV[] F@  1 p FV[] F@  I TS FV[] F@ F*  FEXP F*  I YS FV[] F@ F-  I r FV[] F!
   LOOP ;
2 FV-NEW CONSTANT P
1e 0 P FV[] F!   -1e 1 P FV[] F!
1e-12 P ' RESIDUALS 8 LM-FIT
." fit: a = " 0 P FV[] F@ F.  ." b = " 1 P FV[] F@ F.  ." sum of squares " F.  CR

\ The standard errors of a and b are the square roots of the diagonal of the covariance.
P ' RESIDUALS 8 LM-COVARIANCE CONSTANT C
." errors: " 0 0 C MAT[] F@ FSQRT F.  1 1 C MAT[] F@ FSQRT F.  CR
