\ An example of complex numbers, the FFT and complex roots with the SCIENCE words; run it with
\   cargo run --manifest-path science/Cargo.toml -- science/examples/complex.fs

ALSO SCIENCE
6 SET-PRECISION

\ A complex number is two floats on the float stack: the real part, then the imaginary part.
: Z. ( F: z -- )  FSWAP F. FDUP F0< IF ." - " FABS ELSE ." + " THEN F. ." i" ;

." (1+2i)(3+4i) = "  1e 2e 3e 4e Z* Z. CR
." (1+2i)/(3+4i) = "  1e 2e 3e 4e Z/ Z. CR
." sqrt(-4) = "  -4e 0e ZSQRT Z. CR
." e^(i pi) = "  0e 3.141592653589793e ZEXP Z. CR
." i^i = "  0e 1e 0e 1e Z** Z. CR
." |3+4i| = "  3e 4e ZABS F. CR
." (1+i)^8 = "  1e 1e 8 Z^N Z. CR

\ The roots of a polynomial with real coefficients may be complex. A root is two floats.
." x^2 + 2x + 5: "  1e 2e 5e ZQUADRATIC  Z. ."  ,  " Z. CR
." x^3 - 1: "  1e 0e 0e -1e ZCUBIC  Z. ."  ,  " Z. ."  ,  " Z. CR

\ All the roots of x^4 + 4, from a vector: the real and the imaginary parts come back as two vectors.
: SHOW-ROOTS ( re im -- )  {: re im :}
   re FV-LEN 0 ?DO  I re FV[] F@  I im FV[] F@  Z.  ." ;  "  LOOP CR ;
4e 0e 0e 0e 1e 5 FV-FROM-STACK POLY-ROOTS
." x^4 + 4: "  OVER SWAP SHOW-ROOTS FREE THROW

\ The FFT of a signal of 64 samples with a wave of 5 periods (amplitude 1) and one of 12
\ periods (amplitude 0.5). The spectrum has its peaks at 5 and 12 (N/2 times the amplitude: 32 and 16).
64 CONSTANT POINTS
POINTS FV-NEW CONSTANT RE
POINTS FV-NEW CONSTANT IM
: SAMPLE ( n -- )  {: n :}
   n S>F 2e F* 3.141592653589793e F* POINTS S>F F/ FDUP
   5e F* FCOS  FSWAP 12e F* FCOS 0.5e F* F+  n RE FV[] F! ;
: FILL-SAMPLES ( -- )  POINTS 0 DO I SAMPLE LOOP ;
FILL-SAMPLES
RE IM FFT
RE IM ZV-ABS CONSTANT SPECTRUM
: PEAKS ( -- )  POINTS 2/ 0 DO
      I SPECTRUM FV[] F@ FDUP 1e F> IF  ." bin " I . ." has " F. CR  ELSE FDROP THEN
   LOOP ;
PEAKS

\ The inverse gives the samples back: the first sample was 1 + 0.5 = 1.5.
RE IM IFFT
." sample 0 again: "  0 RE FV[] F@ F. CR
