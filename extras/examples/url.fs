\ An example of the URL words; run it with
\   cargo run --manifest-path extras/Cargo.toml -- extras/examples/url.fs

ALSO URL

\ Show a text that a word gave, and free it (the words ALLOCATE their answer).
: SHOW ( c-addr u -- )  2DUP TYPE DROP FREE THROW ;

: LINE ( c-addr u label-addr label-u -- )  TYPE ."  = " TYPE CR ;

\ Cut a URL. The parts point into the text; nothing is copied.
: ADDRESS ( -- c-addr u )  S" https://ann@example.com:8443/search/books?q=forth%20in%20rust&page=2#results" ;

ADDRESS URL-SCHEME   S" scheme"   LINE
ADDRESS URL-USERINFO S" userinfo" LINE
ADDRESS URL-HOST     S" host"     LINE
ADDRESS URL-PORT . CR
ADDRESS URL-PATH     S" path"     LINE
ADDRESS URL-QUERY    S" query"    LINE
ADDRESS URL-FRAGMENT S" fragment" LINE

\ A parameter of the query, decoded.
ADDRESS URL-QUERY S" q" URL-PARAM 0= THROW  PERCENT> SHOW CR
ADDRESS URL-QUERY S" page" URL-PARAM 0= THROW  TYPE CR
ADDRESS URL-QUERY S" sort" URL-PARAM . CR

\ Write a query: encode each value, so that & = and spaces cannot break it.
S" q=" TYPE  S" fish & chips" >PERCENT SHOW  S" &lang=" TYPE  S" nl/be" >PERCENT SHOW CR

\ A host with a port that is not a number is refused.
S" http://example.com:80a/" ' URL-HOST CATCH . 2DROP CR
