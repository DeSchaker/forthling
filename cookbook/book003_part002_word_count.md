# Word count: a dictionary and a sort

*Book 3, Data and tools. Needs the core only (the word lists `COLLECTIONS` and `STRINGS`).*

The program: [`examples/book003_example002_word_count.fs`](../examples/book003_example002_word_count.fs).
Its output is beside it, in `book003_example002_word_count.out`. Run it with:

```
cargo run --release -- examples/book003_example002_word_count.fs
```

It counts the words of a short text and prints the five most common ones. It takes about
36 400 steps (found by lowering `--max-steps` until it stopped), far inside the 10 million of
the playground. In the playground, pick *Word count* under *Examples*.

```
30 words, 14 different. The top 5:
  4 a
  4 dog
  4 fox
  4 the
  3 lazy
```

## Cut the text into words

A word is a run of letters and apostrophes (`doesn't` is one word); everything else only
separates words. The text is made lower case first, so `The` and `the` count as one word.
`LOWER` changes a text *in place*, and `S"` gives a text that must not be changed, so the
program works on a copy from `ALLOCATE`:

```forth
TEXT {: ta tu :}
tu ALLOCATE THROW {: buf :}
ta buf tu CMOVE   buf tu LOWER COUNT-WORDS   buf FREE THROW
```

`COUNT-WORDS` walks over the text with an index `i`. It skips what is not a letter. At a
letter it remembers the start `j`, moves `i` to the end of the run, and the word is the
piece from `j` to `i`:

```forth
a j +  i j -  COUNT-WORD
```

Nothing is copied here: the word is two cells `( c-addr u )` that point into the text.

## Count in a dictionary

A dictionary from `ALSO COLLECTIONS` maps a text to a cell. For each word, `DICT@` gives
the old count and a flag, or only a flag when the word is new:

```forth
: COUNT-WORD ( c-addr u -- )
  1 TOTAL +!
  2DUP COUNTS DICT@ IF 1+ ELSE 1 THEN  ROT ROT COUNTS DICT! ;
```

`DICT!` copies the key, so the text may be freed afterwards, and it replaces the old value of
a key that is there. That is why the program can free `buf` right after counting. Keys are
compared byte for byte, which is the reason for `LOWER`.

## Put them in order

A dictionary keeps the keys in the order they were added, so that a program prints the same
output every time. For a top list the words must be in order of count. `SORT-CELLS` sorts
cells with a word of your own, `( a b -- flag )`, true when `a` goes before `b`. The cells
are the numbers `0 .. n-1` of the entries, and the words and the counts wait in vectors
under those numbers. `DICT-EACH` fills them, once for each word:

```forth
: KEEP ( c-addr u n -- )
  NUMS VEC+!  LENS VEC+!  ADDRS VEC+!
  ORDER VEC# ORDER VEC+! ;
```

The order is the one of the sort word `BEFORE?`: more often first, and for the same count
alphabetically with `COMPARE`. That is why `a` comes before `dog`, `fox` and `the`:

```forth
: BEFORE? ( i j -- flag )  {: i j :}
  i NUM@ j NUM@ = IF  i WORD@ j WORD@ COMPARE 0<  ELSE  i NUM@ j NUM@ >  THEN ;
```

Then one line sorts the vector in place, and `TOP` prints the first entries:

```forth
ORDER VEC-DATA ['] BEFORE? SORT-CELLS
```

The words in the vectors point at the keys inside the dictionary, so the dictionary must
not change while they are in use. The sort is stable, so equal entries keep the order they
had.

## Two things to know

- Inside a definition, write `[']`, not `'`. `'` reads the name when the word *runs*, from
  the input, and the program stops with `-16` (a zero-length name).
- Locals can be changed with `TO` (`i 1+ TO i`), which is what a loop with an index needs.

## Try it

- Print every word, not only the top five (`ORDER VEC# TOP`).
- Ignore very short words: skip a word of fewer than three letters in `COUNT-WORD`.
- Write the counts as JSON with `JSON{ ... JSON}` and `JSON-KEY` / `JSON-NUMBER` (chapter 1),
  walking the sorted order.
- Read a bigger text: the `forthling` program reads a file with `SLURP-FILE`, and the MCP
  server gives a text as the `data` argument and the word `DATA`.
