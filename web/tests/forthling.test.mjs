// Tests of the WebAssembly module through forthling.js; run with
//   node --test web/tests/forthling.test.mjs
// after building it (see web/README.md).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { load } from "../www/forthling.js";

const wasm = new URL("../target/wasm32-unknown-unknown/release/forthling_web.wasm", import.meta.url);
const bytes = await readFile(wasm);
const fresh = () => load(bytes);

test("a session keeps its definitions", async () => {
  const forth = await fresh();
  let r = forth.eval(": SQUARE DUP * ;\n7 SQUARE");
  const { steps, max_depth, ...rest } = r;
  assert.deepEqual(rest, { output: "", stack: [49], fstack: [], compiling: false, error: null });
  assert.ok(steps > 0 && max_depth === 2, `${steps} ${max_depth}`);
  r = forth.eval("SQUARE . 1.5E0 2E0 F*");
  assert.equal(r.output, "2401 ");
  assert.deepEqual(r.fstack, [3.0]);
  assert.equal(forth.eval(": HALF 2 /").compiling, true);
  forth.eval(";");
  assert.deepEqual(forth.words(), ["HALF", "SQUARE"]);
  assert.deepEqual(forth.reset().stack, []);
  assert.deepEqual(forth.words(), []);
});

test("errors come back as data", async () => {
  const forth = await fresh();
  const r = forth.eval(".\" before\" 1 2 NOSUCH");
  assert.equal(r.output, "before");
  assert.deepEqual(r.error, {
    code: -13, message: "undefined word", detail: "NOSUCH", hint: null,
    text: "error -13: undefined word: NOSUCH",
    stack: [1, 2], fstack: [], words: [], source: "input", line: 1, column: 16, token: "NOSUCH",
  });
  assert.deepEqual(r.stack, []);
  // A word in a word list outside the search order gets a hint.
  assert.equal(forth.eval("TRACE-ON").error.hint, "it is in the word list DEBUG; ALSO DEBUG finds it");
  forth.eval(": STEP 0 / ;\n: MAIN 1 2 STEP ;");
  const e = forth.eval("5\n  MAIN").error;
  assert.deepEqual([e.words, e.stack, e.line, e.column], [["MAIN", "STEP", "/"], [5, 1], 2, 3]);
});

test("the sandbox holds in the browser", async () => {
  const forth = await fresh();
  const started = Date.now();
  assert.equal(forth.eval(": FOREVER BEGIN AGAIN ; FOREVER").error.code, -258);
  assert.ok(Date.now() - started < 10_000);
  // Files are in memory only.
  assert.equal(forth.eval("INCLUDE x.fs").error.code, -38);
  assert.equal(forth.eval("S\" x.txt\" R/O OPEN-FILE . .").output, "-69 0 ");
  assert.equal(forth.eval("KEY").error.code, -39);
  assert.equal(forth.eval(": STARS 2000000 0 DO 42 EMIT LOOP ; STARS").error.detail, "output");
  assert.equal(forth.eval("1 2 + .").output, "3 ");
});

test("files live in the session's memory", async () => {
  const forth = await fresh();
  forth.writeFile("lib/square.fs", ": SQUARE DUP * ;");
  forth.writeFile("main.fs", "REQUIRE lib/square.fs 7 SQUARE .");
  assert.equal(forth.eval("INCLUDE main.fs").output, "49 ");
  forth.eval("S\" out.txt\" W/O CREATE-FILE THROW S\" héllo\" ROT WRITE-LINE THROW");
  assert.equal(forth.readFile("out.txt"), "héllo\n");
  assert.equal(forth.readFile("none.txt"), null);
  assert.equal(forth.deleteFile("none.txt"), false);
  assert.deepEqual(forth.files(), [
    { name: "lib/square.fs", size: 16 }, { name: "main.fs", size: 32 }, { name: "out.txt", size: 7 },
  ]);
  // Blocks are a file there too.
  forth.eval("1 BLOCK 1024 BLANK S\" 6 7 * .\" 1 BLOCK SWAP MOVE UPDATE SAVE-BUFFERS");
  assert.equal(forth.eval("EMPTY-BUFFERS 1 LOAD").output, "42 ");
  assert.throws(() => forth.writeFile("big", "x".repeat(4 * 1024 * 1024)), /full/);
  assert.equal(forth.deleteFile("main.fs"), true);
  assert.equal(forth.eval("INCLUDE main.fs").error.code, -38);
  forth.reset();
  assert.deepEqual(forth.files(), []);
});

