# forthling for Python

The [forthling](https://github.com/DeSchaker/forthling) Forth interpreter as a Python
module, built with [PyO3](https://pyo3.rs/) on its host API.

## Installing

From the root of the repository, with Rust installed:

```
pip install ./python
```

## Using

```python
import math
import forthling

f = forthling.Forth()
print(f.interpret(": SQUARE DUP * ; 7 SQUARE ."))   # 49

f.push(6)
f.push(7)
f.interpret("*")
print(f.pop(), f.stack)                             # 42 []

# A word written in Python: it gets two cells and pushes a float.
f.add_word("HYPOT", math.hypot, inputs=2)
print(f.interpret("3 4 HYPOT F."))                  # 5.

try:
    f.interpret("NOSUCHWORD")
except forthling.ForthError as e:
    print(e.code, e)                                # -13 error -13: undefined word: NOSUCHWORD
```

- `Forth(memory=None, input="", sandbox=False, max_steps=None, max_time=None,
  max_output=None)`: `memory` is the size in bytes (1 MiB by default); `input` is what
  `KEY`, `ACCEPT` and `REFILL` read from the terminal. `sandbox=True` is for code you do
  not trust: no file access, and each `interpret` is limited to 10 million steps, 5
  seconds and 1 MiB of output. `max_steps`, `max_time` (seconds) and `max_output` (bytes)
  set a limit of their own, with or without the sandbox. A limit that is reached raises
  `ForthError` with code -258, something forbidden -259.
- `interpret(text)` and `include(path)` return what was printed. An uncaught `THROW`
  raises `ForthError`, with `code` (the THROW code) and `output` (what was printed
  before it); the stacks are then empty.
- `push(n)`, `pop()`, `fpush(r)`, `fpop()`, and `stack` (a list, the top last).
- `add_word(name, func, inputs=0, wordlist=None)` adds a word that calls `func` with
  `inputs` cells from the stack, the deepest first. Its result is pushed: `None`
  nothing, a bool a flag (-1 or 0), an int a cell, a float on the float stack, a tuple
  or list each item in turn. With `wordlist="NAME"` the word goes into a word list of
  that name, which Forth searches after `ALSO NAME`. A Python exception in `func` is
  `THROW -257` in Forth; if nothing catches it, `interpret` raises the exception itself.
- `finished` tells whether `BYE` was executed.
- `S" FORTHLING-PYTHON" ENVIRONMENT?` gives `1 true` in Forth.

## Testing

```
pip install ./python
python -m unittest discover -s python/tests
```
