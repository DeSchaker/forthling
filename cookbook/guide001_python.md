# Forth for people who know Python

*Guide. Needs the core only.*

You know Python: lists, dicts, `with`, f-strings, exceptions, slicing. Forth has all of
these ideas, with other names and one big difference. This guide takes the Python you know
and shows the forthling words beside it, with the stack picture of each word.

Every snippet here was run in forthling. The program
[`examples/guide001_example001_python.fs`](../examples/guide001_example001_python.fs) holds
them all, with the Python in comments; its output is beside it, in
`guide001_example001_python.out`, and `cargo test` compares the two. Run it with:

```
cargo run --release -- examples/guide001_example001_python.fs
```

It takes under 7500 steps (found by lowering `--max-steps` until it stopped), so it also
fits in the playground. It starts with:

```forth
ALSO FUNCTIONAL ALSO COLLECTIONS ALSO STRINGS ALSO DYNAMIC-STRINGS
```

Those are four **word lists**: groups of words. `ALSO` puts a list in the search order, so
its words can be used. The core words need no `ALSO`.

## The one big difference: the stack

In Python you write `f(a, b)`: the function name first, the arguments after. In Forth the
arguments come first and the word after. The arguments wait on a **stack**, and a word takes
what it needs from the top and leaves its results there.

```forth
3 4 *      \ the stack: 12
```

`3` and `4` are put on the stack, and `*` replaces them by `12`. A **stack picture** says
what a word does: `( a b -- c )` means *takes `a` and `b`, leaves `c`*, with the top of the
stack on the right. Every word in the manuals has one, and so should every word you write.
`.` prints the top and drops it; `.S` shows the whole stack without changing it.

Three more differences you meet at once:

- **There are no types.** A number, an address and a flag are all just cells. `.` on a string
  address prints a number. The comment with the stack picture is your type checker.
- **True is `-1` and false is `0`.** In an `IF`, any number but 0 counts as true.
- **Strings are two numbers**, `( c-addr u )`: where the text starts and how long it is.
  `S" hello"` puts both on the stack, and `TYPE` prints them.

## Names and assignment

```python
x = 5
x = x + 1
a, b = 1, 2
a, b = b, a
```

```forth
5 VALUE X
: BUMP ( -- )  X 1+ TO X ;
: SWAP-DEMO ( -- )  1 2 SWAP .S 2DROP ;     \ <2> 2 1
```

`VALUE` makes a name with a value, `TO` changes it. There is no `a, b = b, a`, because there
are no names to swap: the values are on the stack, and `SWAP` turns the top two around.
`DUP`, `DROP`, `OVER`, `ROT` and `NIP` do the rest. A short program has few names; the stack
does the work of the temporary variables.

## Functions

```python
def area(w, h):
    return w * h

def divide(a, b):
    return a // b, a % b
```

```forth
: AREA ( w h -- n )  * ;
: DIVIDE ( a b -- r q )  /MOD ;
```

`:` starts a definition and `;` ends it. The stack picture is the signature. A function with
two results is a word that leaves two numbers. `3 4 AREA .` prints `12`, and `17 5 DIVIDE . .`
prints `3 2`: the quotient on top, so it is printed first.

If you want names for the parameters, like Python, use locals:

```forth
: AREA2 ( w h -- n )  {: w h :} w h * ;
```

`{: w h :}` takes the two values from the stack and gives them names inside the word. The
first name is the deepest value. It reads like Python; short words are often clear enough
without it.

Python's `//` and `%` round down, so `-7 // 2` is `-4` and `-7 % 2` is `1`. The words `/` and
`MOD` do the same here: `-7 2 / .` prints `-4`, and `-7 2 MOD .` prints `1`. (Many Forth
systems round towards zero. forthling follows Python.)

## if, for and while

```python
"big" if n > 10 else "small"

for i in range(5):
    print(i)

i = 0
while i < 3:
    print(i)
    i += 1

for i in range(10):
    if i == 7:
        print(i)
        break
```

