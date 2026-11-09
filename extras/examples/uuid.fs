\ An example of the UUID word; run it with
\   cargo run --manifest-path extras/Cargo.toml -- extras/examples/uuid.fs

ALSO UUID

\ Show a text that a word gave, and free it (the words ALLOCATE their answer).
: SHOW ( c-addr u -- )  2DUP TYPE DROP FREE THROW ;

\ With a seed the run is repeatable; without one, every run differs.
42 SEED!

: IDS ( n -- )  0 DO  UUID4 SHOW CR  LOOP ;
3 IDS
