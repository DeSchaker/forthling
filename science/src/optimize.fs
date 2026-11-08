\ Minimization and curve fitting for a function that the program passes as an execution
\ token, in the word list SCIENCE (see lib.rs). Like calculus.fs the words keep their floats
\ in a scratch vector. A NaN from the function, or a method that does not settle, gives -264.

\ --- Small helpers ---

\ Copies a vector into another of the same length.
: (V-COPY) ( from to -- )
   {: from to :}  from FV-LEN 0 ?DO  I from FV[] F@  I to FV[] F!  LOOP ;

\ A NaN on the float stack is an error; the value stays.
: (NOT-NAN) ( F: x -- x )
   FDUP FDUP F= 0= IF -264 THROW THEN ;

\ --- Nelder-Mead simplex ---
\ The simplex is a matrix of n+1 rows (points) by n columns; fv holds the value at each row.

\ Copies vector v into row r of the matrix m.
: (NM-PUT) ( v m r -- )
   {: v m r :}  v FV-LEN 0 ?DO  I v FV[] F@  r I m MAT[] F!  LOOP ;

\ Copies row r of the matrix m into the vector v.
: (NM-GET) ( m r v -- )
   {: m r v :}  v FV-LEN 0 ?DO  r I m MAT[] F@  I v FV[] F!  LOOP ;

\ The rows with the lowest value, the highest and the second highest.
: (NM-RANK) ( fv -- lo hi nx )
   {: fv | lo hi nx :}
   fv FV-LEN 1 ?DO
      I fv FV[] F@  lo fv FV[] F@ F< IF I TO lo THEN
      I fv FV[] F@  hi fv FV[] F@ F> IF I TO hi THEN
   LOOP
   hi 0= IF 1 ELSE 0 THEN TO nx
   fv FV-LEN 0 ?DO
      I hi <> IF  I fv FV[] F@  nx fv FV[] F@ F> IF I TO nx THEN  THEN
   LOOP
   lo hi nx ;

\ The centroid of all rows but hi.
: (NM-CENTROID) ( m hi cv -- )
   {: m hi cv :}
   m MAT-COLS 0 ?DO
      0e
      m MAT-ROWS 0 ?DO  I hi <> IF  I J m MAT[] F@ F+  THEN  LOOP
      m MAT-ROWS 1- S>F F/  I cv FV[] F!
   LOOP ;

\ The point c + t (x_hi - c) in tv, with t in element 2 of the scratch vector s, and the
\ function value there: t = -1 reflects, -2 expands, -0.5 and 0.5 contract.
: (NM-TRY) ( m hi cv tv xt s -- ) ( F: -- f )
   {: m hi cv tv xt s :}
   cv FV-LEN 0 ?DO
      hi I m MAT[] F@  I cv FV[] F@  F-  2 s (S@) F*  I cv FV[] F@ F+  I tv FV[] F!
   LOOP
   tv xt EXECUTE (NOT-NAN) ;

\ Moves every point halfway to the best one and evaluates them again.
: (NM-SHRINK) ( m lo fv tv xt -- )
   {: m lo fv tv xt :}
   m MAT-ROWS 0 ?DO
      I lo <> IF
         m MAT-COLS 0 ?DO
            lo I m MAT[] F@  J I m MAT[] F@ F+  0.5e F*  J I m MAT[] F!
         LOOP
         m I tv (NM-GET)
         tv xt EXECUTE (NOT-NAN) I fv FV[] F!
      THEN
   LOOP ;

