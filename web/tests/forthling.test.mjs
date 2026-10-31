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
  assert.deepEqual(r, { output: "", stack: [49], fstack: [], compiling: false, error: null });
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
    code: -13, message: "undefined word", detail: "NOSUCH",
    text: "error -13: undefined word: NOSUCH",
  });
  assert.deepEqual(r.stack, []);
});

test("the sandbox holds in the browser", async () => {
  const forth = await fresh();
  const started = Date.now();
  assert.equal(forth.eval(": FOREVER BEGIN AGAIN ; FOREVER").error.code, -258);
  assert.ok(Date.now() - started < 10_000);
  assert.equal(forth.eval("INCLUDE x.fs").error.code, -259);
  assert.equal(forth.eval("S\" x.txt\" R/O OPEN-FILE . .").output, "-259 0 ");
  assert.equal(forth.eval("KEY").error.code, -39);
  assert.equal(forth.eval(": STARS 2000000 0 DO 42 EMIT LOOP ; STARS").error.detail, "output");
  assert.equal(forth.eval("1 2 + .").output, "3 ");
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
  // Floats, double cells and big numbers come through.
  assert.equal(forth.eval("2E0 FSQRT F. 1. 2. D+ D. -9223372036854775808 .").output,
    "1.4142135623731 3 -9223372036854775808 "); // as natively
});
