# The language

What the forthling system offers as a Forth, and how it follows the standard. Back to the [README](../README.md).

## Features

- A REPL and file execution (nested with `INCLUDE`, `INCLUDED` and `REQUIRE`)
- File access: `OPEN-FILE CREATE-FILE READ-LINE READ-FILE WRITE-LINE WRITE-FILE`,
  `FILE-POSITION REPOSITION-FILE FILE-SIZE RESIZE-FILE`, `DELETE-FILE RENAME-FILE` and more
- Colon definitions compiled to threaded code, with `RECURSE`, `IMMEDIATE`,
  `POSTPONE`, `LITERAL`, `[ ]`, `' EXECUTE` and `SEE`
- `IF ELSE THEN`, `BEGIN UNTIL/AGAIN/WHILE REPEAT`, `DO ?DO LOOP +LOOP LEAVE`,
  `CASE OF ENDOF ENDCASE`
- Locals: `{: a b | c -- out :}`, `LOCALS|` and `(LOCAL)`, with `TO` for assigning
- `VARIABLE`, `CONSTANT`, `VALUE`, `DEFER`, `BUFFER:`, `MARKER`, `:NONAME`, `CREATE … DOES>`
- Quotations (Forth 2019): `[: … ;]` makes a nameless word and leaves its xt, inside a
  definition or at the prompt: `3 [: 2 * ;] EXECUTE .` prints `6`
- Byte-addressed memory with 64-bit cells, and a heap: `ALLOCATE FREE RESIZE`
- Double-cell numbers: `123.` is a double, with `D+ D- D. D.R D< DNEGATE DABS M+ M*/`,
  `2VALUE`, `2LITERAL` and more
- Structures (`BEGIN-STRUCTURE FIELD: CFIELD: +FIELD END-STRUCTURE`), `MS`, `TIME&DATE`,
  `KEY?`, and `AT-XY`/`PAGE` for the screen
- `SOURCE`, `>IN`, `STATE`, `WORD`, `PARSE`, `REFILL`, `SAVE-INPUT`, `FIND` and `EVALUATE`,
  working on input buffers in Forth memory
- JSON: read it in place (`JSON@` with a path such as `users[0].name`) and write it to the
  output or to a string (see [JSON](wordlists.md#json))
- A sandbox for code you do not trust: limits on steps, time and output, and no file
  access (see [Sandbox](embedding.md#sandbox))
- Errors as standard THROW codes, e.g. `demo.fs:3: error -13: undefined word: FOO`,
  which `CATCH` can catch
- Strings: `COMPARE SEARCH /STRING -TRAILING BLANK CMOVE CMOVE> SLITERAL`, and text
  substitution with `REPLACES SUBSTITUTE UNESCAPE`
- Blocks: `BLOCK BUFFER UPDATE FLUSH SAVE-BUFFERS EMPTY-BUFFERS LOAD THRU LIST`, kept in
  a block file
- Floating point: `1.5E3` is a float, on a stack of its own, with `F+ F* F/ F. FS. FE.`,
  `FSQRT FSIN FEXP FLN F** FATAN2` and more, `FVARIABLE FCONSTANT FVALUE`, `>FLOAT` and
  `REPRESENT`
- Extended characters in UTF-8: `XC@+ XC!+ XCHAR+ XEMIT XKEY X-SIZE X-WIDTH XHOLD` and
  more; `CHAR`, `[CHAR]`, `'€'` and `PARSE` take whole characters
- Word lists and the search order: `WORDLIST GET-ORDER SET-ORDER ALSO ONLY PREVIOUS
  DEFINITIONS ORDER SEARCH-WORDLIST`, and `TRAVERSE-WORDLIST NAME>STRING`
- Modules: `MODULE name ... EXPORT ... END-MODULE` keeps helper words private and makes the
  rest a word list; `USING name` opens it (see [Modules](wordlists.md#modules))
- Programming tools: `.S ? DUMP SEE WORDS`, `[IF] [ELSE] [THEN]`, `[DEFINED] [UNDEFINED]`,
  `AHEAD CS-PICK CS-ROLL`, `N>R NR>` and `SYNONYM`
- Words inspired by other Forths: the loops `+DO -DO U+DO U-DO FOR NEXT -LOOP ?LEAVE ?EXIT`, strings (`BOUNDS PLACE STR=
  SCAN SKIP S+`), stack and comparison words (`2NIP THIRD <= U<= UMAX`), `ROL ROR CTZ LOG2`,
  `[IFDEF] [IFUNDEF]`, `ALIAS`, `VOCABULARY`, `RANDOM`, `UTIME`, `SLURP-FILE` and more (see
  [Words inspired by other Forths](../COMPLIANCE.md#words-inspired-by-other-forths))

Part of the dictionary is written in Forth itself (`src/prelude.fs`).

## Standard compliance

The open work is in the
[issues on GitHub](https://github.com/DeSchaker/forthling/issues). The CORE, CORE EXT, BLOCK, DOUBLE, EXCEPTION, FACILITY, FILE, LOCALS,
MEMORY, SEARCH, STRING, TOOLS, XCHAR and FLOATING word sets are complete, and their tests in the
[Forth-2012 test suite](https://github.com/gerryjackson/forth2012-test-suite) pass (`core.fr`,
`coreplustest.fth`, `coreexttest.fth`, `blocktest.fth`, `doubletest.fth`, `exceptiontest.fth`,
`facilitytest.fth`, `filetest.fth`, `localstest.fth`, `memorytest.fth`,
`searchordertest.fth`, `stringtest.fth`, `toolstest.fth` and all floating-point tests in
`fp/`, the paranoia test included). Of TOOLS EXT, everything is
present except `;CODE ASSEMBLER CODE EDITOR` (there is no assembler or editor). The test
suite has no XCHAR tests, so forthling has its own, in the same style:
[`tests/xchartest.fth`](../tests/xchartest.fth).

The system documentation that section 4.1 of the standard asks for (every
implementation-defined option, how forthling acts on each ambiguous condition, and the
other facts about the system) is in [`COMPLIANCE.md`](../COMPLIANCE.md), with a Dutch
translation in [`VERKLARING.md`](../VERKLARING.md). Those files also list the main
implementation-defined choices, such as 64-bit cells, floored division, UTF-8 for extended
characters and the sizes of the stacks and of memory.
