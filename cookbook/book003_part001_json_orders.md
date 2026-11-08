# JSON: read a document, write a summary

*Book 3, Data and tools. Needs the core only (the word list `JSON`).*

The program: [`examples/book003_example001_json_orders.fs`](../examples/book003_example001_json_orders.fs).
Its output is beside it, in `book003_example001_json_orders.out`. Run it with:

```
cargo run --release -- examples/book003_example001_json_orders.fs
```

It reads a small JSON document with the orders of a shop, prints one line for each order
with its total, and writes a summary as JSON again. It takes about 1700 steps (found by
lowering `--max-steps` until it stopped), far inside the 10 million of the playground. In the
playground, pick *JSON orders* under *Examples*.

```
The document is valid.
Shop: Pear Tree
Order 1 for Ada : 8.75
Order 2 for Alan : 15.00
Order 3 for Grace : 0.00  (has a note)
Revenue: 23.75
{"orders":3,"revenue":2375,"customers":["Ada","Alan","Grace"]}
0 0
```

## A JSON value is its text

`ALSO JSON` makes the words visible. There is no tree in memory and no parser to start. A
JSON value is its text, two cells `( c-addr u )`, and the reading words give back *pieces of
that text*: the value of a key, the nth element of an array. Nothing is copied, so reading
is cheap and does not use the heap.

```forth
: ORDERS ( -- json )  DOC S" orders" JSON-GET DROP ;
```

`JSON-GET` takes a key and returns the value and a flag. `JSON-NTH` does the same for an
array, `JSON-LENGTH` counts the elements or keys, and `JSON@` follows a whole path such as
`orders[1].customer` in one go. The path reads like the document:

```forth
: NUM ( json c-addr u -- n )  JSON@ DROP JSON>NUMBER DROP ;
```

`JSON>NUMBER` turns a number into an integer. For a number with a fraction use
`JSON>FLOAT`, which leaves the value on the float stack.

## A missing key is a flag, not an error

Every reading word that can fail returns a flag. A missing key, or a value of the wrong
kind, gives 0 and leaves nothing to use; it never throws. So a program has two ways, and
the example shows both:

- Where the document is known to have the key, `DROP` the flag (`NUM` does that).
- Where the key is optional, use it. Only order 3 has a `note`:

```forth
oa ou S" note" JSON-GET IF  2DROP ."   (has a note)"  ELSE  2DROP  THEN CR
```

A value of `null` is a value: `JSON-GET` finds the key (flag -1), and `JSON-TYPE` says 0 for
null. The last line of the output shows the other case: `{oops` is not valid, so
`JSON-VALID?` gives 0 and `JSON@` finds nothing.

## Walking an array

An array is walked with its length and `JSON-NTH`. A JSON value takes two cells, so a local
that holds one needs two names (`{: ta tu :}`):

```forth
: ORDER-TOTAL ( order -- cents )
  S" items" JSON-GET DROP  {: ia iu :}
  0  ia iu JSON-LENGTH 0 ?DO
    ia iu I JSON-NTH DROP  {: ta tu :}
    ta tu S" qty" NUM  ta tu S" price" NUM  * +
  LOOP ;
```

The sum starts at 0 under the loop and grows by quantity times price for each item. Order 3
has an empty array, and `0 ?DO` skips the loop: its total is 0.

The prices in the document are whole cents (`150`, `425`), so every sum is exact.
`.CENTS` prints a number of cents with a point, with `<# # # [CHAR] . HOLD #S #>`. When prices
have a fraction in the document, read them with `JSON>FLOAT` and convert them with `F>FX`
(word list `FIXED`), since a binary float cannot hold `0.1` exactly.

## Strings are copies

A string is the one place where reading allocates: `JSON>STRING` gives an `ALLOCATE`d copy,
because the text in the document has escapes (`\n`, `é`) that must be unfolded. So the
word that uses it frees it:

```forth
: .CUSTOMER ( order -- )
  S" customer" JSON@ DROP JSON>STRING DROP  2DUP TYPE  DROP FREE THROW ;
```

## Writing

Writing goes to the output, or, between `<JSON` and `JSON>`, to a string that is returned
as `( c-addr u )` and `ALLOCATE`d like any other. The commas and quotes come by themselves:

```forth
<JSON  JSON{
  S" orders"  JSON-KEY  ORDERS JSON-LENGTH JSON-NUMBER
  S" revenue" JSON-KEY  REVENUE JSON-NUMBER
  S" customers" JSON-KEY  JSON[  ...  JSON]
JSON}  JSON>
```

`JSON-KEY` puts in a key, then one of `JSON-NUMBER`, `JSON-STRING`, `JSON-FLOAT`,
`JSON-FLAG` or `JSON-NULL` puts in a value, and `JSON{ JSON}` and `JSON[ JSON]` nest. In
`SUMMARY` the customers are read from the document and written straight into the array.
`JSON-RAW` puts in a value that is JSON already.

## Try it

- Add an `"email"` to an order and print it only when it is there.
- Count the items sold per `sku` with a dictionary (`ALSO COLLECTIONS`) and write the
  counts as an object, with `DICT-EACH` and a word that does `JSON-KEY` and `JSON-NUMBER`.
- A document larger than 1024 characters cannot be written as a string in the source (a
  line of code is at most that long). The `forthling` program reads a file with `SLURP-FILE`,
  and the MCP server gives it as the `data` argument and the word `DATA`.
