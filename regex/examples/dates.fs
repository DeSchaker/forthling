\ An example of the REGEX words; run it with
\   cargo run --manifest-path regex/Cargo.toml -- regex/examples/dates.fs

ALSO REGEX

\ Is the whole text a date such as 2026-11-01?
: DATE? ( c-addr u -- flag )  S" ^\d{4}-\d{2}-\d{2}$" RE-MATCH? ;
S" 2026-11-01" DATE? .
S" 1 November" DATE? .  CR

\ How many numbers are in a text?
: NUMBERS ( c-addr u -- n )  S" \d+" RE-COUNT ;
S" 3 apples, 12 pears and 7 plums" NUMBERS . CR

\ Part n of a date in a text, from the groups of the pattern.
: DATE-PATTERN ( -- c-addr u )  S" (\d+)-(\d+)-(\d+)" ;
: PART ( c-addr u n -- )
  >R 2DUP DATE-PATTERN R> RE-GROUP
  IF  ( c-addr u start len ) >R NIP + R> TYPE  ELSE  2DROP  THEN ;
: DATE. ( c-addr u -- )  2DUP 3 PART ." /" 2DUP 2 PART ." /" 1 PART ;
S" shipped on 2026-11-01." DATE. CR

\ Another order: the replacement names the groups. The new text is ALLOCATEd.
: SWAP-DATE ( c-addr u -- c-addr' u' )  DATE-PATTERN S" $3/$2/$1" RE-REPLACE ;
S" 2026-11-01" SWAP-DATE 2DUP TYPE CR  DROP FREE THROW

\ A bad pattern throws -262.
: BAD ( -- )  S" a" S" (" RE-MATCH? . ;
' BAD CATCH . CR
