\ Talks to a small JSON API on this machine. Run it with:
\   forthling-http --allow-host 127.0.0.1 http/examples/items.fs
\ A body is ALLOCATEd: the words below FREE it when they are done with it.
ALSO HTTP  ALSO JSON

: ADD-ITEM ( -- )   \ POST an item; print the status and the id in the answer
  S" http://127.0.0.1:8080/items"
  S\" {\"name\":\"apple\"}"
  S" application/json"
  HTTP-POST ( body u status )  . ." : id "
  2DUP S" id" JSON@ DROP JSON>NUMBER DROP . CR
  DROP FREE THROW ;

: LIST-ITEMS ( -- )   \ GET the list with a token; print the first name
  S" Authorization" S" Bearer secret" HTTP-HEADER
  S" http://127.0.0.1:8080/items" HTTP-GET ( body u status )  . ." : "
  2DUP S" items[0].name" JSON@ DROP JSON>STRING DROP TYPE CR
  DROP FREE THROW ;

ADD-ITEM
LIST-ITEMS
