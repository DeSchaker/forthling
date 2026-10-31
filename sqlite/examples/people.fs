\ An example of the SQLITE words; run it with
\   cargo run --manifest-path sqlite/Cargo.toml -- sqlite/examples/people.fs

ALSO SQLITE

\ THROWs a nonzero ior after showing the message of SQLite.
: ?DB ( ior -- )
  DUP IF  DB-ERROR 2DUP TYPE CR DROP FREE DROP  THEN THROW ;

S" :memory:" DB-OPEN ?DB VALUE DB

S" CREATE TABLE people (name TEXT, born INTEGER)" DB DB-EXEC ?DB

S" INSERT INTO people VALUES (?1, ?2)" DB DB-PREPARE ?DB VALUE INSERT
: PERSON ( c-addr u year -- )
  2 INSERT DB-BIND ?DB  1 INSERT DB-BIND-TEXT ?DB
  INSERT DB-STEP ?DB DROP ;
S" Ada Lovelace" 1815 PERSON
S" Alan Turing" 1912 PERSON
S" Grace Hopper" 1906 PERSON
INSERT DB-FINALIZE ?DB

S" SELECT name, born FROM people WHERE born > ?1 ORDER BY born" DB DB-PREPARE ?DB VALUE QUERY
: BORN-AFTER ( year -- )
  1 QUERY DB-BIND ?DB
  BEGIN  QUERY DB-STEP ?DB  WHILE
    1 QUERY DB-COLUMN-INT .  0 QUERY DB-COLUMN 2DUP TYPE CR DROP FREE THROW
  REPEAT ;
.( Born after 1900:) CR  1900 BORN-AFTER
QUERY DB-FINALIZE ?DB
DB DB-CLOSE ?DB
