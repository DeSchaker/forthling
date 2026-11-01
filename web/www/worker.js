// Runs forthling away from the page, so that the page never freezes; the
// page can always stop this worker and start a new one.

import { load } from "./forthling.js";

const forth = await load(new URL("forthling.wasm", import.meta.url));

// The files of the session as { name: text }, for the page to keep.
function snapshot() {
  return Object.fromEntries(forth.files().map(({ name }) => [name, forth.readFile(name)]));
}

// Puts files in; answers the names that did not fit.
function putFiles(files) {
  const failed = [];
  for (const [name, text] of Object.entries(files)) {
    try {
      forth.writeFile(name, text);
    } catch {
      failed.push(name);
    }
  }
  return failed;
}

onmessage = ({ data: { id, action, code, trace, name, text, files } }) => {
  let answer;
  try {
    if (action === "eval") answer = forth.eval(code, { trace });
    else if (action === "reset") answer = forth.reset();
    else if (action === "writeFile") answer = putFiles({ [name]: text }).length ? `${name} does not fit` : null;
    else if (action === "readFile") answer = forth.readFile(name);
    else if (action === "deleteFile") answer = forth.deleteFile(name);
    else if (action === "files") answer = forth.files();
    else if (action === "snapshot") answer = snapshot();
    else if (action === "putFiles") answer = putFiles(files);
    else answer = forth.words();
  } catch (e) {
    answer = { crashed: String(e) }; // a trap in the module: the page starts anew
  }
  postMessage({ id, answer });
};

postMessage({ ready: true });
