\ book003_example001_json_orders.fs - read a JSON document, print a report, write a summary as JSON.
\ Run: cargo run --release -- examples/book003_example001_json_orders.fs
\ Needs the core only (the word list JSON); it also runs in the playground.
\
\ How it works:
\   1. A JSON value is its text, two cells ( c-addr u ). JSON-GET and JSON-NTH give the part
\      of the text that is a key or an element, so nothing is copied; JSON@ takes a path.
\   2. A missing key gives the flag 0, not an error. Where the key is known to be there, the
\      flag is DROPped; for "note", which only one order has, the program uses the flag.
\   3. JSON>STRING gives an ALLOCATEd copy of a string, so it is FREEd after use.
\   4. Prices are whole cents, so every sum is exact. .CENTS prints them with a point.
\   5. <JSON ... JSON> writes JSON into a string; JSON{ JSON[ and JSON-KEY put in the commas.

ALSO JSON

\ The document. Prices are in cents, so every sum is exact.
: DOC ( -- json )
  S\" {\"shop\":\"Pear Tree\",\"orders\":[{\"id\":1,\"customer\":\"Ada\",\"items\":[{\"sku\":\"pen\",\"qty\":3,\"price\":150},{\"sku\":\"ink\",\"qty\":1,\"price\":425}]},{\"id\":2,\"customer\":\"Alan\",\"items\":[{\"sku\":\"pen\",\"qty\":10,\"price\":150}]},{\"id\":3,\"customer\":\"Grace\",\"items\":[],\"note\":null}]}" ;

\ Reading. A missing key gives the flag 0, so ask for the flag and use it, or DROP it
\ where the document is known to have the key.
: ORDERS ( -- json )  DOC S" orders" JSON-GET DROP ;
: NUM ( json c-addr u -- n )  JSON@ DROP JSON>NUMBER DROP ;

\ What an order costs: the sum of quantity times price over its items.
: ORDER-TOTAL ( order -- cents )
  S" items" JSON-GET DROP  {: ia iu :}
  0  ia iu JSON-LENGTH 0 ?DO
    ia iu I JSON-NTH DROP  {: ta tu :}
    ta tu S" qty" NUM  ta tu S" price" NUM  * +
  LOOP ;

: .CENTS ( n -- )  0 <# # # [CHAR] . HOLD #S #> TYPE ;

: .CUSTOMER ( order -- )   \ JSON>STRING gives an ALLOCATEd string: FREE it
  S" customer" JSON@ DROP JSON>STRING DROP  2DUP TYPE  DROP FREE THROW ;

: .ORDER ( order -- )  {: oa ou :}
  ." Order " oa ou S" id" NUM .  ." for " oa ou .CUSTOMER
  ."  : " oa ou ORDER-TOTAL .CENTS
  oa ou S" note" JSON-GET IF  2DROP ."   (has a note)"  ELSE  2DROP  THEN CR ;

: REVENUE ( -- cents )
  0  ORDERS JSON-LENGTH 0 ?DO  ORDERS I JSON-NTH DROP ORDER-TOTAL +  LOOP ;

\ Writing. Between <JSON and JSON> the output goes to a string; the commas come by
\ themselves.
: SUMMARY ( -- c-addr u )
  <JSON  JSON{
    S" orders"  JSON-KEY  ORDERS JSON-LENGTH JSON-NUMBER
    S" revenue" JSON-KEY  REVENUE JSON-NUMBER
    S" customers" JSON-KEY  JSON[
      ORDERS JSON-LENGTH 0 ?DO
        ORDERS I JSON-NTH DROP  S" customer" JSON@ DROP JSON>STRING DROP
        2DUP JSON-STRING  DROP FREE THROW
      LOOP
    JSON]
  JSON}  JSON> ;

: MAIN ( -- )
  DOC JSON-VALID? IF  ." The document is valid." ELSE  ." The document is not valid."  THEN CR
  DOC S" shop" JSON@ DROP JSON>STRING DROP  ." Shop: " 2DUP TYPE CR  DROP FREE THROW
  ORDERS JSON-LENGTH 0 ?DO  ORDERS I JSON-NTH DROP .ORDER  LOOP
  ." Revenue: " REVENUE .CENTS CR
  SUMMARY  2DUP TYPE CR  DROP FREE THROW
  \ Text that is not JSON is not valid, and nothing can be read from it.
  S" {oops" JSON-VALID? .  S" {oops" S" a" JSON@ . 2DROP CR ;
MAIN
