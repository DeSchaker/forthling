// Runs forthling away from the page, so that the page never freezes; the
// page can always stop this worker and start a new one.

import { load } from "./forthling.js";

const forth = await load(new URL("forthling.wasm", import.meta.url));

onmessage = ({ data: { id, action, code } }) => {
  let answer;
  try {
    if (action === "eval") answer = forth.eval(code);
    else if (action === "reset") answer = forth.reset();
    else answer = forth.words();
  } catch (e) {
    answer = { crashed: String(e) }; // a trap in the module: the page starts anew
  }
  postMessage({ id, answer });
};

postMessage({ ready: true });
