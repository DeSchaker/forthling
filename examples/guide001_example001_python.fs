\ guide001_example001_python.fs - the snippets of "Forth for people who know Python", run.
\ Run: cargo run --release -- examples/guide001_example001_python.fs
\ Needs the core only (the word lists FUNCTIONAL, COLLECTIONS, STRINGS and DYNAMIC-STRINGS).
\
\ Each part has the Python on one line in a comment and the Forth under it. The chapter
\ cookbook/guide001_python.md explains them. The program prints one short line for each part,
\ so you see that the Forth gives the answer the Python comment says.

ALSO FUNCTIONAL ALSO COLLECTIONS ALSO STRINGS ALSO DYNAMIC-STRINGS

: PART ( c-addr u -- )  CR ." -- " TYPE CR ;

\ ---------------------------------------------------------------------------------------
S" names and assignment" PART
\ x = 5; x = x + 1; print(x)
5 VALUE X
: BUMP ( -- )  X 1+ TO X ;
BUMP X .  CR
\ a, b = 1, 2; a, b = b, a        (the stack shows the result, the top last)
: SWAP-DEMO ( -- )  1 2 SWAP .S 2DROP ;
SWAP-DEMO CR

\ ---------------------------------------------------------------------------------------
S" functions" PART
\ def area(w, h): return w * h
: AREA ( w h -- n )  * ;
3 4 AREA . CR
\ the same with names, like Python's parameters
: AREA2 ( w h -- n )  {: w h :} w h * ;
3 4 AREA2 . CR
\ def divide(a, b): return a // b, a % b      (two results)
: DIVIDE ( a b -- r q )  /MOD ;
17 5 DIVIDE . . CR
\ Python's // and % floor, and so do / and MOD here
-7 2 / .  -7 2 MOD . CR

\ ---------------------------------------------------------------------------------------
S" if, for and while" PART
\ "big" if n > 10 else "small"
: SIZE ( n -- c-addr u )  10 > IF S" big" ELSE S" small" THEN ;
12 SIZE TYPE SPACE 3 SIZE TYPE CR
\ for i in range(5): print(i)
: COUNT-UP ( -- )  5 0 DO I . LOOP ;
COUNT-UP CR
\ i = 0
\ while i < 3: print(i); i += 1
: WHILE-DEMO ( -- )  0 BEGIN DUP 3 < WHILE DUP . 1+ REPEAT DROP ;
WHILE-DEMO CR
\ for i in range(10):
\     if i == 7: print(i); break
: BREAK-DEMO ( -- )  10 0 DO I 7 = IF I . LEAVE THEN LOOP ;
BREAK-DEMO CR

\ ---------------------------------------------------------------------------------------
S" lists" PART
\ v = []; v.append(30); v.append(10); v.append(20)
VEC-VARIABLE V
30 V VEC+!  10 V VEC+!  20 V VEC+!
\ len(v), v[1], v.pop()
V VEC# .  1 V VEC@ .  V VEC-POP . CR
\ v.append(20); v.sort()
20 V VEC+!
: SHOW-CELLS ( addr n -- )  0 DO DUP I CELLS + @ . LOOP DROP ;
V ' < VEC-SORT  V VEC-DATA SHOW-CELLS CR
\ sum(v), max(v)
V VEC-DATA ' + FOLD-CELLS .  V VEC-DATA ' MAX FOLD-CELLS . CR
\ v[1:3]       (from 1 up to, not including, 3)
: SLICE ( addr n from to -- addr' n' )  {: a n from to :} a from CELLS +  to from - ;
V VEC-DATA 1 3 SLICE SHOW-CELLS CR

\ ---------------------------------------------------------------------------------------
S" list comprehensions" PART
\ [x * x for x in range(1, 10) if x % 2 == 0]
: EVEN? ( n -- flag )  2 MOD 0= ;
: SQUARE ( n -- n^2 )  DUP * ;
1 10 RANGE ' EVEN? FILTER ' SQUARE MAP  .S 2DROP 2DROP CR
\ sum(x * x for x in range(1, 6))
1 6 RANGE ' SQUARE MAP 5 ' + FOLD .  CR
\ any(x > 2 for x in [1, 2, 3]), all(x > 0 for x in [1, 2, 3])
: >2? ( n -- flag )  2 > ;
: >0? ( n -- flag )  0> ;
1 2 3 3 ' >2? ANY? .  1 2 3 3 ' >0? ALL? . CR
\ min(3, 1, 4), max(3, 1, 4)
3 1 4 3 MIN-OF .  3 1 4 3 MAX-OF . CR

