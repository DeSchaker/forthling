# A native window for forthling

The word list `WINDOW` for [forthling](https://github.com/DeSchaker/forthling), through
[minifb](https://github.com/emoon/rust_minifb). `SHOW-WINDOW` opens a window with the
picture of the canvas (the word list `IMAGE`) and waits until the window is closed or Esc is
pressed. A canvas smaller than 1280 x 800 is enlarged by a whole factor (up to 16), a larger
one is shown as it is; the window can be resized and keeps the aspect ratio.

A window reaches the machine, so it needs the permission `System`, which
`Forth::sandboxed` denies. It is not in the MCP server, the Python module or the
playground (the playground shows the picture on the page).

## Running

`forthling-window` is forthling with these words, the canvas words and the fixed-point
words installed:

```
cargo run --release --manifest-path window/Cargo.toml -- picture.fs
```

with `picture.fs`:

```
ALSO IMAGE ALSO WINDOW
64 64 CANVAS
: DRAW 64 0 DO 64 0 DO I J I J XOR 4 * PIXEL LOOP LOOP ;
DRAW SHOW-WINDOW
```

In a Rust program: `forth.install(&forthling::Canvas)` and `forth.install(&forthling_window::Window)`.
`S" FORTHLING-WINDOW" ENVIRONMENT?` gives `1 true` when the words are there.

## Words

| Word | Stack | |
|------|-------|---|
| `SHOW-WINDOW` | `( -- )` | open a window with the canvas and wait until it is closed or Esc is pressed; `-24` when there is no canvas, `-259` without the permission, `-263` when the window cannot be opened |

Not done: the window only shows the picture as it is when the word is called; it does not
update while the program draws, and it gives no keys to `KEY`.