test("the tutorial", async () => {
  const forth = await fresh();
  const lessons = forth.lessons();
  assert.equal(lessons.length, 20);
  assert.deepEqual(Object.keys(lessons[0]), ["number", "title", "level", "example"]);
  assert.equal(lessons[3].title, "Defining words");
  let r = forth.tutorial("LESSON", 4);
  assert.ok(r.output.includes("Lesson 4 of 20, beginner: Defining words"));
  assert.deepEqual([r.lesson, r.passed], [4, []]);
  forth.eval(": CUBE DUP DUP * * ;");
  r = forth.tutorial("CHECK");
  assert.ok(r.output.startsWith("Well done!"), r.output);
  assert.deepEqual(r.passed, [4]);
  assert.ok(forth.tutorial("NEXT").output.includes("Lesson 5 of 20"));
  assert.equal(forth.tutorial("LESSON", 21).error.code, -24);
  // In the code it takes ALSO TUTORIAL.
  assert.ok(forth.eval("ALSO TUTORIAL HINT").output.startsWith("Hint:"));
});

test("what WebAssembly lacks is handled", async () => {
  const forth = await fresh();
  // MS cannot sleep there; it does nothing.
  assert.equal(forth.eval("1000 MS 1 .").output, "1 ");
  // TIME&DATE has the time of the page.
  const year = new Date().getUTCFullYear();
  assert.deepEqual(forth.eval("TIME&DATE").stack.slice(-1), [year]);
  // Deep nesting fits the stack of the module.
  const nest = "DEFER DEEP : (DEEP) DUP IF 1- ['] DEEP CATCH THROW THEN ; ' (DEEP) IS DEEP 900 DEEP .";
  const deep = forth.eval(nest);
  assert.deepEqual([deep.error, deep.output], [null, "0 "]);
  assert.equal(forth.eval("ALSO JSON S\" [1,[2,3]]\" S\" [1][0]\" JSON@ DROP TYPE").output, "2");
  assert.equal(forth.eval("ALSO FUNCTIONAL 1 6 RANGE [: DUP * ;] MAP 5 ' + FOLD .").output, "55 ");
  // The trace words are installed.
  forth.reset();
  assert.equal(forth.eval("ALSO DEBUG : SQ DUP * ; 3 TRACE-ON SQ TRACE-OFF").output,
    "SQ <1> 3\n  DUP <1> 3\n  * <2> 3 3\n");
  // A run can be recorded word by word, for Step through; BREAK is in it.
  const traced = forth.eval(": F SQ BREAK 1+ ; 4 F", { trace: 3 });
  assert.deepEqual(traced.stack, [9, 17]);
  assert.deepEqual(traced.trace, [
    { word: ":", depth: 0, stack: [9], stack_depth: 1 },
    { word: "F", depth: 0, stack: [9, 4], stack_depth: 2 },
    { word: "SQ", depth: 1, stack: [9, 4], stack_depth: 2 },
  ]);
  assert.equal(traced.trace_dropped, 4); // DUP, *, BREAK, 1+
  assert.equal(forth.eval("1 DROP").trace, undefined);
  // Floats, double cells and big numbers come through.
  assert.equal(forth.eval("2E0 FSQRT F. 1. 2. D+ D. -9223372036854775808 .").output,
    "1.4142135623731 3 -9223372036854775808 "); // as natively
});

test("regular expressions", async () => {
  const forth = await fresh();
  const r = forth.eval('ALSO REGEX S" order 66" S" [0-9]+" RE-FIND . . .');
  assert.equal(r.output, "-1 2 6 ");
  assert.equal(forth.eval('S" a" S" (" RE-MATCH?').error.code, -262);
});

