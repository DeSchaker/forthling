\ Fetches a page and shows its status, its size and its content type. Run it with:
\   forthling-http --allow-host example.com http/examples/page.fs
ALSO HTTP

: SHOW-PAGE ( -- )
  S" https://example.com/" HTTP-GET ( body u status )
  . . ." bytes, "
  S" content-type" HTTP-RESPONSE-HEADER IF 2DUP TYPE DROP FREE THROW THEN
  CR  FREE THROW ;

SHOW-PAGE
