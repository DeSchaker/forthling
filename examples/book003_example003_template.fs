\ book003_example003_template.fs - fill in a text template with SUBSTITUTE and print letters.
\ Run: cargo run --release -- examples/book003_example003_template.fs
\ Needs the core only; it also runs in the playground.
\
\ How it works:
\   1. A template is a text with %name% marks. REPLACES gives a name its text,
\      SUBSTITUTE copies the template into a buffer and puts the texts in place of the marks.
\   2. A number becomes text with <# ... #>, so a price in cents prints as 12,50.
\   3. %% in a template is one % in the result. A mark that has no text is kept as it is.
\   4. Text that comes from a user must not become part of a template as it is: UNESCAPE
\      doubles each % in it, so that it comes out the way it was typed.

: TEMPLATE ( -- c-addr u )
  S\" Dear %name%,\n\nThank you for your order of %qty% x %item%.\nThe total is %total% euro, with a 5%% discount already taken off.\nYour order number is %number%.\n\nKind regards,\n%shop%\n" ;

CREATE OUT 400 ALLOT        \ where SUBSTITUTE puts the result
CREATE TBUF 400 ALLOT       \ a template that the program builds
VARIABLE TLEN

: EUR ( cents -- c-addr u )  S>D <# # # [CHAR] , HOLD #S #> ;
: NUM ( n -- c-addr u )  S>D <# #S #> ;

: SHOW-RESULT ( c-addr u n -- )
  DUP 0< IF  ." (" . ." : the result does not fit)" 2DROP  ELSE  DROP TYPE  THEN ;

: LETTER ( name-a name-u item-a item-u qty cents number -- )
  {: na nu ia iu qty cents number :}
  na nu S" name" REPLACES
  ia iu S" item" REPLACES
  qty NUM S" qty" REPLACES
  qty cents * EUR S" total" REPLACES
  number NUM S" number" REPLACES
  TEMPLATE OUT 400 SUBSTITUTE SHOW-RESULT ;

S" The Forth Shop" S" shop" REPLACES

S" Alice" S" brass bell" 2 1250 1001 LETTER
CR S" Bob" S" ink pot" 10 395 1002 LETTER

\ A mark without a text stays in the result, so a typing error is easy to see.
CR S" Fill in %name% and %nmae%." OUT 400 SUBSTITUTE
>R TYPE  ."  (" R> . ." replaced)" CR

\ A note typed by a customer goes at the end of a template. Copied as it is, the note could
\ use a mark itself. UNESCAPE makes it text.
: NOTE ( -- c-addr u )  S" Note: ring twice, %shop% is a secret." ;

: BUILD ( escape? -- c-addr u )
  S" Hello %name%. " TBUF SWAP CMOVE  14 TLEN !
  IF  NOTE TBUF TLEN @ + UNESCAPE NIP
  ELSE  NOTE TBUF TLEN @ + SWAP CMOVE  NOTE NIP  THEN
  TLEN @ + TBUF SWAP ;

CR ." As it is:    "  0 BUILD OUT 400 SUBSTITUTE SHOW-RESULT CR
   ." Unescaped:   " -1 BUILD OUT 400 SUBSTITUTE SHOW-RESULT CR