```forth
: SIZE ( n -- c-addr u )  10 > IF S" big" ELSE S" small" THEN ;
: COUNT-UP ( -- )  5 0 DO I . LOOP ;
: WHILE-DEMO ( -- )  0 BEGIN DUP 3 < WHILE DUP . 1+ REPEAT DROP ;
: BREAK-DEMO ( -- )  10 0 DO I 7 = IF I . LEAVE THEN LOOP ;
```

- `IF ... ELSE ... THEN` is an expression too: both branches leave the same on the stack. The
  test comes **before** the `IF`: `n 10 >` makes the flag, `IF` reads it.
- `limit start DO ... LOOP` is `for i in range(start, limit)`. `I` is the counter. Mind the
  order: the limit first, then the start. `LEAVE` is `break`.
- `BEGIN test WHILE body REPEAT` is `while`. The loop word `UNTIL` ends `BEGIN ... UNTIL`.
- `IF`, `DO`, `BEGIN` and the other control words work **inside a definition**. At the top of
  a program, wrap them in a word first.

## Lists

```python
v = []
v.append(30); v.append(10); v.append(20)
len(v); v[1]; v.pop()
v.append(20); v.sort()
sum(v); max(v)
v[1:3]
```

```forth
VEC-VARIABLE V
30 V VEC+!  10 V VEC+!  20 V VEC+!
V VEC# .  1 V VEC@ .  V VEC-POP .            \ 3 10 20
20 V VEC+!   V ' < VEC-SORT
V VEC-DATA ' + FOLD-CELLS .                  \ 60
V VEC-DATA ' MAX FOLD-CELLS .                \ 30
```

A vector (`VEC-…`) grows by itself. The vector is the **last** argument of its words: `x v VEC+!`
reads like "x, to v, add". An index outside the vector gives the error `-24`, like Python's
`IndexError`. The sort takes a word `( a b -- flag )` that says whether `a` goes before `b`:
`' <` sorts up, `' >` sorts down. Python's `key=` becomes a word of your own.

`VEC-DATA ( v -- addr n )` gives the elements as a block of cells in memory. Many words work on
such a block: `FOLD-CELLS`, `MAP-CELLS`, `FILTER-CELLS`, `SUM-CELLS`. A slice is no copy, only
a change of `addr` and `n`:

```forth
: SLICE ( addr n from to -- addr' n' )  {: a n from to :} a from CELLS +  to from - ;
V VEC-DATA 1 3 SLICE                          \ the elements 20 30
```

The tick `'` gives the address of a word to hand to another word, like passing a function in
Python. In a definition write `[']`; at the top level `'`.

## List comprehensions

```python
[x * x for x in range(1, 10) if x % 2 == 0]
sum(x * x for x in range(1, 6))
any(x > 2 for x in [1, 2, 3])
min(3, 1, 4)
```

```forth
: EVEN? ( n -- flag )  2 MOD 0= ;
: SQUARE ( n -- n^2 )  DUP * ;
1 10 RANGE ' EVEN? FILTER ' SQUARE MAP        \ the stack: 4 16 36 64
1 6 RANGE ' SQUARE MAP 5 ' + FOLD .           \ 55
: >2? ( n -- flag )  2 > ;
1 2 3 3 ' >2? ANY? .                          \ -1
3 1 4 3 MIN-OF .                              \ 1
```

