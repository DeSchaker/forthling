# A command-line tool: arguments with NEXT-ARG

*Book 3, Data and tools. Needs the `OPERATING-SYSTEM` words, so it runs in the `forthling`
program but not in the playground.*

The program: [`examples/book003_example004_cli_tool.fs`](../examples/book003_example004_cli_tool.fs).
Its output is beside it, in `book003_example004_cli_tool.out`. Run it with:

```
cargo run --release -- examples/book003_example004_cli_tool.fs
```

It is a small tool called `lines` that shows files with a line number before each line (`-n`)
or only counts the lines (`-c`). Run without arguments it prints a usage text and shows
itself at work on a file it writes. It takes under 2000 steps (found by lowering
`--max-steps` until it stopped).

```
$ lines -n demo.txt
  1: first line
  2: second line
  3: third line

$ lines -c demo.txt
demo.txt: 3

$ lines missing.txt
lines: cannot read missing.txt
```

## The arguments of a script

Run a script with more words after it, and the `forthling` program takes them as arguments for
the script. Put `--` before them, so that the program does not read `-n` as an option of its
own:

```
forthling lines.fs -- -n notes.txt more.txt
```

`NEXT-ARG ( -- c-addr u )` takes the first argument that is left and gives it as a string.
When there are none left, it gives `0 0`. So a loop over all the arguments is short:

```forth
: MAIN ( -- )
  BEGIN  ARGUMENT DUP WHILE
    2DUP S" -n" COMPARE 0= IF  2DROP -1 NUMBERING !  ELSE
    2DUP S" -c" COMPARE 0= IF  2DROP -1 COUNTING !   ELSE
      SHOW-FILE
    THEN THEN
  REPEAT  2DROP ;
```

An option sets a flag and any other word is a file name. The words `ARGC` and `ARG` look at
the arguments without taking them (`ARGC @` counts the program name too, so it is 1 when
there are no arguments).

## Why ARGUMENT is a DEFER

A cookbook program must give the same output each time, and the tests run it without
arguments. So the program shows the tool at work on a fake command line: a string such as
`-n demo.txt`. `ARGUMENT` is a deferred word, with `NEXT-ARG` as its action:

```forth
DEFER ARGUMENT
' NEXT-ARG IS ARGUMENT
```

`FAKE-LINE` points it at `FAKE-ARG`, which hands out the words of a string one by one.
`MAIN` is the same in both cases, and it is the real tool. The last line chooses:

```forth
: RUN ( -- )  ARGC @ 1 = IF  DEMO  ELSE  MAIN  THEN ;
```

## Reading a file line by line

`SHOW-FILE` opens the file and reads it with `READ-LINE`, which gives the length of the line
and a flag that is false at the end. A file that does not open gives an error message and the
tool goes on with the next argument. Numbers are made with `<# #S #>` and padded with
`SPACES` to a width of three.

## Two things to know

- Options act on the files after them: `lines demo.txt -c` shows `demo.txt` as it is and
  counts only the files that come after the `-c`. That is how many small tools work. To make
  the options count for all files, read all the arguments first and handle the files after.
- Without `--` the program reads `-n` as one of its own options and stops with
  `unknown option -n`.

## Try it

- Add a `-w` option that counts words as well.
- Read the arguments first into an array (see the word count chapter), then handle the files.
- Make the tool read standard input when there is no file name, with `STDIN` and `READ-LINE`.
- Print `Usage` and stop when an option is not known.