\ ---------------------------------------------------------------------------------------
S" dictionaries" PART
\ ages = {"alice": 30, "bob": 25}; ages["carol"] = 41
DICT-VARIABLE AGES
30 S" alice" AGES DICT!  25 S" bob" AGES DICT!  41 S" carol" AGES DICT!
\ ages["alice"]    and    "dave" in ages
S" alice" AGES DICT@ . .   S" dave" AGES DICT@ . CR
\ del ages["bob"]; len(ages)
S" bob" AGES DICT-DELETE .  AGES DICT# . CR
\ for name, age in ages.items(): print(name, age)
: SHOW-AGE ( c-addr u age -- )  >R TYPE ." =" R> . ;
: SHOW-AGES ( -- )  AGES ['] SHOW-AGE DICT-EACH ;
SHOW-AGES CR

\ ---------------------------------------------------------------------------------------
S" strings" PART
\ s = "  Hello, World  "; s.strip()
: TEXT ( -- c-addr u )  S"   Hello, World  " ;
TEXT TRIM TYPE CR
\ len(s), s[2:7], s[-5:]
TEXT TRIM NIP .  TEXT TRIM 2 /STRING 5 MIN TYPE  SPACE  TEXT TRIM DUP 5 - /STRING TYPE CR
\ "World" in s, s.startswith("Hello")
TEXT S" World" CONTAINS? .  TEXT TRIM S" Hello" STRING-PREFIX? . CR
\ "a,b,c".split(",")
$VARIABLE CSV
: PRINT-PART ( c-addr u -- )  TYPE ." ;" ;
S" a,b,c" CSV $!  CSV 44 ' PRINT-PART $ITER CR
\ ",".join(["a", "b", "c"])
$[]VARIABLE LETTERS
S" a" LETTERS $[]+!  S" b" LETTERS $[]+!  S" c" LETTERS $[]+!
LETTERS S" , " JOIN 2DUP TYPE DROP FREE THROW CR
\ s.upper()
CREATE BUF 20 ALLOT
S" forthling" BUF SWAP CMOVE  BUF 9 UPPER TYPE CR

\ ---------------------------------------------------------------------------------------
S" f-strings" PART
\ f"Dear {name}, you owe {amount} euro."
CREATE OUT 100 ALLOT
: NUM ( n -- c-addr u )  S>D <# #S #> ;
: LETTER ( -- c-addr u )
  S" Ada" S" name" REPLACES
  1250 NUM S" amount" REPLACES
  S" Dear %name%, you owe %amount% euro." OUT 100 SUBSTITUTE DROP ;
LETTER TYPE CR
\ f"{3.14159:.2f}"   and   f"{n:05d}"
: 2F ( F: x -- ) ( -- c-addr u )  100E F* FROUND F>S S>D <# # # 46 HOLD #S #> ;
3.14159E 2F TYPE CR
: N05 ( n -- c-addr u )  S>D <# # # # # # #> ;
42 N05 TYPE CR

\ ---------------------------------------------------------------------------------------
S" exceptions" PART
\ try: 1 // 0   except ZeroDivisionError: ...
: SAFE-DIV ( a b -- q )  ['] / CATCH IF 2DROP ." (division by zero) " 0 THEN ;
1 0 SAFE-DIV .  6 3 SAFE-DIV . CR
\ raise ValueError("bad")     and     except ValueError
99 CONSTANT BAD-VALUE
: CHECK ( n -- n )  DUP 0< IF BAD-VALUE THROW THEN ;
: TRY-CHECK ( n -- )  ['] CHECK CATCH DUP BAD-VALUE = IF DROP DROP ." bad value " ELSE DROP . THEN ;
5 TRY-CHECK  -5 TRY-CHECK CR

\ ---------------------------------------------------------------------------------------
S" with" PART
\ with resource() as r: body(r)     the resource is closed even when the body raises
VARIABLE OPEN-COUNT
: WITH-BUFFER ( xt -- )   \ xt is ( addr -- )
  100 ALLOCATE THROW  1 OPEN-COUNT +!
  DUP >R SWAP CATCH
  R> FREE THROW  -1 OPEN-COUNT +!
  THROW ;
: WRITE-ONE ( addr -- )  65 SWAP C!  ." wrote " ;
: FAIL-AFTER ( addr -- )  DROP ." about to fail " 77 THROW ;
' WRITE-ONE WITH-BUFFER  OPEN-COUNT @ . CR
' FAIL-AFTER ' WITH-BUFFER CATCH . OPEN-COUNT @ . CR

\ ---------------------------------------------------------------------------------------
S" numbers" PART
\ 7 // 2 is 3, and 7 / 2 is 3.5
7 2 / .  7E 2E F/ F. CR
\ True and False are -1 and 0, and any other number is true in an IF
3 4 < .  3 4 > . CR
\ print("total:", 17); print("done")
.( total: ) 17 . CR
