\ book003_example007_http_json.fs - ask a JSON API a question and read the answer.
\ Run: cargo run --release --manifest-path http/Cargo.toml -- --allow-host api.github.com http/examples/book003_example007_http_json.fs
\ Needs the program forthling-http (the word list HTTP) and a network. The tests do not run it,
\ because the answer comes from a server we do not control.
\
\ How it works:
\   1. FETCH sets two headers (GitHub wants a User-Agent) and sends a GET.
\   2. The status is a number the program looks at: 200 is an answer, 404 is "not there".
\   3. The body is JSON; JSON@ takes a field out of it by its path, JSON>STRING reads it.
\   4. Every body is ALLOCATEd, so the program FREEs it when it is done.
\   5. A host that was not allowed with --allow-host throws -259; CATCH turns that into a message.

ALSO HTTP  ALSO JSON

: FETCH ( url u -- body u status )
  S" User-Agent" S" forthling-cookbook" HTTP-HEADER
  S" Accept" S" application/vnd.github+json" HTTP-HEADER
  HTTP-GET ;

\ Types the text at a path of a JSON value, followed by a space.
: .FIELD ( json u path u -- )
  JSON@ DROP JSON>STRING DROP  2DUP TYPE DROP FREE THROW  SPACE ;

: REPO-URL ( -- url u )  S" https://api.github.com/repos/DeSchaker/forthling" ;

: SHOW-REPO ( -- )
  REPO-URL FETCH ( body u status )
  DUP 200 <> IF  ." status " . DROP FREE THROW CR EXIT  THEN
  DROP
  2DUP S" full_name" .FIELD
  2DUP S" language" .FIELD
  2DUP S" license.spdx_id" .FIELD
  2DUP S" default_branch" .FIELD  CR
  DROP FREE THROW ;

\ A repository that is not there: the status says so, the program does not stop.
: SHOW-MISSING ( -- )
  S" https://api.github.com/repos/DeSchaker/no-such-repo" FETCH ( body u status )
  ." status " .  2DUP S" message" .FIELD  DROP FREE THROW CR ;

\ A header of the latest response.
: SHOW-TYPE ( -- )
  REPO-URL FETCH ( body u status )  DROP DROP FREE THROW
  S" content-type" HTTP-RESPONSE-HEADER IF  2DUP TYPE DROP FREE THROW  ELSE ." (none)" THEN CR ;

\ A host the program was not allowed to reach.
: TRY-OTHER ( -- )
  S" https://example.org/" HTTP-GET ( body u status )  DROP DROP FREE THROW ;

SHOW-REPO
SHOW-MISSING
SHOW-TYPE
' TRY-OTHER CATCH ." refused with " . CR
