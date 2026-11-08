\ An example of the special functions of the SCIENCE words; run it with
\   cargo run --manifest-path science/Cargo.toml -- science/examples/special_functions.fs

ALSO SCIENCE
6 SET-PRECISION

\ The normal distribution: the chance that a standard normal variable is at most x.
: NORMAL-CDF ( F: x -- p )  2e FSQRT F/ ERF 1e F+ 2e F/ ;

." P(Z <= 1.96) = "  1.96e NORMAL-CDF F.  CR
." P(|Z| > 1.96) = "  1.96e 2e FSQRT F/ ERFC F.  CR

\ The chi-square distribution with k degrees of freedom is a gamma distribution
\ with shape k/2 and scale 2, so its survival function is Q(k/2, x/2).
: CHI-SQUARE-P ( F: x k -- p )  2e F/ FSWAP 2e F/ GAMMA-Q ;
\ The famous 5% point for 1 degree of freedom is 3.841.
." chi-square, 1 degree, 3.841: p = "  3.841e 1e CHI-SQUARE-P F.  CR
." chi-square, 10 degrees, 18.307: p = "  18.307e 10e CHI-SQUARE-P F.  CR

\ The binomial distribution: the chance of at most k successes in n trials with
\ chance p each is I(1-p; n-k, k+1), the incomplete beta function.
: BINOMIAL-CDF ( k n -- ) ( F: p -- cdf )
   {: k n :}  1e FSWAP F-  n k - S>F  k 1+ S>F  FROT BETA-I ;
\ 14 or fewer heads in 40 tosses of a fair coin:
." P(heads <= 14 of 40) = "  14 40 0.5e BINOMIAL-CDF F.  CR

\ Gamma extends the factorial; LNGAMMA works where a factorial overflows.
." 5! = "  5 FACTORIAL F.  CR
." Gamma(1/2)^2 = "  0.5e GAMMA FDUP F* F.  CR
." ln(1000!) = "  1001e LNGAMMA F.  CR

\ The logistic function and its slope.
." logistic(0), logistic(2): "  0e LOGISTIC F.  2e LOGISTIC F.  CR
." slope at 0: "  0e LOGISTIC-D F.  CR
