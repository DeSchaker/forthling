// forthling in JavaScript: a thin layer over the exports of forthling.wasm.
//
//   import { load } from "./forthling.js";
//   const forth = await load("forthling.wasm");
//   forth.eval(": SQUARE DUP * ; 7 SQUARE .");   // { output: "49 ", stack: [], ... }
//
// Each answer is { output, stack, fstack, compiling, error }, where error is
// null or { code, message, detail, text }. One module is one session: words
// defined in one eval stay for the next, until reset().
//
//   forth.eval(code, { trace: 10000 })   // also records the words that ran:
//   // trace: [{ word, depth, stack, stack_depth }, ...], trace_dropped
//
// The session's files are in memory (at most 4 MiB); INCLUDE, the FILE words
// and BLOCK use them, and so can the page:
//
//   forth.writeFile("square.fs", ": SQUARE DUP * ;");
//   forth.eval("INCLUDE square.fs 7 SQUARE .");   // output: "49 "
//   forth.readFile("square.fs");                    // the text, or null
//   forth.files();                                  // [{ name, size }, ...]

const encoder = new TextEncoder();
const decoder = new TextDecoder();

export class Forth {
  #wasm;

  constructor(instance) {
    this.#wasm = instance.exports;
  }

  /**
   * Runs Forth code; returns the answer. With `trace`, a number, the answer
   * also has the first that many words that ran, each with how deep it was
   * nested and the data stack when it started (the top 8 cells).
   */
  eval(code, { trace = 0 } = {}) {
    const bytes = encoder.encode(code);
    const at = this.#wasm.fl_input(bytes.length);
    new Uint8Array(this.#wasm.memory.buffer, at, bytes.length).set(bytes);
    this.#wasm.fl_record(trace);
    try {
      return this.#answer(this.#wasm.fl_eval(Date.now() / 1000));
    } finally {
      this.#wasm.fl_record(0);
    }
  }

  /** The state, without running anything. */
  state() {
    return this.#answer(this.#wasm.fl_state());
  }

  /** Starts a new session; returns its state. */
  reset() {
    return this.#answer(this.#wasm.fl_reset());
  }

  /** The words defined in the session, the latest first. */
  words() {
    return this.#answer(this.#wasm.fl_words());
  }

  /** Puts a file in the session's files; throws when it does not fit. */
  writeFile(name, text) {
    const nameBytes = encoder.encode(name);
    const contents = typeof text === "string" ? encoder.encode(text) : text;
    this.#input(nameBytes.length + contents.length, (bytes) => {
      bytes.set(nameBytes);
      bytes.set(contents, nameBytes.length);
    });
    const failed = this.#answer(this.#wasm.fl_write_file(nameBytes.length));
    if (failed !== null) throw new Error(failed);
  }

  /** The text of a file of the session, or null when there is none. */
  readFile(name) {
    const bytes = encoder.encode(name);
    this.#input(bytes.length, (input) => input.set(bytes));
    return this.#answer(this.#wasm.fl_read_file());
  }

  /** The session's files: [{ name, size }], sorted by name. */
  files() {
    return this.#answer(this.#wasm.fl_files());
  }

  #input(length, fill) {
    const at = this.#wasm.fl_input(length);
    fill(new Uint8Array(this.#wasm.memory.buffer, at, length));
  }

  #answer(length) {
    // The memory may have grown, so the view is made anew each time.
    const bytes = new Uint8Array(this.#wasm.memory.buffer, this.#wasm.fl_answer(), length);
    return JSON.parse(decoder.decode(bytes));
  }
}

/** Loads forthling from a URL, a Response, or the bytes of the .wasm. */
export async function load(source) {
  if (typeof source === "string" || source instanceof URL) {
    source = fetch(source);
  }
  source = await source;
  const { instance } = source instanceof Response
    ? await WebAssembly.instantiateStreaming(source, {})
    : await WebAssembly.instantiate(source, {});
  return new Forth(instance);
}
