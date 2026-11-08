# Tools for writing and finding faults

Tests, a tracer, a debugger, help and a tutorial, all written in Forth words. Back to the [README](../README.md).

## Tests

The word list `TESTING` (`ALSO TESTING`) checks your own code with the syntax of the
Forth-2012 test suite: the cells that the code between `T{` and `->` leaves must be the
cells after `->`. A wrong result is printed with its place and counted, and the run goes
on. The forthling program, the MCP server, the Python module and the playground install it;
a Rust program does with `forth.install(&forthling::Testing)`. The words need no permission.

```forth
ALSO TESTING
: SQUARE  DUP * ;

TESTING squares
T{ 3 SQUARE -> 9 }T
T{ -4 SQUARE -> 16 }T
T{ 5 SQUARE -> 24 }T
T{ 2 SQUARE -> 4 5 }T
TEST-SUMMARY
```

The last two tests fail, and print (`file.fs` is the file the lines are in):

```
FAIL file.fs:7 (squares): incorrect result: expected <1> 24, got <1> 25
FAIL file.fs:8 (squares): wrong number of results: expected <2> 4 5, got <1> 4
tests: 2 passed, 2 failed
```

| Word | Stack |
|---|---|
| `T{` | `( -- )`: start a test; the stack as it is now stays below the test |
| `->` | `( -- )`: the cells since `T{` are the actual results |
| `}T` | `( -- )`: the cells since `->` are the expected results; compare, and print and count a difference |
| `TESTING` | `( "text" -- )`: the rest of the line names the tests that follow, in the failure lines |
| `TEST-SUMMARY` | `( -- )`: print how many tests passed and failed |
| `SET-EXACT` | `( -- )`: compare floats as identical (the default) |
| `SET-NEAR` | `( -- )`: compare floats within `REL-NEAR` or `ABS-NEAR` |
| `REL-NEAR`, `ABS-NEAR` | `( -- a-addr )`: float variables with the tolerances; `1E-12` and `0E` at the start |

A failure line has the input (`input`, `evaluate` or the path of the file), the line of `T{`
(from 1), the group, and the cells of both sides in the current `BASE`, each list as its
depth and its cells (the top 32 of a long list). If there is something on the stack below
the test, a second line shows it. An error inside a test is not caught: it ends the run as
anywhere else, so the limits of a sandbox cannot be hidden. `->` or `}T` in the wrong place
throws -22. Cells are compared as integers; floats have their own rule, below.

**Floats.** The floats on the float stack count too: those that the code leaves between `T{`
and `->` must be those after `->`, in the same number. This is the rule of `ttester.fs` of
the Forth-2012 test suite, and a test may mix cells and floats (the two stacks are compared
apart). A float is identical by default, so `0.1E 0.2E F+` is not `0.3E`. After `SET-NEAR`
two floats are equal when they are identical, or within `REL-NEAR` (relatively, as `F~` with
a negative tolerance) or within `ABS-NEAR` (absolutely); set them with `F!`, and go back with
`SET-EXACT`. The failure line then has a second line for the floats, in scientific notation:

```forth
ALSO TESTING
SET-NEAR
T{ 0.1E 0.2E F+ -> 0.3E }T
1E-6 REL-NEAR F!
T{ 1E 1.1E -> 1E 1E }T
```

```
FAIL file.fs:5: incorrect result
  floats: expected <2> 1E0 1E0, got <2> 1E0 1.1E0
```

The first test passes (the default `REL-NEAR` is `1E-12`). The float stack below the test is
left alone. A test of floats only has no cells to show, so the first line has none.

`forthling --test file.fs ...` runs the files, prints `tests: N passed, M failed` at the end
(the counts run over all the files) and exits with 1 if a test failed or an error stopped
the run. A file still has to say `ALSO TESTING` itself. The MCP server gives the counts of
each `forth_eval` call as `tests` in the result (see [mcp/README.md](../mcp/README.md)); a
Rust program takes them with `Forth::take_test_results()`, which also starts counting again.

The Forth-2012 test suite that `cargo test` runs has its own `T{` in the main word list;
it does not use this one.

## Tracing

To see what code does word by word, switch on a trace. `TRACE-ON` and `TRACE-OFF` are in
the word list `DEBUG` (`ALSO DEBUG`), installed by the forthling program, the MCP server
and the playground. Each word that runs gets a line: indented by how deep it is nested,
then its name and the data stack when it starts, as `.S` shows it (at most the top 8
cells):

```
ALSO DEBUG
: SQ  DUP * ;
: F  3 SQ 1+ ;
TRACE-ON F . TRACE-OFF
```

```
F <0>
  SQ <1> 3
    DUP <1> 3
    * <2> 3 3
  1+ <1> 9
. <1> 10
10
```

