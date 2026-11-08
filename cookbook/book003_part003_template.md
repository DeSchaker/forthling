# Template: fill in a letter with SUBSTITUTE

*Book 3, Data and tools. Needs the core only.*

The program: [`examples/book003_example003_template.fs`](../examples/book003_example003_template.fs).
Its output is beside it, in `book003_example003_template.out`. Run it with:

```
cargo run --release -- examples/book003_example003_template.fs
```

It prints two letters from one template, shows what happens with a mark that has no text,
and shows how to put text from a user into a template safely. It takes about 560 steps
(found by lowering `--max-steps` until it stopped), far inside the 10 million of the
playground. In the playground, pick *Template* under *Examples*. The first letter:

```
Dear Alice,

Thank you for your order of 2 x brass bell.
The total is 25,00 euro, with a 5% discount already taken off.
Your order number is 1001.

Kind regards,
The Forth Shop
```

## Marks and texts

A template is a text with marks between percent signs, such as `%name%`. Two words from the
core do the work:

- `REPLACES ( text-a text-u name-a name-u -- )` gives a name its text. Names do not depend
  on case, and the text is copied, so the program may change its own buffers afterwards.
- `SUBSTITUTE ( src-a src-u dest-a dest-u -- dest-a u n )` copies the source into the
  destination buffer and puts the texts in place of the marks. `u` is the length of the
  result and `n` is how many marks were replaced.

```forth
na nu S" name" REPLACES
...
TEMPLATE OUT 400 SUBSTITUTE SHOW-RESULT
```

The template here comes from `S\"`, which accepts `\n` for a new line. The destination
`OUT` is a buffer of 400 bytes made with `CREATE OUT 400 ALLOT`.

## Numbers are texts too

A text can only be a text, so a number goes through the number-formatting words first.
`NUM` gives a count, and `EUR` gives a price in cents with a comma:

```forth
: EUR ( cents -- c-addr u )  S>D <# # # [CHAR] , HOLD #S #> ;
: NUM ( n -- c-addr u )  S>D <# #S #> ;
```

`#` and `#S` make the digits from the right, and `HOLD` puts a character between them. The
price is kept in whole cents, so nothing is lost (see chapter 1). `2 x 1250` becomes
`25,00`.

## Two things about the marks

- `%%` in a template is one `%` in the result: `5%% discount` prints as `5% discount`.
- A mark that has no text is kept as it is, with its percent signs. The result is
  `Fill in Bob and %nmae%. (1 replaced)`, so a typing error in a name shows in the output
  and `n` tells that not all marks were found.

If the result does not fit in the buffer, `SUBSTITUTE` leaves the buffer alone and returns
`-78` as `n`. The program checks that in `SHOW-RESULT`, so a letter that is too long gives a
message and not half a letter.

## Text from a user

What if the text of a customer becomes part of a template? A customer wrote
`Note: ring twice, %shop% is a secret.`, and the program puts that after `Hello %name%. `
and runs `SUBSTITUTE`. The customer's `%shop%` is then a mark like any other:

```
As it is:    Hello Bob. Note: ring twice, The Forth Shop is a secret.
Unescaped:   Hello Bob. Note: ring twice, %shop% is a secret.
```

`UNESCAPE ( src-a src-u dest-a -- dest-a u )` doubles each `%` while it copies, so
`SUBSTITUTE` turns it back into one `%` and the note comes out the way it was typed. Use it
for every text you do not control. (A text that is the *value* of a mark is never read for
marks again, so `REPLACES` needs no `UNESCAPE`.)

## Two things to know

- `REPLACES` and `SUBSTITUTE` take their names from the stack, so a helper such as
  `: SET PARSE-NAME REPLACES ;` does not work inside a definition: `PARSE-NAME` reads the
  input when the word *runs*, and the program stops with `-13`. Write `S" name" REPLACES`.
- The marks stay until you change them. The program sets `%shop%` once and the other marks
  for each letter.

## Try it

- Add a mark `%date%` and give it a fixed text with `REPLACES`.
- Read the orders from JSON (chapter 1) and print a letter for each.
- Put the template in a text file and read it with `SLURP-FILE` in the `forthling` program.
- Make the letters one after another into a bigger buffer, and see what `-78` does.