\ Minimizes the function xt, with the effect ( v -- ) ( F: -- y ) for a vector v of n floats,
\ by the Nelder-Mead simplex method. x is the start and changes in place to the best point;
\ step is the size of the first simplex (each coordinate in turn is moved by it). Ends when
\ the values at the points differ by at most tol * (1 + |best|). Leaves the best value.
: NELDER-MEAD ( x xt -- ) ( F: step tol -- fmin )
   {: x xt | n m fv tv rv cv s lo hi nx it :}
   x FV-LEN TO n
   n 1 < IF -24 THROW THEN
   n 1+ n MAT-NEW TO m
   n 1+ FV-NEW TO fv
   n FV-NEW TO tv  n FV-NEW TO rv  n FV-NEW TO cv
   5 FV-NEW TO s                       \ 0 step, 1 tol, 2 t, 3 f(reflected)
   1 s (S!)  0 s (S!)
   0 s (S@) F0= IF -24 THROW THEN
   n 1+ 0 ?DO
      x tv (V-COPY)
      I 0> IF  I 1- tv FV[] DUP F@ 0 s (S@) F+ F!  THEN
      tv m I (NM-PUT)
      tv xt EXECUTE (NOT-NAN) I fv FV[] F!
   LOOP
   BEGIN
      fv (NM-RANK) TO nx TO hi TO lo
      hi fv FV[] F@  lo fv FV[] F@ F-
      lo fv FV[] F@ FABS 1e F+  1 s (S@) F*
      (F<=)
      DUP 0= IF
         DROP
         it 1000 n * 1000 + >= IF -264 THROW THEN
         it 1+ TO it
         m hi cv (NM-CENTROID)
         -1e 2 s (S!)  m hi cv tv xt s (NM-TRY) 3 s (S!)
         tv rv (V-COPY)
         3 s (S@)  lo fv FV[] F@ F< IF
            -2e 2 s (S!)  m hi cv tv xt s (NM-TRY)
            FDUP 3 s (S@) F< IF
               tv m hi (NM-PUT)  hi fv FV[] F!
            ELSE
               FDROP  rv m hi (NM-PUT)  3 s (S@) hi fv FV[] F!
            THEN
         ELSE
            3 s (S@)  nx fv FV[] F@ F< IF
               rv m hi (NM-PUT)  3 s (S@) hi fv FV[] F!
            ELSE
               3 s (S@)  hi fv FV[] F@ F< IF -0.5e ELSE 0.5e THEN 2 s (S!)
               m hi cv tv xt s (NM-TRY)
               FDUP  3 s (S@) hi fv FV[] F@ FMIN  F< IF
                  tv m hi (NM-PUT)  hi fv FV[] F!
               ELSE
                  FDROP  m lo fv tv xt (NM-SHRINK)
               THEN
            THEN
         THEN
         FALSE
      THEN
   UNTIL
   m lo x (NM-GET)
   lo fv FV[] F@
   m FREE THROW  fv FREE THROW  tv FREE THROW  rv FREE THROW  cv FREE THROW  s FREE THROW ;

\ --- Levenberg-Marquardt ---
\ Nonlinear least squares. The program passes a word with the effect ( p r -- ) that fills
\ the vector r of m residuals (the model minus the data) for the parameter vector p. The
\ Jacobian comes from central differences.

\ The Jacobian at p in jm (m by n), by central differences; rp, rm hold m residuals and sc
\ two floats.
: (LM-JAC) ( p xt jm rp rm sc -- )
   {: p xt jm rp rm sc :}
   p FV-LEN 0 ?DO
      I p FV[] F@  0 sc (S!)
      0 sc (S@) FABS 1e FMAX 6e-6 F*  1 sc (S!)
      0 sc (S@) 1 sc (S@) F+  I p FV[] F!  p rp xt EXECUTE
      0 sc (S@) 1 sc (S@) F-  I p FV[] F!  p rm xt EXECUTE
      0 sc (S@)  I p FV[] F!
      rp FV-LEN 0 ?DO
         I rp FV[] F@  I rm FV[] F@ F-  1 sc (S@) 2e F* F/  I J jm MAT[] F!
      LOOP
   LOOP ;

\ Solves (A + lambda diag(A)) d = rhs, with lambda in element 1 of the scratch vector s;
\ dg holds the diagonal of A.
: (LM-STEP) ( a dg rhs s -- d )
   {: a dg rhs s :}
   a MAT-ROWS 0 ?DO
      I dg FV[] F@  FDUP 1e-12 FMAX 1 s (S@) F*  F+  I I a MAT[] F!
   LOOP
   rhs a MAT-SOLVE ;

