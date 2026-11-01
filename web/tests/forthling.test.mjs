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
  assert.deepEqual(forth.files(), [
    { name: "lib/square.fs", size: 16 }, { name: "main.fs", size: 32 }, { name: "out.txt", size: 7 },
  ]);
  // Blocks are a file there too.
  forth.eval("1 BLOCK 1024 BLANK S\" 6 7 * .\" 1 BLOCK SWAP MOVE UPDATE SAVE-BUFFERS");
  assert.equal(forth.eval("EMPTY-BUFFERS 1 LOAD").output, "42 ");
  assert.throws(() => forth.writeFile("big", "x".repeat(4 * 1024 * 1024)), /full/);
  forth.reset();
  assert.deepEqual(forth.files(), []);
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
