\ An example of random distributions and Monte Carlo integration with the SCIENCE words; run it with
\   cargo run --manifest-path science/Cargo.toml -- science/examples/random.fs

ALSO SCIENCE
6 SET-PRECISION
2024 SEED!                              \ the same seed gives the same numbers

\ The mean and the variance of n draws of a word with the effect ( F: -- x ).
: STATS ( xt n -- ) ( F: -- mean var )
   {: xt n | s :}
   2 FV-NEW TO s
   n 0 ?DO
      xt EXECUTE  FDUP 0 s FV[] F@ F+ 0 s FV[] F!
      FDUP F* 1 s FV[] F@ F+ 1 s FV[] F!
   LOOP
   0 s FV[] F@ n S>F F/  FDUP
   1 s FV[] F@ n S>F F/  FSWAP FDUP F* F-
   s FREE THROW ;
: SHOW-STATS ( xt -- )  2000 STATS ."  mean " FSWAP F. ."  variance " F. CR ;

\ Heights: normal, mean 175, standard deviation 7.
: HEIGHT  175e 7e RANDOM-NORMAL ;
." height (175, 7)    " ' HEIGHT SHOW-STATS
\ Waiting times: exponential with 4 arrivals a minute.
: WAIT  4e RANDOM-EXPONENTIAL ;
." waiting (rate 4)   " ' WAIT SHOW-STATS
\ A gamma law with shape 2 and scale 3 has mean 6 and variance 18.
: GAMMA-DRAW  2e 3e RANDOM-GAMMA ;
." gamma (2, 3)       " ' GAMMA-DRAW SHOW-STATS
\ Customers in an hour: Poisson with mean 12.
: CUSTOMERS  12e RANDOM-POISSON S>F ;
." poisson (12)       " ' CUSTOMERS SHOW-STATS

\ Heads in 20 tosses of a fair coin, 2000 times, counted per number of heads.
CREATE TALLY 21 CELLS ALLOT  TALLY 21 CELLS ERASE
: TOSS  2000 0 DO  20 0.5e RANDOM-BINOMIAL CELLS TALLY + 1 SWAP +!  LOOP ;
TOSS
: SHOW-TALLY  ." heads:" 21 0 DO  I 8 = I 12 = OR IF I . ." -> " I CELLS TALLY + @ . THEN  LOOP CR ;
SHOW-TALLY

\ The normal distribution: density, cumulative distribution and the quantile that undoes it.
." P(Z <= 1.96) = "  1.96e NORMAL-CDF F.  CR
." 97.5% point = "  0.975e NORMAL-QUANTILE F.  CR

\ Monte Carlo integration: the integral of sqrt(x) from 0 to 1 is 2/3. The standard error
\ shrinks as 1/sqrt(n): a hundred times the points, a tenth of the error.
: ROOT ( F: x -- y )  FSQRT ;
." n = 1000:   " ' ROOT 1000 0e 1e MC-INTEGRATE  ." integral " FSWAP F. ." +- " F. CR
." n = 100000: " ' ROOT 100000 0e 1e MC-INTEGRATE  ." integral " FSWAP F. ." +- " F. CR