\ Fits the parameters p (changed in place) so that the sum of the squares of the m residuals
\ of xt is smallest. Ends when a step lowers the sum by at most tol times the sum. Leaves the
\ sum of squares. -264 after 200 steps, or when the residuals are NaN.
: LM-FIT ( p xt m -- ) ( F: tol -- ssr )
   {: p xt m | n r rt pt jm rp rm sc s dg jt a g rhs d it done :}
   p FV-LEN TO n
   n 1 < m 1 < OR IF -24 THROW THEN
   m FV-NEW TO r  m FV-NEW TO rt  m FV-NEW TO rp  m FV-NEW TO rm
   n FV-NEW TO pt  n FV-NEW TO dg  2 FV-NEW TO sc
   m n MAT-NEW TO jm
   5 FV-NEW TO s                       \ 0 tol, 1 lambda, 2 ssr, 3 ssr of the trial
   0 s (S!)
   p r xt EXECUTE  r r FV-DOT (NOT-NAN) 2 s (S!)
   1e-3 1 s (S!)
   FALSE TO done
   BEGIN
      it 200 >= IF -264 THROW THEN
      it 1+ TO it
      p xt jm rp rm sc (LM-JAC)
      jm MAT-T TO jt
      jt jm MAT* TO a
      jt r MAT*V TO g
      g FV-NORM F0= IF
         TRUE TO done
      ELSE
         n 0 ?DO  I I a MAT[] F@  I dg FV[] F!  LOOP
         -1e g FV-SCALE TO rhs
         BEGIN
            a dg rhs s (LM-STEP) TO d
            n 0 ?DO  I p FV[] F@  I d FV[] F@ F+  I pt FV[] F!  LOOP
            d FREE THROW
            pt rt xt EXECUTE
            rt rt FV-DOT 3 s (S!)
            3 s (S@) 2 s (S@) F< IF
               2 s (S@) 3 s (S@) F-  2 s (S@) 0 s (S@) F*  (F<=) TO done
               pt p (V-COPY)  rt r (V-COPY)
               3 s (S@) 2 s (S!)
               1 s (S@) 10e F/ 1e-15 FMAX 1 s (S!)
               TRUE
            ELSE
               1 s (S@) 10e F* 1 s (S!)
               1 s (S@) 1e12 F> IF TRUE TO done TRUE ELSE FALSE THEN
            THEN
         UNTIL
         rhs FREE THROW
      THEN
      jt FREE THROW  a FREE THROW  g FREE THROW
      done
   UNTIL
   2 s (S@)
   r FREE THROW  rt FREE THROW  rp FREE THROW  rm FREE THROW  pt FREE THROW
   dg FREE THROW  sc FREE THROW  jm FREE THROW  s FREE THROW ;

\ The covariance matrix of the fitted parameters p: (J^T J)^-1 times the sum of squares over
\ (m - n), with the Jacobian J at p. The square roots of the diagonal are the standard errors.
\ Needs m above n; -264 when J^T J is singular.
: LM-COVARIANCE ( p xt m -- c )
   {: p xt m | n r jm rp rm sc jt a inv :}
   p FV-LEN TO n
   m n <= IF -24 THROW THEN
   m FV-NEW TO r  m FV-NEW TO rp  m FV-NEW TO rm  2 FV-NEW TO sc
   m n MAT-NEW TO jm
   p r xt EXECUTE
   p xt jm rp rm sc (LM-JAC)
   jm MAT-T TO jt
   jt jm MAT* TO a
   a MAT-INV TO inv
   r r FV-DOT  m n - S>F F/
   inv MAT-SCALE
   r FREE THROW  rp FREE THROW  rm FREE THROW  sc FREE THROW  jm FREE THROW
   jt FREE THROW  a FREE THROW  inv FREE THROW ;