test("the benchmark words use the clock of the page, in microseconds", async () => {
  const forth = await fresh();
  // The clock is read at each use, so it advances during one call.
  let r = forth.eval("UTIME 2DROP : SPIN 300000 0 DO I DROP LOOP ; ALSO BENCHMARK ' SPIN BENCH");
  assert.equal(r.error, null);
  assert.equal(r.stack.length, 1);
  assert.ok(r.stack[0] > 0 && r.stack[0] < 5_000_000, `BENCH gave ${r.stack[0]}`);
  r = forth.eval("DROP 3 ' SPIN BENCH-BEST");
  assert.equal(r.error, null);
  assert.ok(r.stack[0] > 0, `BENCH-BEST gave ${r.stack[0]}`);
  // UTIME is in microseconds since 1970, not whole seconds.
  r = forth.eval("DROP UTIME DROP");
  assert.ok(Math.abs(r.stack[0] / 1000 - Date.now()) < 5000, `UTIME gave ${r.stack[0]}`);
  assert.equal(forth.eval("999999999 BENCH").error.code, -24);
});

test("a run that draws on the canvas hands over its pixels once", async () => {
  const forth = await fresh();
  assert.equal(forth.eval("1 2 +").canvas, undefined);
  let r = forth.eval("ALSO IMAGE 3 2 CANVAS 1 0 255 0 0 RGB PIXEL");
  assert.equal(r.error, null);
  assert.equal(r.canvas.width, 3);
  assert.equal(r.canvas.height, 2);
  assert.deepEqual([...r.canvas.rgba], [
    0, 0, 0, 255, 255, 0, 0, 255, 0, 0, 0, 255,
    0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255,
  ]);
  // Nothing drawn since: no picture in the answer.
  assert.equal(forth.eval("1 2 +").canvas, undefined);
  r = forth.eval("0 0 2 1 0 0 255 RGB LINE");
  assert.deepEqual([...r.canvas.rgba.slice(0, 4)], [0, 0, 255, 255]);
});

// The playground examples that are copies of programs in examples/ (issue #77): the text must be
// the file, and running it in the module must give the output in the .out file beside it.
// The only change allowed is that the lines about the command line ("Run: cargo run ..." and
// "Needs the core only ...") are left out. Mandelbrot is left out of the check on purpose: its
// picture is smaller in the playground, to stay within the step limit, and it draws on a canvas.
const forPlayground = (text) =>
  text.replace(/\n$/, "").split("\n")
    .filter((l) => !/^\\ (Run: cargo run|Needs the core only)/.test(l))
    .map((l) => l.replace(/\s+Run: cargo run.*$/, ""))
    .join("\n");

test("the playground examples match the programs in examples/", async () => {
  const page = await readFile(new URL("../www/index.html", import.meta.url), "utf8");
  const start = page.indexOf("const examples = {");
  const end = page.indexOf("\n};", start);
  assert.ok(start >= 0 && end > start, "the examples object is not in index.html");
  const examples = new Function(`return ${page.slice(start + "const examples = ".length, end + 2)}`)();
  const copies = {
    Maze: "book001_example001_maze",
    Sudoku: "book001_example002_sudoku",
    Simplex: "book001_example003_simplex",
  };
  for (const [name, stem] of Object.entries(copies)) {
    const dir = new URL("../../examples/", import.meta.url);
    const program = (await readFile(new URL(`${stem}.fs`, dir), "utf8")).replace(/\r\n/g, "\n");
    const expected = (await readFile(new URL(`${stem}.out`, dir), "utf8")).replace(/\r\n/g, "\n");
    assert.equal(examples[name], forPlayground(program), `${name} differs from examples/${stem}.fs`);
    const r = (await fresh()).eval(examples[name]);
    assert.equal(r.error, null, `${name}: ${JSON.stringify(r.error)}`);
    assert.equal(r.output, expected, `${name} gives other output than examples/${stem}.out`);
  }
});
