# forthling: system documentation for Forth-2012

This is the documentation that section 4.1 of the Forth-2012 standard asks of a
standard system: the implementation-defined options, how the system acts on each
ambiguous condition, and other facts about the system. It follows the order of the
standard: first the Core word set (4.1), then each optional word set (x.4.1). The item
names are short paraphrases of the standard's; the section numbers point to the
standard's own text.

A Dutch translation is in [`VERKLARING.md`](VERKLARING.md). The roadmap and the status
of each word set are in [`ROADMAP.md`](ROADMAP.md).

## Labeling (5.1.1)

forthling is a Forth-2012 Standard System,

- providing the Core Extensions word set;
- providing the Block word set and the Block Extensions word set;
- providing the Double-Number word set and the Double-Number Extensions word set;
- providing the Exception word set and the Exception Extensions word set;
- providing the Facility word set and the Facility Extensions word set;
- providing the File-Access word set and the File-Access Extensions word set;
- providing the Floating-Point word set and the Floating-Point Extensions word set;
- providing the Locals word set and the Locals Extensions word set;
- providing the Memory-Allocation word set;
- providing the Programming-Tools word set;
- providing names from the Programming-Tools Extensions word set: all except `;CODE`,
  `ASSEMBLER`, `CODE` and `EDITOR`;
- providing the Search-Order word set and the Search-Order Extensions word set;
- providing the String-Handling word set and the String-Handling Extensions word set;
- providing the Extended-Character word set and the Extended-Character Extensions word
  set.

