// Runs forthling away from the page, so that the page never freezes; the
// page can always stop this worker and start a new one.

import { load } from "./forthling.js";

const forth = await load(new URL("forthling.wasm", import.meta.url));

onmessage = ({ data: { id, action, code, trace, name, text } }) => {
  let answer;
  try {
    if (action === "eval") answer = forth.eval(code, { trace });
    else if (action === "reset") answer = forth.reset();
    else if (action === "writeFile") answer = forth.writeFile(name, text) ?? null;
    else if (action === "readFile") answer = forth.readFile(name);
    else if (action === "files") answer = forth.files();
    else answer = forth.words();
  } catch (e) {
    answer = { crashed: String(e) }; // a trap in the module: the page starts anew
  }
  postMessage({ id, answer });
};

postMessage({ ready: true });
