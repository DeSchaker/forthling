// forthling in JavaScript: a thin layer over the exports of forthling.wasm.
//
//   import { load } from "./forthling.js";
//   const forth = await load("forthling.wasm");
//   forth.eval(": SQUARE DUP * ; 7 SQUARE .");   // { output: "49 ", stack: [], ... }
//
// Each answer is { output, stack, fstack, compiling, error }, where error is
// null or { code, message, detail, text }. One module is one session: words
// defined in one eval stay for the next, until reset().

const encoder = new TextEncoder();
const decoder = new TextDecoder();

export class Forth {
  #wasm;

  constructor(instance) {
    this.#wasm = instance.exports;
  }

  /** Runs Forth code; returns the answer. */
  eval(code) {
    const bytes = encoder.encode(code);
    const at = this.#wasm.fl_input(bytes.length);
    new Uint8Array(this.#wasm.memory.buffer, at, bytes.length).set(bytes);
    return this.#answer(this.#wasm.fl_eval(Date.now() / 1000));
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