The insides of the system's own words (such as `/` and `.`, written in Forth) are not
traced, nor are the words that run while a definition is compiled, nor the system's words
that read the input, such as `\`, `(` and `[IF]`. The lines are output,
so the output limit of the sandbox holds for them. A host records the same as data:
`record_trace(Some(max))` and then `take_trace()`, a `TraceEntry` per word (`word`,
`depth`, `stack`, `stack_depth`); `forth_eval` of the MCP server does this with
`trace: true`. When no trace is on, it costs one test per word.

## Debugging

`BREAK`, also in the word list `DEBUG`, stops the program where it stands. The forthling
program then shows the words that are running and the stack, and asks how to go on: `s`
(or an empty line) steps to the next word, `n` steps over the words this one calls, `f`
runs on until the word this one is in has finished, `c` continues to the next `BREAK`, and
`q` quits with -28 (user interrupt). A file `debug.fs`:

```
ALSO DEBUG
: SQ  DUP * ;
: F  3 SQ 1+ ;
: MAIN  BREAK F 2 SQ + ;
MAIN .
```

and `forthling debug.fs` in a terminal, with the answers typed after the prompts:

```
break: MAIN > BREAK  <0>
s step, n over, f out, c continue, q quit> s
step: MAIN > F  <0>
s step, n over, f out, c continue, q quit> s
step: MAIN > F > SQ  <1> 3
s step, n over, f out, c continue, q quit> f
step: MAIN > SQ  <2> 10 2
s step, n over, f out, c continue, q quit> c
14
```

A host decides what a pause does with `set_debugger`: a function that gets the system
and a `Pause` (why it stopped, the word, its depth, and the stacks, the running words and
the place in the input, as for an error) and returns a `Resume` (`Step`, `Over`, `Out`,
`Continue`) or an error to throw. The time spent in it does not count towards the time
limit. Without a debugger, `BREAK` does nothing.

The MCP server and the playground cannot wait in the middle of a run, as the program
waits for the terminal: `CATCH`, `EVALUATE` and `INCLUDE` run Forth inside Rust calls, so
a run can only stop while its host function runs. So they do it differently. In
`forth_eval`, `BREAK` does not stop: the result shows the stacks and the running words at
each `BREAK`. In the playground, **Step through** runs the code while it records each
word, then plays the run back: forwards and back, step, over, out and on to the next
`BREAK`, with the stack before and after each word. Since the sandbox makes a run come out
the same each time, the record is what happened.

## Inspecting code

Words from Gforth help to find a fault. They are in the `forthling` program, the MCP server
and the playground, with no word list to open.

- `~~` prints the place in the source and the data stack, when the definition runs (at
  once when typed): `file:line:column: <2> 1 2`. `~~BT` adds a backtrace. `BT` prints the
  words that are running, the innermost first.
- `ASSERT( code )` runs the code, which leaves a flag, and throws -2 with the place when
  the flag is false. The variable `ASSERT-LEVEL` (1 at the start) decides what is
  compiled: `ASSERT0(` always, `ASSERT1(` and `ASSERT(` from level 1, `ASSERT2(` from
  level 2, `ASSERT3(` from level 3. A level that is too low skips the code up to the
  next `)`, so it must not contain one. Set the level before the definitions.
- `WHERE name` says where the program defined a word: `SQ defined at demo.fs:1`; a word of
  the system says so. `LOCATE name` adds the line itself when the source is a file that
  can be read. `SEE-CODE` is `SEE`: both show the instructions of the word.
- `LATEST` and `LATESTXT` give the newest word, `FIND-NAME ( c-addr u -- nt | 0 )` looks a
  name up, `NAME>LINK ( nt -- nt | 0 )` gives the word defined before it in its word list
  and `XT>NAME ( xt -- nt )` the name token. In forthling a name token and an execution
  token are the same number. `NAME>STRING` gives the name.
- `VLIST` is `WORDS`. `INCLUDED? ( c-addr u -- flag )` tells whether a file was included
  and `.INCLUDED` lists the files, in order.

```
: SQ  DUP * ;
: F  3 ~~ SQ 1+ ;
F .
WHERE SQ
```

```
demo.fs:2:8: <1> 3
10 SQ defined at demo.fs:1
```

## Help

`HELP name` shows the stack effect of a word, a line on what it does and, for most
words, an example. Every word of the system and of the extensions has an entry; a test
fails when one has not, and runs the examples, so the text cannot drift from the words.
For a name that is not defined it lists names that are close, and the hint of an
undefined-word error points to `HELP`.

```
: SQ ( n -- n*n ) DUP * ;
HELP SQ          \ SQ ( n -- n*n )
HELP SWAP        \ SWAP ( x1 x2 -- x2 x1 ), what it does, and an example
HELP DUPP        \ DUPP is not defined. Close: DUMP DUP
```

A word you define shows the `( )` comment after its name. The MCP server has the tool
`forth_help` (the same, as JSON), and a Rust host adds the text of its own words with
`Forth::add_help`; `Forth::help_problems` and `Forth::help_examples` are for its tests.
The text is in `src/help.txt`, about 50 KB in the binary (52,224 bytes more in a release
build). The cargo feature `help` is on by default; `default-features = false` leaves
the text out, and `HELP` still shows the comments of your own words.

## Tutorial

New to Forth? The tutorial teaches it inside forthling, in 20 short lessons: ten for
beginners (the stack, arithmetic, defining words, decisions, loops, variables, strings)
and ten beyond (recursion, locals, tables, execution tokens, exceptions, floating point,
formatting numbers, JSON and finding a bug with a trace). Each lesson explains a little,
gives code to try and a task; `CHECK` tests your answer and says what is still wrong.

```
ALSO TUTORIAL
LESSONS          \ the list, with the lessons you have done
4 LESSON         \ lesson 4: Defining words
: CUBE DUP DUP * * ;
CHECK            \ Well done! Lesson 4 is done. NEXT goes on to: Printing.
HINT  NEXT       \ a hint, and the next lesson
```

The program, the MCP server and the playground have it; in the playground the
**Tutorial** button opens the lessons beside the editor and keeps your progress in the
browser. `Forth::new()` leaves it out, so a standard program sees no extra words; a Rust
host installs it with `forth.install(&forthling::Tutorial)`, and `Forth::tutorial(word)`
runs its words also without `ALSO TUTORIAL`, for buttons.
