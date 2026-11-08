\ Random distributions and Monte Carlo integration, in the word list SCIENCE (see lib.rs).
\ They are built on RND of the core, so SEED! makes a run repeatable. Each word uses a fixed
\ number of draws per value, so the same seed gives the same numbers. The words keep their
\ floats in a scratch vector (see calculus.fs) and the integer results go on the data stack.

: (F0>) ( F: x -- ) ( -- flag )  0e F> ;
: (RECIP) ( F: x -- 1/x )  1e FSWAP F/ ;
: (PI) ( F: -- pi )  1e FATAN 4e F* ;

\ A uniform float in the open interval (0, 1): 53 random bits plus a half, over 2^53.
: (U) ( F: -- u )  RND 11 RSHIFT S>F 0.5e F+  9007199254740992e F/ ;

\ --- Draws ---

\ A uniform number in [lo, hi).
: RANDOM-UNIFORM ( F: lo hi -- x )
   FOVER F-  (U) F*  F+ ;

\ A normal number with the mean and the standard deviation, by the Box-Muller transform.
: RANDOM-NORMAL ( F: mean sd -- x )
   (U) FLN -2e F* FSQRT  (U) 2e F* (PI) F* FCOS  F*  F*  F+ ;

\ An exponential number with the rate: the mean is 1/rate.
: RANDOM-EXPONENTIAL ( F: rate -- x )
   FDUP (F0>) 0= IF -24 THROW THEN
   (U) FLN FNEGATE FSWAP F/ ;

\ Marsaglia and Tsang for a shape of at least 1: d = a - 1/3, c = 1/sqrt(9 d); take
\ v = (1 + c x)^3 with x normal, and accept d v when u < 1 - 0.0331 x^4 or
\ ln u < x^2/2 + d (1 - v + ln v). The scratch vector holds 0 d, 1 c, 2 x, 3 v.
: (GAMMA1) ( F: a -- x )
   {: | s :}
   4 FV-NEW TO s
   1e 3e F/ F-  FDUP 0 s (S!)
   9e F* FSQRT (RECIP)  1 s (S!)
   BEGIN
      0e 1e RANDOM-NORMAL  FDUP 2 s (S!)
      1 s (S@) F*  1e F+  FDUP FDUP F* F*  FDUP 3 s (S!)
      (F0>) IF
         (U)  2 s (S@) FDUP F*  FDUP F*  0.0331e F*  1e FSWAP F-  FOVER FSWAP F< IF
            FDROP TRUE
         ELSE
            FLN  2 s (S@) FDUP F* 2e F/
            0 s (S@)  1e 3 s (S@) F-  3 s (S@) FLN F+  F*  F+
            F<
         THEN
      ELSE FALSE THEN
   UNTIL
   0 s (S@) 3 s (S@) F*
   s FREE THROW ;

\ A gamma number with the shape a and the scale (mean = a * scale).
: RANDOM-GAMMA ( F: shape scale -- x )
   FOVER (F0>) 0= IF -24 THROW THEN
   FDUP (F0>) 0= IF -24 THROW THEN
   FSWAP FDUP 1e F< IF
      FDUP 1e F+ (GAMMA1)             \ a x
      (U) FROT (RECIP) F**  F*            \ x u^(1/a)
   ELSE (GAMMA1) THEN
   F* ;

\ A beta number: x / (x + y) for two gamma numbers.
: RANDOM-BETA ( F: a b -- x )
   FDUP (F0>) 0= IF -24 THROW THEN
   FOVER (F0>) 0= IF -24 THROW THEN
   1e RANDOM-GAMMA  FSWAP 1e RANDOM-GAMMA  ( y x )
   FDUP FROT F+ F/ ;

\ A chi-square number with nu degrees of freedom.
: RANDOM-CHI-SQUARE ( F: nu -- x )  2e F/ 2e RANDOM-GAMMA ;

\ A Poisson number, as an integer. Knuth's method multiplies uniform numbers until the
\ product drops below e^-lambda; a large mean is cut into pieces of at most 30, which add up.
: (POISSON30) ( F: lambda -- ) ( -- k )
   FNEGATE FEXP  0                   \ L, k
   1e
   BEGIN  (U) F*  FOVER FOVER F<  WHILE  1+  REPEAT   \ stop when p <= L
   FDROP FDROP ;