`RANGE`, `FILTER`, `MAP` and `FOLD` (Python's `functools.reduce`) work on **the items on the
stack**, with the count on top: `1 10 RANGE` leaves `1 2 … 9 9`. `FILTER` leaves the items
that passed and their new count; `MAP` takes the count away, so after a `MAP` you give the
count again when the next word needs it (`5 ' + FOLD`). It is a pipeline, left to right, like the comprehension read in order.

For large amounts of data, use the `-CELLS` forms on a block: they work in place and do not
use the stack. `V VEC-DATA ' + FOLD-CELLS` above sums the vector.

You can also write the word inline as a quotation: `[: 2 MOD 0= ;]` is a small word without a
name, like a `lambda`. Inside a definition it works as well.

## Dictionaries

```python
ages = {"alice": 30, "bob": 25}
ages["carol"] = 41
ages["alice"]
"dave" in ages
del ages["bob"]
len(ages)
for name, age in ages.items(): ...
```

```forth
DICT-VARIABLE AGES
30 S" alice" AGES DICT!  25 S" bob" AGES DICT!  41 S" carol" AGES DICT!
S" alice" AGES DICT@ . .                      \ -1 30
S" dave" AGES DICT@ .                         \ 0
S" bob" AGES DICT-DELETE .  AGES DICT# .      \ -1 2
: SHOW-AGE ( c-addr u age -- )  >R TYPE ." =" R> . ;
: SHOW-AGES ( -- )  AGES ['] SHOW-AGE DICT-EACH ;       \ alice=30 carol=41
```

The key is text, the value is one cell (a number, or an address of something bigger). `DICT@`
leaves `x true` when the key is there and just `false` when it is not, so a missing key is
no error, like `ages.get`. Keys come out in the order they were added, as in Python 3.7+.
The loop is a word you hand to `DICT-EACH`, which calls it once for each pair.

## Strings

```python
s = "  Hello, World  "
s.strip()
len(s), s[2:7], s[-5:]
"World" in s
s.startswith("Hello")
"a,b,c".split(",")
", ".join(["a", "b", "c"])
s.upper()
```

```forth
: TEXT ( -- c-addr u )  S"   Hello, World  " ;
TEXT TRIM TYPE                                         \ Hello, World
TEXT TRIM 2 /STRING 5 MIN TYPE                         \ llo,
TEXT TRIM DUP 5 - /STRING TYPE                         \ World
TEXT S" World" CONTAINS? .                             \ -1
TEXT TRIM S" Hello" STRING-PREFIX? .                   \ -1
```

A slice is again a change of `c-addr` and `u`; nothing is copied. `n /STRING` drops the first
`n` characters, `n MIN` keeps at most `n`. For `s[-5:]` the text is cut at `length - 5`.

`split` and `join` use the dynamic strings, which live in memory that you free:

```forth
$VARIABLE CSV
: PRINT-PART ( c-addr u -- )  TYPE ." ;" ;
S" a,b,c" CSV $!   CSV 44 ' PRINT-PART $ITER           \ a;b;c;
$[]VARIABLE LETTERS
S" a" LETTERS $[]+!  S" b" LETTERS $[]+!  S" c" LETTERS $[]+!
LETTERS S" , " JOIN 2DUP TYPE DROP FREE THROW          \ a, b, c
```

`44` is the code of the comma. `JOIN` gives a new string in `ALLOCATE`d memory, so after
use it is released with `FREE` (`THROW` turns the error code into an error, as in Python an
exception). `UPPER` and `LOWER` change a text **in place**: a literal in `S"` is not for you
to change, so copy it into a buffer first (`CREATE BUF 20 ALLOT`, `CMOVE`).

## f-strings

```python
f"Dear {name}, you owe {amount} euro."
f"{3.14159:.2f}"
f"{42:05d}"
```

```forth
CREATE OUT 100 ALLOT
: NUM ( n -- c-addr u )  S>D <# #S #> ;
: LETTER ( -- c-addr u )
  S" Ada" S" name" REPLACES
  1250 NUM S" amount" REPLACES
  S" Dear %name%, you owe %amount% euro." OUT 100 SUBSTITUTE DROP ;
LETTER TYPE                                   \ Dear Ada, you owe 1250 euro.
: 2F ( F: x -- ) ( -- c-addr u )  100E F* FROUND F>S S>D <# # # 46 HOLD #S #> ;
3.14159E 2F TYPE                              \ 3.14
: N05 ( n -- c-addr u )  S>D <# # # # # # #> ;
42 N05 TYPE                                   \ 00042
```

Two parts. **Marks in a template** (`%name%`) take their text from `REPLACES`, and
`SUBSTITUTE` makes the result in a buffer you give it. Chapter 3 of book 3
([Template](book003_part003_template.md)) tells all about it. **The format of a number** is
made with `<# ... #>`: `#` makes one digit from the right, `#S` all the rest, `HOLD` puts a
character in (46 is the point). `.2f` is two `#` and a point, `05d` is five `#`.

To print straight away, `.` prints a number, `." text"` prints a text, and `.( text)` prints
a text while the program is being read.

## Exceptions

```python
try:
    q = a // b
except ZeroDivisionError:
    q = 0

raise ValueError("bad")
```

```forth
: SAFE-DIV ( a b -- q )  ['] / CATCH IF 2DROP ." (division by zero) " 0 THEN ;
1 0 SAFE-DIV .  6 3 SAFE-DIV .                \ (division by zero) 0 2
99 CONSTANT BAD-VALUE
: CHECK ( n -- n )  DUP 0< IF BAD-VALUE THROW THEN ;
: TRY-CHECK ( n -- )
  ['] CHECK CATCH DUP BAD-VALUE = IF DROP DROP ." bad value " ELSE DROP . THEN ;
5 TRY-CHECK  -5 TRY-CHECK                     \ 5 bad value
```

`xt CATCH` runs the word. It leaves `0` when all went well, and the **error code** when
something threw. Errors have numbers, not classes: `-10` is a division by zero, `-24` an
index or argument out of range, `-13` an unknown word. The negative numbers are for the
system; a code of your own is positive (`99` above). `THROW` with a code raises the error,
and `THROW` with `0` does nothing, so it is safe to call after any `CATCH` or `FREE`.

After a throw, the stack goes back to its depth at the `CATCH`. In forthling the failing
word's arguments are still there, which is why `SAFE-DIV` drops two numbers. Without a `CATCH`, an
error stops the program and prints a message with the line.

## with

```python
with resource() as r:
    body(r)
```

In Python the `with` closes the resource even if the body raises. In Forth you write that as
a word that gets the body as an argument:

```forth
VARIABLE OPEN-COUNT
: WITH-BUFFER ( xt -- )                       \ xt is ( addr -- )
  100 ALLOCATE THROW  1 OPEN-COUNT +!
  DUP >R SWAP CATCH
  R> FREE THROW  -1 OPEN-COUNT +!
  THROW ;
: WRITE-ONE ( addr -- )  65 SWAP C!  ." wrote " ;
: FAIL-AFTER ( addr -- )  DROP ." about to fail " 77 THROW ;
' WRITE-ONE WITH-BUFFER  OPEN-COUNT @ .       \ wrote 0
' FAIL-AFTER ' WITH-BUFFER CATCH . OPEN-COUNT @ .   \ about to fail 77 0
```

The word asks for memory, runs the body inside `CATCH`, **always** gives the memory back,
and only then passes on the error (the last `THROW`). The counter shows both calls end with
0 buffers open. The same pattern holds for a file: `OPEN-FILE` first and `CLOSE-FILE` after
the `CATCH`.

## Numbers and printing

```python
7 // 2        # 3
7 / 2         # 3.5
print("total:", 17)
```

```forth
7 2 / .  7E 2E F/ F.                          \ 3 3.5
.( total: ) 17 .
```

Integers and floats are different things in Forth. A number with an `E` is a float and lives
on a **separate float stack**: `F+`, `F*` and `F.` take their arguments from there. There is
no `float(x)` that works by itself, but `S>F` and `F>S` move a number between the two. For
sums where `0.1 + 0.2` must be exactly `0.3` (money), see the fixed-point numbers in
[the word lists](../docs/wordlists.md).

## What you will miss, and what you gain

- **Messages.** A wrong stack effect is not caught as you type. The error comes later, where
  the stack is wrong. Check often with `.S`, and give each word a stack picture.
- **Objects and classes.** There are none. A record is a block of memory with offsets
  (`BEGIN-STRUCTURE`), and `CREATE ... DOES>` makes words that share a behaviour.
- **Libraries.** There is no `import`. A word list (`ALSO JSON`) makes the words of a library
  visible; see [the word lists](../docs/wordlists.md).

What you gain: a small language you can learn whole, and a sandbox with limits on steps, time
and memory that you set.

## Try it

- Write `MAP-PAIRS`, the word for `{k: f(v) for k, v in d.items()}`, with `DICT-EACH`.
- Write `ENUMERATE ( v xt -- )` that calls `xt` with `( i x -- )` for each element.
- Change `WITH-BUFFER` so that the size is a parameter, `( size xt -- )`.
- Look up a Python idiom you use every day, and find its words in
  [the word lists](../docs/wordlists.md).
