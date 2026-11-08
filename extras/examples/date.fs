\ An example of the DATE words; run it with
\   cargo run --manifest-path extras/Cargo.toml -- extras/examples/date.fs

ALSO DATE

\ Show a text that a word gave, and free it (the words ALLOCATE their answer).
: SHOW ( c-addr u -- )  2DUP TYPE DROP FREE THROW ;

\ The name of a day: 1 is Monday.
: DAY-NAME ( n -- c-addr u )  1- 3 * S" MonTueWedThuFriSatSun" DROP + 3 ;

\ A time from text with a zone offset: +02:00 is two hours ahead of UTC, so the answer is in UTC.
S" 2026-11-08T21:30:00+02:00" ISO8601>
DUP >ISO8601 SHOW CR
DUP DAY-OF-WEEK DAY-NAME TYPE CR

\ Add days and months. A month that is too short loses the extra days.
DUP 30 +DAYS >ISO8601 SHOW CR
S" 2026-01-31" ISO8601> 1 +MONTHS >ISO8601 SHOW CR
S" 2024-01-31" ISO8601> 1 +MONTHS >ISO8601 SHOW CR

\ The parts of a time, in the order of TIME&DATE: the year is on top, so it prints first.
SECONDS>DATE  . . . . . .  CR

\ A calendar of a month; the weekday of the first day sets the indent.
: FIRST-DAY ( year month -- n )  {: y m :}  0 0 0 1 m y DATE>SECONDS DAY-OF-WEEK 1- ;
: CALENDAR ( year month -- )
  2DUP FIRST-DAY                       \ ( year month first )
  ."  Mo Tu We Th Fr Sa Su" CR
  DUP 3 * SPACES
  >R DAYS-IN-MONTH 1+ 1 DO
    I 3 .R
    R@ I + 7 MOD 0= IF CR THEN
  LOOP R> DROP CR ;
2026 11 CALENDAR
2024 2 CALENDAR