: RANDOM-POISSON ( -- k ) ( F: lambda -- )
   {: | k :}
   FDUP F0< IF -24 THROW THEN
   FDUP 1e8 F> IF -24 THROW THEN
   BEGIN  FDUP 30e F>  WHILE
      30e (POISSON30) k + TO k  30e F-
   REPEAT
   (POISSON30) k + ;

\ A binomial number: the successes in n trials of probability p, as an integer.
: RANDOM-BINOMIAL ( n -- k ) ( F: p -- )
   {: n | k :}
   n 0< IF -24 THROW THEN
   FDUP F0< IF -24 THROW THEN
   FDUP 1e F> IF -24 THROW THEN
   n 0 ?DO  FDUP (U) F> IF k 1+ TO k THEN  LOOP
   FDROP k ;

\ --- The normal distribution (mean 0, standard deviation 1) ---

\ The density.
: NORMAL-PDF ( F: x -- f )  FDUP F* -2e F/ FEXP  2e (PI) F* FSQRT F/ ;

\ The cumulative distribution, from erfc so that the tails keep their digits.
: NORMAL-CDF ( F: x -- p )  2e FSQRT FNEGATE F/ ERFC 2e F/ ;

\ The quantile: x with NORMAL-CDF(x) = p, for p between 0 and 1 (else -46). A start from
\ the rational approximation of Abramowitz and Stegun 26.2.23, then three Halley steps.
\ The scratch vector holds 0 p, 1 x, 2 t.
: (QUANTILE) ( F: p -- x )              \ for p < 1/2
   {: | s :}
   3 FV-NEW TO s
   FDUP 0 s (S!)
   FLN -2e F* FSQRT  FDUP 2 s (S!)
   FDUP 0.010328e F* 0.802853e F+ F* 2.515517e F+      \ the numerator
   2 s (S@) FDUP 0.001308e F* 0.189269e F+ F* 1.432788e F+ 2 s (S@) F* 1e F+ F/
   2 s (S@) FSWAP F-  FNEGATE  1 s (S!)
   3 0 DO
      1 s (S@) NORMAL-CDF  0 s (S@) F-
      2e (PI) F* FSQRT F*  1 s (S@) FDUP F* 2e F/ FEXP F*       \ u
      FDUP 1 s (S@) F* 2e F/ 1e F+ F/
      1 s (S@) FSWAP F-  1 s (S!)
   LOOP
   1 s (S@)
   s FREE THROW ;
: NORMAL-QUANTILE ( F: p -- x )
   FDUP (F0>) 0= IF -46 THROW THEN
   FDUP 1e F< 0= IF -46 THROW THEN
   FDUP 0.5e F= IF FDROP 0e EXIT THEN
   FDUP 0.5e F> IF 1e FSWAP F- (QUANTILE) FNEGATE ELSE (QUANTILE) THEN ;

\ --- Monte Carlo integration ---

\ The integral of the function xt, ( F: x -- y ), over [a, b] from n random points: the
\ mean of the function times the width, and the standard error of that estimate (the width
\ times the standard deviation over the square root of n). The error shrinks as 1/sqrt(n).
\ The scratch vector holds 0 a, 1 the width, 2 the mean, 3 the sum of squares of the
\ differences (Welford's running update).
: MC-INTEGRATE ( xt n -- ) ( F: a b -- mean err )
   {: xt n | s :}
   n 2 < IF -24 THROW THEN
   4 FV-NEW TO s
   FOVER F-  1 s (S!)  0 s (S!)
   n 1+ 1 DO
      0 s (S@)  1 s (S@) (U) F* F+  xt EXECUTE        \ y
      FDUP 2 s (S@) F-                                \ y delta
      FDUP I S>F F/ 2 s (S@) F+  2 s (S!)             \ the new mean
      FSWAP 2 s (S@) F-  F*  3 s (S@) F+  3 s (S!)    \ delta * (y - new mean)
   LOOP
   2 s (S@) 1 s (S@) F*
   3 s (S@) n 1- S>F F/  n S>F F/  FSQRT  1 s (S@) F* FABS
   s FREE THROW ;
