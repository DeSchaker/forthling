\ An example of the CSV words; run it with
\   cargo run --manifest-path extras/Cargo.toml -- extras/examples/csv.fs

ALSO CSV

\ A small price list: a header, a field with a comma, a field with quotes
\ and a field that is empty.
: DATA ( -- c-addr u )
  S\" name,note,price\nPencil,\"Soft, 2B\",0.45\nNotebook,\"A5, \"\"lined\"\"\",2.10\nEraser,,0.30\n" ;

\ Show a field between brackets; CSV>STRING makes "" one quote. The new text is ALLOCATEd.
: SHOW-FIELD ( c-addr u -- )
  CSV>STRING ." [" 2DUP TYPE ." ]" DROP FREE THROW ;

\ Show one record and leave the rest of the text.
: SHOW-RECORD ( c-addr u -- c-addr' u' )
  BEGIN  CSV-FIELD >R SHOW-FIELD R> 0= WHILE  SPACE  REPEAT  CR ;

\ Skip the header, then show a record at a time while there is text.
\ (BEGIN is compile-only, so the loop is in a word.)
: SHOW-ALL ( c-addr u -- )
  CSV-SKIP  BEGIN  DUP WHILE  SHOW-RECORD  REPEAT  2DROP ;
DATA SHOW-ALL

\ Write fields: a field gets quotes only when it needs them.
: WRITE-FIELD ( c-addr u -- )  CSV-QUOTE 2DUP TYPE DROP FREE THROW ;
S" Ruler" WRITE-FIELD  44 EMIT
S" 30 cm, steel" WRITE-FIELD  44 EMIT
S\" 12\" ruler" WRITE-FIELD  CR

\ Another separator: semicolons, as spreadsheets in some countries write them.
59 CSV-SEPARATOR
S" 1,5;2,25" CSV-FIELD DROP TYPE CR  2DROP

\ A quote that is never closed throws -265.
: BAD ( -- )  S\" \"open" CSV-FIELD 2DROP 2DROP DROP ;
' BAD CATCH . CR
