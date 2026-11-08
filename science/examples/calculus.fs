\ An example of integration and differential equations with the SCIENCE words; run it with
\   cargo run --manifest-path science/Cargo.toml -- science/examples/calculus.fs

ALSO SCIENCE
6 SET-PRECISION

\ A function is passed as an execution token; it has the effect ( F: x -- y ).
\ The standard normal density:
: DENSITY ( F: x -- y )  FDUP F* -2e F/ FEXP  2e 3.141592653589793e F* FSQRT F/ ;

\ The chance that a standard normal variable lies within 1.96 of 0.
-1.96e 1.96e ' DENSITY 1e-10 INTEGRATE
." P(|Z| < 1.96) = "  FSWAP F.  CR
." error estimate below 1e-9: "  1e-9 F< .  CR

\ The same integral with Simpson's rule and Romberg's method:
." Simpson, 20 panels: "  -1.96e 1.96e ' DENSITY 20 SIMPSON F.  CR
-1.96e 1.96e ' DENSITY 1e-10 ROMBERG FDROP
." Romberg: "  F.  CR

\ A function that is not smooth: the area under sqrt(x) from 0 to 1 is 2/3.
0e 1e ' FSQRT 1e-8 INTEGRATE FDROP
." sqrt: "  F.  CR

\ A double integral: x * y over the unit square is 1/4. The function that the outer
\ integral calls integrates again.
FVARIABLE X
: INNER ( F: y -- x*y )  X F@ F* ;
: OUTER ( F: x -- f )  X F!  0e 1e ['] INNER 1e-10 INTEGRATE FDROP ;
0e 1e ' OUTER 1e-9 INTEGRATE FDROP
." double integral: "  F.  CR

\ A differential equation: logistic growth y' = r y (1 - y / K) with r = 0.5 and K = 100.
\ The state y is a float vector; the derivative fills dy.
: GROWTH ( y dy -- ) ( F: t -- )
   FDROP  SWAP 0 SWAP FV[] F@
   FDUP 100e F/ 1e FSWAP F- F* 0.5e F*
   0 SWAP FV[] F! ;
\ The exact solution, with y(0) = 5.
: EXACT ( F: t -- y )  -0.5e F* FEXP 19e F* 1e F+ 100e FSWAP F/ ;

5e 1 FV-FROM-STACK
DUP ' GROWTH 0e 10e 1e-9 ODE-RKF
." y(10) with ODE-RKF: "  DUP 0 SWAP FV[] F@ F.  CR
." y(10) exactly: "  10e EXACT F.  CR
FREE THROW

\ Runge-Kutta of order 4 with a fixed step: 100 steps.
5e 1 FV-FROM-STACK
DUP ' GROWTH 100 0e 10e RK4
." y(10) with RK4: "  DUP 0 SWAP FV[] F@ F.  CR
FREE THROW