The word sets with a test file in the
[Forth-2012 test suite](https://github.com/gerryjackson/forth2012-test-suite) pass it,
the floating-point tests and the paranoia test included. The suite has no tests for the
Extended-Character word set, so forthling has its own:
[`tests/xchartest.fth`](tests/xchartest.fth).

## Core: implementation-defined options (4.1.1)

- **Aligned addresses** (3.1.3.3): `ALIGN` and `ALIGNED` align to 8 address units, the
  size of a cell. The system does not demand alignment: `@`, `!` and the other memory
  words work at any address.
- **`EMIT` of a non-graphic character**: the byte goes to standard output unchanged; the
  terminal decides what it shows.
- **Character editing of `ACCEPT`**: forthling does none itself. Standard input is read
  a line at a time, so the line editing of the terminal applies (backspace, for one).
  `ACCEPT` keeps at most the requested number of characters and drops the rest of the
  line.
- **Character set** (3.1.2, `EMIT`, `KEY`): a character is a byte (0 to 255), and text is
  read and written as bytes. Extended characters are UTF-8 (see the XCHAR section).
- **Character-aligned addresses** (3.1.3.3): every address is character-aligned.
- **Matching of character-set extensions** (3.4.2): names are compared after turning the
  ASCII letters `a` to `z` into capitals; all other bytes, those of UTF-8 included, must
  match exactly.
- **Control characters as a space delimiter** (3.4.1.1): when the text interpreter,
  `PARSE-NAME` or `BL WORD` look for a name, every character up to and including the
  space (0 to 32) delimits it. `PARSE` with a space delimiter matches only the space.
- **Format of the control-flow stack** (3.2.3.2): a separate stack in the compiler, not
  the data stack. Each origin or destination takes one entry, and `CS-PICK` and
  `CS-ROLL` work on it.
- **Digits larger than thirty-five** (3.2.1.2): they cannot occur, since `BASE` can be
  at most 36. With `BASE` outside 2 to 36, no word converts as a number, and numbers are
  displayed in decimal.
- **Display after input ends in `ACCEPT`**: nothing; the cursor is where the terminal's
  echo of the Enter key left it, at the start of the next line.
- **Exception abort sequence** (`ABORT"`): `ABORT"` throws -2. When nothing catches it,
  the message is shown as `error -2: aborted: message` (with `file:line:` in front in a
  file), the stacks are cleared and control returns to the terminal. When files are run
  from the command line, the run stops with exit code 1.
- **Input line terminator** (3.2.4.1): the Enter key, which ends a line with LF (or with
  CR LF, which is treated the same).
- **Maximum size of a counted string** (3.1.3.4, `WORD`): 255 characters.
- **Maximum size of a parsed string** (3.4.1): 1024 characters, the size of an input
  line. `WORD` throws -18 for a string longer than 255 characters.
- **Maximum size of a definition name** (3.3.1.2): forthling sets no limit of its own;
  the input line (1024 characters) is the limit.
- **Maximum string length for `ENVIRONMENT?`**: no limit; a string that names no known
  query gives false. The name of an installed extension is a query too (see Extensions).
- **Selecting the user input device** (3.2.4.1): the terminal is standard input, so it is
  chosen when starting forthling, by redirecting or piping into it. Files given on the
  command line are run instead of the REPL.
- **Selecting the user output device** (3.2.4.2): standard output, chosen the same way.
- **Methods of dictionary compilation** (3.3): headers and compiled code live outside
  Forth memory, so data space holds only what a program puts there (`,` `ALLOT`
  `CREATE` `VARIABLE` and so on). A colon definition is compiled to a list of
  instructions (calls, literals, branches), which an inner interpreter runs.
- **Bits in an address unit** (3.1.3.3): 8.
- **Number representation and arithmetic** (3.2.1.1): two's complement; cells are 64
  bits, stored little-endian in memory.
- **Ranges for n, +n, u, d, +d and ud** (3.1.3, 3.1.4): n from -2^63 to 2^63-1, +n from 0
  to 2^63-1, u from 0 to 2^64-1, d from -2^127 to 2^127-1, +d from 0 to 2^127-1 and ud
  from 0 to 2^128-1.
- **Read-only data-space regions** (3.3.3): none.
- **Size of the `WORD` buffer** (3.3.3.6): 255 characters plus the count.
- **Size of a cell** (3.1.3): 8 address units.
- **Size of a character** (3.1.2): 1 address unit.
- **Size of the terminal input buffer** (3.3.3.5): 1024 characters; a longer line throws
  -18.
- **Size of the pictured numeric output buffer** (3.3.3.6): 256 characters; going
  beyond it throws -17.
- **Size of `PAD`** (3.3.3.6): 256 characters.
- **Case sensitivity** (3.4.2): names are not case-sensitive for the letters `a` to `z`;
  digits in numbers may be either case too (`$ff`).
- **System prompt** (3.3, `QUIT`): ` ok` after a line, or ` compiled` while a definition
  is open.
- **Division rounding** (3.2.2.1): floored, for `/ MOD /MOD */ */MOD` and `M*/`.
  `SM/REM` is there for symmetric division.
- **Value of `STATE` when true**: -1.
- **Values after arithmetic overflow** (3.2.2.2): the result wraps around, modulo 2^64
  (for `+ - * 2* NEGATE` and the like). The words that divide throw -11 when the quotient
  does not fit.
- **Whether the current definition can be found after `DOES>`**: no; a definition can
  be found only after its `;`.

## Core: ambiguous conditions (4.1.2)

General conditions:

- **A name that is neither a word nor a number**: throws -13.
- **A definition name that is too long**: cannot occur below the length of an input line.
- **Addressing a region not listed in 3.3.3**: any address inside Forth memory can be
  read and written, including the system's own buffers and variables; an address
  outside it throws -9.
- **An argument of the wrong type**: not detected; a cell is a cell.
- **The execution token of a word without interpretation semantics** (`'`, `FIND`):
  is returned as usual. Executing such a word outside a definition throws -14.
- **Dividing by zero**: throws -10, also in `M*/`, `FM/MOD`, `SM/REM` and `UM/MOD`.
- **Data stack or return stack overflow**: throws -3 or -5. The data and return stacks
  hold 65536 cells each, and calls nest at most 10000 deep (-5 beyond that).
- **No room for loop-control parameters**: loops have a stack of their own that grows
  as needed; the call depth above limits it.
- **No room in the dictionary**: throws -8.
- **Interpreting a word without interpretation semantics**: throws -14.
- **Changing the input buffer or a string literal**: allowed; it simply changes what is
  parsed next or what the literal holds afterwards.
- **Overflow of the pictured numeric output string**: throws -17.
- **Parsed string overflow**: throws -18.
- **An out-of-range result**: `*` wraps around. `*/`, `*/MOD`, `FM/MOD`, `SM/REM`,
  `UM/MOD` and `M*/` throw -11. `>NUMBER` wraps around modulo 2^128.
- **Data stack or return stack underflow**: throws -4 or -6.
- **A zero-length name at the end of the input buffer**: throws -16.

Specific conditions:

- **`>IN` beyond the input buffer**: the input buffer counts as used up; parsing goes on
  with the next line.
- **`RECURSE` after `DOES>`**: compiles a call to the whole defining word, not to the
  `DOES>` part.
- **`RESTORE-INPUT` with another input source**: nothing changes and the flag is true
  (failure).
- **Data space with definitions in it de-allocated**: headers and code are not in data
  space, so a definition keeps working; its data (a `VARIABLE` cell, say) may be
  overwritten.
- **Data space access with incorrect alignment**: works.
- **The data-space pointer not aligned** (`,`): works; the cell is stored unaligned.
- **Fewer than u+2 items for `PICK` or `ROLL`**: throws -4.
- **No loop-control parameters** (`I`, `J`, `LEAVE`, `UNLOOP`, `LOOP`, `+LOOP`): throws
  -26.
- **`IMMEDIATE` after a definition without a name**: the `:NONAME` definition becomes
  immediate, which changes nothing.
- **`TO` not followed by a value or a local**: throws -32.
- **Name not found** (`'`, `POSTPONE`, `[']`, `[COMPILE]`): throws -13.
- **`DO`, `?DO` or `WITHIN` with parameters of different types**: not detected.
- **`POSTPONE`, `[COMPILE]`, `'` or `[']` applied to `TO`**: works; the word compiled
  then acts as `TO` where it runs.
- **`WORD` with a string longer than a counted string**: throws -18.
- **A shift of a cell or more** (`LSHIFT`, `RSHIFT`): every bit is shifted out, so
  `1 64 LSHIFT` gives 0. A negative shift count does the same.
- **A word not defined by `CREATE`** (`>BODY`, `DOES>`): `>BODY` throws -31 and `DOES>`
  throws -21.
- **`#`, `#S`, `HOLD`, `HOLDS` or `SIGN` outside `<#` and `#>`**: they work on the
  pictured numeric output buffer as it is, and throw -17 when it is full.
- **A deferred word without an action**: throws -21.
- **`IS` or `ACTION-OF` on a word not defined by `DEFER`**: throws -32.
- **`POSTPONE`, `[COMPILE]`, `[']` or `'` applied to `ACTION-OF` or `IS`**: works like
  `TO` above.
- **`\x` not followed by two hexadecimal digits** (`S\"`): the digits that are there
  are used, so `\x4z` gives character 4 followed by `z`.
- **A `\` before any other character** (`S\"`): the character stands for itself.

## Core: other system documentation (4.1.3)

- **Non-standard words that use `PAD`**: none; the system never uses `PAD`.
- **Terminal facilities**: standard input, read a line at a time (`KEY` returns once a
  line has been entered), and standard output. `AT-XY` and `PAGE` write ANSI escape
  sequences. There is no raw mode, so special keys never arrive.
- **Program data space**: memory is 1 MiB unless `--memory` or the config file sets
  another size (64 KiB to 4 GiB). Of the default 1 MiB, 1,021,112 address units are free
  at the start (`UNUSED`); data space and the heap of `ALLOCATE` share them.
- **Return stack space**: 65536 cells, and a call depth of at most 10000. Locals and loop
  parameters are kept apart from it.
- **Data stack space**: 65536 cells. The floating-point stack holds 1024 floats.
- **System dictionary space**: none of data space. The system's fixed buffers and
  variables take the first 27,464 address units of Forth memory; the dictionary itself
  is outside Forth memory.

## Block (7.4.1)

Implementation-defined options:

- **Layout of `LIST`**: a line `Screen u`, then the 16 lines of the block, each preceded
  by its number (0 to 15), with control characters shown as spaces and trailing spaces
  left out.
- **Line length for `\`**: 64 characters; in a block, `\` skips to the end of its line of
  64.

Ambiguous conditions:

- **A block that cannot be read**: throws -33.
- **I/O errors in block transfers**: throws -33 when reading and -34 when writing.
- **An invalid block number** (`BLOCK`, `BUFFER`, `LOAD`): numbers run from 1 to 1048576;
  any other throws -35.
- **A program changing `BLK`**: it does not change the input source. `BLK` is set
  again on the next change of input source.
- **`UPDATE` without a current block buffer**: does nothing.

Other system documentation:

- **Multiprogramming restrictions on buffer addresses**: forthling is not a
  multiprogramming system. There are 8 block buffers; an address from `BLOCK` or `BUFFER`
  stays valid until the buffer is reused for another block.
- **Blocks available**: 1 to 1048576, in one block file (`blocks.fb` in the current
  directory, or the file `--blocks` or the config file names). Any block can hold source
  text or data. Blocks beyond the end of the file read as spaces.

## Double-Number (8.4.1)

- **`D>S` with a d outside the range of n**: gives the low cell, the value modulo 2^64.

## Exception (9.4.1)

`THROW` codes: forthling throws the codes of table 9.1 of the standard, with these
messages when nothing catches them:

| Code | Message | Code | Message |
|---:|---|---:|---|
| -1 | aborted | -38 | non-existent file |
| -2 | aborted (and the `ABORT"` message) | -39 | unexpected end of file |
| -3 | stack overflow | -44 | floating-point stack overflow |
| -4 | stack underflow | -45 | floating-point stack underflow |
| -5 | return stack overflow | -49 | search-order overflow |
| -6 | return stack underflow | -50 | search-order underflow |
| -8 | dictionary overflow | -56 | `QUIT` |
| -9 | invalid memory address | -59 | `ALLOCATE` |
| -10 | division by zero | -60 | `FREE` |
| -11 | result out of range | -61 | `RESIZE` |
| -13 | undefined word | -62 | `CLOSE-FILE` exception |
| -14 | interpreting a compile-only word | -63 | `CREATE-FILE` exception |
| -15 | invalid `FORGET` | -64 | `DELETE-FILE` exception |
| -16 | attempt to use zero-length string as a name | -65 | `FILE-POSITION` exception |
| -17 | pictured numeric output string overflow | -66 | `FILE-SIZE` exception |
| -18 | parsed string overflow | -67 | `FILE-STATUS` exception |
| -21 | unsupported operation | -68 | `FLUSH-FILE` exception |
| -22 | control structure mismatch | -69 | `OPEN-FILE` exception |
| -24 | invalid numeric argument | -70 | `READ-FILE` exception |
| -26 | loop parameters unavailable | -71 | `READ-LINE` exception |
| -29 | compiler nesting | -72 | `RENAME-FILE` exception |
| -31 | `>BODY` used on non-`CREATE`d definition | -73 | `REPOSITION-FILE` exception |
| -32 | invalid name argument | -74 | `RESIZE-FILE` exception |
| -33 | block read exception | -75 | `WRITE-FILE` exception |
| -34 | block write exception | -76 | `WRITE-LINE` exception |
| -35 | invalid block number | -78 | `SUBSTITUTE` |
| -37 | file I/O exception | -79 | `REPLACES` |

forthling has one code of its own: -256, input nesting too deep (more than 15 files,
`EVALUATE`d strings or `LOAD`ed blocks inside each other). The Python module adds -257, a
Python exception in a word written in Python (see Extensions). A positive code, or one that is
not in the table, is shown as `exception`. How forthling acts on each ambiguous
condition is described in this document, word set by word set.

## Facility (10.4.1)

Implementation-defined options:

- **Encoding of keyboard events** (`EKEY`): `EKEY` returns the Unicode code point of the
  next character typed, which arrives once a line has been entered. The special keys
  (`K-UP`, `K-F1` and the like) are numbered from $110000, above Unicode, and never
  arrive; the modifier masks are $1000000 (shift), $2000000 (control) and $4000000 (alt).
- **Length of a clock tick**: `MS` asks the operating system to sleep for whole
  milliseconds; the actual resolution is the operating system's. `TIME&DATE` counts
  whole seconds, in UTC.
- **Repeatability of `MS`**: it waits at least the given number of milliseconds, and
  may wait longer when the operating system is busy.

Ambiguous conditions:

- **`AT-XY` on a device that cannot do it**: the escape sequence is written anyway, so
  it ends up in redirected output.
- **A structure name used before its `END-STRUCTURE`**: gives 0, since the size is not
  known yet.

## File-Access (11.4.1)

Implementation-defined options:

- **File access methods** (`BIN`, `CREATE-FILE`, `OPEN-FILE`, `R/O`, `R/W`, `W/O`): `R/O`
  is 0, `W/O` is 1 and `R/W` is 2. `BIN` adds 4 and changes nothing: files are always
  read and written as bytes, without translating line endings. `CREATE-FILE` creates the
  file, or empties it if it exists.
- **File exceptions**: each word returns its own ior from table 9.1 (for example -69 for
  `OPEN-FILE` and -70 for `READ-FILE`), also for an invalid file id. `INCLUDE-FILE` and
  `INCLUDED` throw: -37 for an invalid file id or a read error, -38 for a file that
  cannot be opened.
- **Line terminator for `READ-LINE`**: LF or CR LF. A line longer than the buffer is
  continued by the next `READ-LINE`. `WRITE-LINE` ends lines with LF.
- **File name format**: the operating system's paths, relative to the current directory.
  `INCLUDED` and `REQUIRED` also look for a relative name next to the file that is being
  included.
- **What `FILE-STATUS` returns**: x is always 0; the ior is 0 when the file exists, and
  -67 otherwise.
- **Input after an exception in `INCLUDE-FILE` or `INCLUDED`**: the file is closed. If
  nothing catches the exception, the error is shown with the file name and line number
  and input returns to the terminal; `CATCH` restores the input source that was current
  when it was called.
- **Maximum depth of file input nesting**: 15 sources inside each other (files,
  `EVALUATE` and `LOAD` together); one more throws -256.
- **Maximum size of an input line**: 1024 characters; a longer line throws -18.
- **Mapping block ranges to files**: all blocks are in one file, block u at byte offset
  (u-1)×1024.
- **Number of string buffers**: two, used in turn by `S"` and `S\"` outside a
  definition.
- **Size of those buffers**: 1024 characters each.

Ambiguous conditions:

- **`REPOSITION-FILE` beyond the end of the file**: allowed. A later write extends the
  file, and the gap holds zero bytes.
- **Reading positions not written yet**: beyond the end, `READ-FILE` reads 0 characters
  and `READ-LINE` returns a false flag, both with ior 0; a gap reads as zero bytes.
- **An invalid file id for `INCLUDE-FILE`**: throws -37.
- **An I/O error while `INCLUDE-FILE` or `INCLUDED` reads or closes a file**: a read
  error throws -37; the file is closed, and an error when closing is ignored.
- **`INCLUDED` with a file that cannot be opened**: throws -38.
- **A block number that is not mapped**: every valid block number (1 to 1048576) is
  mapped; others throw -35.
- **`SOURCE-ID` while `BLK` is not zero**: gives 0, as for the terminal.
- **`REQUIRED` for a file that is being included**: a file counts as included as soon as
  its inclusion starts, so `REQUIRED` does nothing. `INCLUDED` does include it again.
- **A marker and `REQUIRED`**: a marker does not forget which files were included, so
  after it runs, `REQUIRED` does not load a file again whose words the marker removed.
- **One file under different names, or different files under one name** (`REQUIRED`):
  files are told apart by their full path at the moment they are included. Different
  names for the same file (relative and absolute, say) count as one file; a new file at a
  path that was included before counts as included.
- **A `REQUIRED` file whose stack effect is not ( i*x -- i*x )**: not checked.

## Floating-Point (12.4.1)

Implementation-defined options:

- **Format and range of floats** (12.3.1, `REPRESENT`): IEEE 754 binary64 (double
  precision). The largest float is about 1.7976931348623157E308, the smallest normal one
  about 2.2250738585072014E-308, and subnormals reach down to about 4.9E-324. Infinities,
  NaNs and -0 exist.
- **`REPRESENT` with an infinity or a NaN**: flag2 is false, n is 0, flag1 gives the sign,
  and the buffer holds `inf` or `nan`, filled up with spaces.
- **Rounding** (12.3.1.2): round to nearest, ties to even, for arithmetic and for
  converting text to floats. `FROUND` rounds ties to even too.
- **Size of the floating-point stack** (12.3.3): 1024 floats. Overflow throws -44 and
  underflow -45.
- **Width of the floating-point stack** (12.3.3): 64 bits. It is a separate stack, and a
  float takes 8 address units in memory (`1 FLOATS`), like a DFLOAT; an SFLOAT takes 4.

Ambiguous conditions:

- **`F@`, `F!`, `DF@`, `DF!`, `SF@` or `SF!` at an unaligned address**: works.
- **A result out of range** (`F/`): gives an infinity, or 0 or a subnormal on underflow.
- **`BASE` not decimal** (`REPRESENT`, `F.`, `FE.`, `FS.`): these always work in decimal.
  The text interpreter recognises float literals only while `BASE` is decimal;
  `>FLOAT` is always decimal.
- **Both arguments zero** (`FATAN2`): gives ±0 or ±π, following the signs of the zeros, as
  IEEE does.
- **Cosine of the argument zero** (`FTAN`): cannot happen exactly; near π/2 the result is
  large but finite.
- **d or n not exactly representable** (`D>F`, `S>F`): rounded to the nearest float.
- **Exponent too large** (`DF!`, `DF@`, `SF!`, `SF@`): `SF!` stores an infinity (or 0
  or a subnormal for a tiny value); `DF!` and `DF@` always fit.
- **`FACOSH` below one**: NaN.
- **`FLNP1` at or below minus one**: -inf at -1, NaN below.
- **`FLN` or `FLOG` at or below zero**: -inf at 0, NaN below.
- **`FSQRT` below zero**: NaN.
- **`FACOS`, `FASIN` or `FATANH` beyond one**: NaN; `FATANH` of ±1 is ±inf.
- **The integer part not representable** (`F>D`, `F>S`): throws -11, also for an infinity
  or a NaN.
- **Output longer than the pictured numeric output buffer** (`F.`, `FE.`, `FS.`): they do
  not use that buffer, so any length is written (`1E300 F.` writes 301 digits).

## Locals (13.4.1)

Implementation-defined options:

- **Maximum number of locals in a definition**: 256 (`ENVIRONMENT? #LOCALS`); one more
  throws -21.

Ambiguous conditions:

- **Executing a local in interpretation state**: locals are found only while compiling,
  so the name is undefined (-13), also between `[` and `]`.
- **A local name ending in `:`, `[` or `^`**: an ordinary name.
- **A local name of one non-alphabetic character**: an ordinary name. Inside the
  definition it hides the word of that name (a local `+`, say).
- **`{: ... :}` spanning more than one line**: allowed, in files and at the terminal.
- **`{: ... :}` more than once in a definition**: allowed; the locals of every
  declaration can be used.

## Memory-Allocation (14.4.1)

The standard asks for no extra documentation. For the record: the heap grows down from
the top of Forth memory towards `HERE`, blocks are cell-aligned, and the iors are -59
(`ALLOCATE`), -60 (`FREE`) and -61 (`RESIZE`).

## Programming-Tools (15.4.1)

Implementation-defined options:

- **End marker after `;CODE` and `CODE`, and how that input is processed**: there is no
  assembler, so `;CODE` and `CODE` are not provided.
- **Search-order capability of `EDITOR` and `ASSEMBLER`**: not provided.
- **Source and format of `SEE`**: `SEE` shows a colon definition as `: NAME`, then one
  numbered line per compiled instruction (a word, a number, a branch with its target),
  then `;` and `IMMEDIATE` if it is. Other words get one line, such as `DUP is a
  primitive`, `IF is a compile-only primitive IMMEDIATE`, `5 CONSTANT X` or
  `CREATE BUF ( address 27464 )`; a word of an extension shows as `NAME is a host word`.

Ambiguous conditions:

- **`FORGET` deleting the compilation word list**: `FORGET` removes words only, never
  word lists, so it cannot happen. (A marker does remove the word lists made after it,
  and puts the compilation word list back as it was.)
- **`CS-PICK` or `CS-ROLL` with too few items on the control-flow stack**: throws -22.
- **`FORGET` or `SYNONYM` with a name that cannot be found**: throws -13.
- **`;CODE` on a word not defined by `CREATE`**: `;CODE` is not provided.
- **`POSTPONE` applied to `[IF]`**: works; the word compiled then acts as `[IF]` on the
  input that follows it.
- **The end of the input before a matching `[ELSE]` or `[THEN]`**: in a file or a string,
  skipping simply stops at its end; at the terminal, it continues with the next lines.
- **`FORGET` removing a definition that is still needed**: `FORGET` removes the word and
  every later one, so no remaining definition can refer to it. An execution token kept
  elsewhere (in a `DEFER` or a variable) then refers to whatever word takes its place
  later, or throws -24 when there is none.
- **`IMMEDIATE` applied to a word made by `SYNONYM`**: the synonym becomes immediate;
  the original word does not change.
- **`NR>` without a matching `N>R`**: the top of the return stack is taken as the count;
  -6 if there are too few items.
- **Adding or deleting words during `TRAVERSE-WORDLIST`**: words added are not visited.
  After words are removed (by a marker, say), the walk goes on below the new end of the
  dictionary; if the execution token itself was removed, the next call throws -24.

## Search-Order (16.4.1)

Implementation-defined options:

- **Maximum number of word lists in the search order**: 16 (`ENVIRONMENT? WORDLISTS`).
- **Minimum search order**: `FORTH-WORDLIST` alone, which is what `ONLY` sets.

Ambiguous conditions:

- **Changing the compilation word list during a definition**: the definition goes into
  the word list that was the compilation word list when it started.
- **An empty search order**: only the words that set a new search order are found:
  `FORTH`, `FORTH-WORDLIST`, `GET-ORDER`, `ONLY`, `ORDER` and `SET-ORDER`. Any other word
  is undefined (-13). `PREVIOUS` with an empty search order throws -50.
- **Too many word lists in the search order** (`ALSO`): throws -49.

## String-Handling (17.4.1)

Ambiguous conditions:

- **A substitution that cannot be created** (`REPLACES`): substitutions are kept outside
  Forth memory, so only the computer's own memory limits them.
- **A substitution name with a `%` in it** (`REPLACES`): throws -79.
- **A result too large for the buffer** (`SUBSTITUTE`, `UNESCAPE`): `SUBSTITUTE` leaves
  the buffer alone and returns 0 characters and -78. `UNESCAPE` has no buffer
  length, so it uses the end of the buffer the destination lies in: one of the system's
  buffers (such as `PAD`, 256 bytes), the data space up to the heap, or an `ALLOCATE`d
  block. A result that runs past it throws -9 and writes nothing.
- **The same buffer as source and destination** (`SUBSTITUTE`): works, since the
  whole result is made before it is copied.

## Extended-Character (18.4.1)

Implementation-defined options:

- **Unicode input and display**: xchars are UTF-8 (`ENVIRONMENT? XCHAR-ENCODING` gives
  `UTF-8`), up to $10FFFF in at most 4 bytes. Input methods, the writing direction (left
  to right or right to left) and the glyphs are up to the terminal; forthling passes the
  bytes on unchanged. `XC-WIDTH` and `X-WIDTH` know the common Unicode ranges (0 columns
  for control characters and combining marks, 2 for wide East Asian characters and
  emoji, 1 for the rest), not the full tables.
- **Xchars in files and strings**: files are read and written as bytes, so UTF-8 passes
  through unchanged. A character stays one byte (`MAX-CHAR` is 255): `KEY`, `EMIT`, `C@`
  and the string words work on bytes, and `EKEY>CHAR` is true for ASCII only. `CHAR`,
  `[CHAR]`, `'x'` and `PARSE` take whole xchars.

Ambiguous conditions:

- **Memory that holds no valid xchar** (`X-SIZE` and the other words): malformed UTF-8 is
  read a byte at a time; a byte that does not start a complete xchar is an xchar of its
  own, with the byte as its value.
- **An xchar outside the allowed code points**: writing an xchar above $10FFFF (`XEMIT`,
  `XC!+`, `XC,`, `XHOLD`) writes U+FFFD, the replacement character.
- **`XHOLD` outside `<#` and `#>`**: as for `HOLD` (see Core).

## Extensions

A Rust program that embeds forthling can add words of its own through the host API
(`Extension`, `Forth::wordlist` and `Forth::add_word`; see the README). This leaves the
standard words as they are:

- An extension puts its words in a word list of its own, which a program searches with
  `ALSO NAME`. The only word it adds to `FORTH-WORDLIST` is `NAME`, which replaces the
  first word list of the search order with it, as `FORTH` does.
- `ENVIRONMENT?` knows each installed extension by its name, such as `FORTHLING-SQLITE`,
  and then gives its version and true. `[DEFINED]` tests for a single word.
- The words of an extension belong to the system: `FORGET` throws -15 for them.

The forthling program itself installs no extensions. The Python module (`python/`) installs
`FORTHLING-PYTHON` (version 1), without words of its own: the words that a Python program
adds go into `FORTH-WORDLIST` or into a word list it names. A Python exception in such a
word throws -257, which `CATCH` catches; uncaught, the Python program gets the exception
itself back.
