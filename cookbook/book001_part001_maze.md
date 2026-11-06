# A maze: make it and solve it

*Book 1, Puzzles. Needs the core only; it also runs in the playground.*

The program: [`examples/book001_example001_maze.fs`](../examples/book001_example001_maze.fs).
Its output is beside it, in `book001_example001_maze.out`. Run it with:

```
cargo run --release -- examples/book001_example001_maze.fs
```

In the playground, pick *Maze* under *Examples*.

It makes a maze of 12 x 6 rooms, prints it, finds the shortest way from `S` to `G` and
prints the maze again with the way marked by dots. The maze is always the same, because the
random number generator starts from a fixed seed (2024). Change the seed or the sizes
`CW` and `CH` at the top for another maze.

## The grid

A maze of 12 x 6 rooms needs a grid of 25 x 13 squares, because the walls take squares
too. The rooms are on the squares with odd coordinates; everything else starts as wall.
`GRID` holds one byte per square: 0 is open, 1 is a wall, 2 is on the solution.

```forth
CREATE GRID    GN ALLOT
: POS    ( x y -- i )  GW * + ;
: UNPOS  ( i -- x y )  GW /MOD ;
```

`POS` turns a square into an index in the table, and `UNPOS` turns it back. `/MOD` gives
the remainder first and the quotient second, which is exactly x and y.

## Generate: depth-first backtracking

Start in a room and walk to a room you have not visited, breaking down the wall between.
Keep every room you pass on a stack. When there is no new room next to you, go back one
room. When the stack is empty, every room has been visited and the maze is done.

The stack is a vector, `STACK`, from the `COLLECTIONS` word list:

```forth
VEC-VARIABLE STACK
: WALK  ( -- )
  BEGIN STACK VEC# WHILE
    STACK VEC# 1- STACK VEC@ UNPOS  2DUP CANDIDATES
    ?DUP IF  RND CAND + C@ GO  ELSE  2DROP STACK VEC-POP DROP  THEN
  REPEAT ;
```

`VEC#` is the number of elements, `VEC@` reads one, `VEC+!` adds one at the end and
`VEC-POP` takes the last one off. `WALK` looks at the room on top of the stack
(`STACK VEC# 1- STACK VEC@`), asks `CANDIDATES` how many neighbours are still unvisited,
and either goes to a random one (`GO`) or throws the room away.

`CANDIDATES` fills the small table `CAND` with the directions that lead to a new room
and gives how many there are. It uses local variables, `{: x y :}`, so that `x` and `y`
need not be juggled on the stack inside the loop. The four directions are two small
tables, `DXS` and `DYS`, that `STEP` reads: a step is two squares, from room to room.

## Random numbers

forthling has a `RANDOM` of its own, but this program writes a small generator, so that
the maze is the same on every machine and the output can be checked:

```forth
VARIABLE SEED   2024 SEED !
: RND  ( n -- r )  \ a number from 0 to n-1
  SEED @ 1103515245 * 12345 + 2147483647 AND DUP SEED !  16 RSHIFT SWAP MOD ;
```

This is a linear congruential generator. Each call multiplies the seed, adds a number and
keeps 31 bits. The high bits are the better ones, so `16 RSHIFT` drops the low ones
before `MOD` cuts the number down to the range asked for.

## Solve: breadth-first search

Breadth-first search visits the squares in rings: first the start, then the squares one
step away, then two steps away, and so on. The first time it reaches the goal, it got
there by a shortest way. A queue does the work, and that is again a vector: new squares are
added at the end with `VEC+!`, and a variable, `HEAD`, marks the front. Nothing is ever
removed, so the whole queue stays there to read.

```forth
: SOLVE  ( -- )
  GN 0 DO -1 I CELLS DIST + ! LOOP
  Q VEC-CLEAR  0 HEAD !
  START Q VEC+!  0 START CELLS DIST + !
  BEGIN HEAD @ Q VEC# < WHILE
    HEAD @ Q VEC@  1 HEAD +!  EXPAND
  REPEAT
  MARK-PATH ;
```

`DIST` holds the number of steps from the start, and -1 means "not seen yet". `EXPAND`
looks at the four squares next to the one taken from the queue; every open square not
seen yet gets its distance, its parent (`PARENT`, the square we came from) and a place in
the queue. When the queue runs dry, `MARK-PATH` walks back from the goal along `PARENT`
and writes a 2 on every square on the way.

## The result

```
##################################################
##S ............##  ##                  ##      ##
##############..##  ##  ######  ##########  ##  ##
##          ##..##      ##  ##              ##  ##
##  ##########..######  ##  ##################  ##
##  ##      ##......##              ##  ##......##
##  ##  ##  ######..##############  ##  ##..##..##
##      ##      ##..##              ##  ##..##..##
##  ######  ######..##############  ##  ##..##..##
##      ##      ##..##..........##      ##..##..##
######  ######  ##..##..######..##########..##..##
##          ##    ......##    ..............##G ##
##################################################
```

The shortest way takes 48 steps. Every square is printed two characters wide, so that the
maze looks square in a terminal.

## Words in this part

`VEC-VARIABLE`, `VEC+!`, `VEC@`, `VEC#`, `VEC-POP`, `VEC-CLEAR`, `VEC-FREE` (collections);
`{: ... :}` (local variables); `CREATE` with `,` and `ALLOT` (tables); `WITHIN`, `/MOD`,
`CASE`.

## Try this

- Make the maze bigger (`CW` and `CH`) and see that nothing else changes.
- Print only the way, as a list of squares, by walking `PARENT` from the goal.
- Put the goal in another corner, or in the middle.
