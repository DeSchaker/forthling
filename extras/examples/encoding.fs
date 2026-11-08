\ An example of the ENCODING words; run it with
\   cargo run --manifest-path extras/Cargo.toml -- extras/examples/encoding.fs

ALSO ENCODING

\ Show a text that a word gave, and free it (the words ALLOCATE their answer).
: SHOW ( c-addr u -- )  2DUP TYPE DROP FREE THROW ;

\ A header and a payload as in a JSON Web Token: base64url without padding.
S\" {\"alg\":\"none\"}" >BASE64URL SHOW CR
S\" {\"user\":\"ada\"}" >BASE64URL SHOW CR

\ Base64 both ways, with padding.
S" Forth is fun" >BASE64 2DUP SHOW CR
BASE64> SHOW CR

\ Hex, and the digest of a text: SHA-256 gives 32 bytes, >HEX shows them.
S" Hi!" >HEX SHOW CR
S" abc" SHA-256 >HEX SHOW CR

\ A checksum is a number.
S" 123456789" CRC-32 HEX . DECIMAL CR

\ Bad input throws -265.
: TRY ( c-addr u -- )  ['] BASE64> CATCH IF  2DROP ." refused" ELSE  SHOW  THEN CR ;
S" Zm9v" TRY
S" Zm9" TRY
