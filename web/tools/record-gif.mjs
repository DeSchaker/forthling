// Records the playground's tutorial as an animated GIF for the README:
//
//   cargo build --release --manifest-path web/Cargo.toml --target wasm32-unknown-unknown
//   node web/tools/record-gif.mjs [out.gif]          (default docs/tutorial.gif)
//
// It serves web/www with the built forthling.wasm, starts Chrome headless
// (set CHROME to its path if it is not found), plays a short scene in the
// light theme and writes the GIF. It needs Node 22 and nothing else: the
// screenshots are decoded with node:zlib and the GIF is encoded here.

import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdtemp, readFile, rm, writeFile, mkdir } from "node:fs/promises";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { dirname, extname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { inflateSync } from "node:zlib";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const out = resolve(process.argv[2] ?? join(root, "docs/tutorial.gif"));
const wasm = join(root, "web/target/wasm32-unknown-unknown/release/forthling_web.wasm");
const WIDTH = 720, HEIGHT = 900;

// ---- serving the playground ----------------------------------------------

const types = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm" };
const server = createServer(async (req, res) => {
  const path = new URL(req.url, "http://x").pathname;
  const file = path === "/forthling.wasm" ? wasm : join(root, "web/www", path === "/" ? "index.html" : path);
  try {
    const body = await readFile(file);
    res.writeHead(200, { "content-type": types[extname(file)] ?? "application/octet-stream" });
    res.end(body);
  } catch {
    res.writeHead(404).end();
  }
});
await new Promise((r) => server.listen(0, "127.0.0.1", r));
const url = `http://127.0.0.1:${server.address().port}/`;

// ---- Chrome over the DevTools protocol -----------------------------------

const chromePath = [
  process.env.CHROME,
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Google/Chrome/Application/chrome.exe",
  "/usr/bin/google-chrome",
  "/usr/bin/chromium",
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
].find((p) => p && existsSync(p));
if (!chromePath) throw new Error("Chrome not found; set CHROME to its path");
const profile = await mkdtemp(join(tmpdir(), "forthling-gif-"));
const chrome = spawn(chromePath, [
  "--headless=new", "--remote-debugging-port=0", `--user-data-dir=${profile}`,
  "--no-first-run", "--hide-scrollbars", `--window-size=${WIDTH},${HEIGHT}`, "about:blank",
], { stdio: "ignore" });

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let port;
for (let i = 0; i < 100 && !port; i++) {
  await sleep(100);
  try {
    port = (await readFile(join(profile, "DevToolsActivePort"), "utf8")).split("\n")[0];
  } catch {
    // not yet
  }
}
const targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
const ws = new WebSocket(targets.find((t) => t.type === "page").webSocketDebuggerUrl);
await new Promise((r) => (ws.onopen = r));
let nextId = 0;
const pending = new Map();
ws.onmessage = ({ data }) => {
  const m = JSON.parse(data);
  if (pending.has(m.id)) {
    pending.get(m.id)(m);
    pending.delete(m.id);
  }
};
const send = (method, params = {}) =>
  new Promise((r) => {
    pending.set(++nextId, r);
    ws.send(JSON.stringify({ id: nextId, method, params }));
  });
const js = async (expression) => {
  const r = await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
  if (r.result.exceptionDetails) throw new Error(JSON.stringify(r.result.exceptionDetails));
  return r.result.result.value;
};
const idle = async () => {
  for (let i = 0; i < 100; i++) {
    await sleep(50);
    if (!(await js(`document.getElementById("run").disabled`))) {
      await sleep(100);
      if (!(await js(`document.getElementById("run").disabled`))) return;
    }
  }
  throw new Error("the playground stays busy");
};

// ---- the scene -----------------------------------------------------------

const frames = []; // { png, delay } with delay in ms
async function frame(delay) {
  const shot = await send("Page.captureScreenshot", { format: "png" });
  frames.push({ png: Buffer.from(shot.result.data, "base64"), delay });
}

// A button is shown pressed for a moment, as there is no mouse pointer.
async function press(id, after = 1200) {
  await js(`document.getElementById("${id}").classList.add("gif-press")`);
  await frame(350);
  await js(`(() => { const b = document.getElementById("${id}"); b.classList.remove("gif-press"); b.click(); })()`);
  await idle();
  await frame(after);
}

async function type(text) {
  for (let i = 1; i <= text.length; i++) {
    await js(`document.getElementById("code").value = ${JSON.stringify(text.slice(0, i))}`);
    if (text[i - 1] !== " ") await frame(text[i - 1] === "\n" ? 300 : 70);
  }
}

await send("Page.enable");
await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-color-scheme", value: "light" }] });
await send("Emulation.setDeviceMetricsOverride", { width: WIDTH, height: HEIGHT, deviceScaleFactor: 1, mobile: false });
await send("Page.navigate", { url });
await sleep(800);
await idle();
// A fresh start, and a page that fits the frame.
await js(`localStorage.clear()`);
await send("Page.reload");
await sleep(800);
await idle();
await js(`(() => {
  const style = document.createElement("style");
  style.textContent = "#transcript { height: 100px } #lesson-text { max-height: 280px }" +
    " #code { min-height: 0; height: 64px } p.lead { display: none }" +
    " .gif-press { outline: 3px solid var(--accent); outline-offset: 2px }";
  document.head.append(style);
  document.getElementById("code").value = "";
})()`);

await frame(900);
await press("tutorial-button", 1500);
await js(`(() => { const p = document.getElementById("lesson-pick"); p.value = "4"; p.dispatchEvent(new Event("change")); })()`);
await idle();
await frame(2500);
// Down to the task, as a reader would scroll.
for (const at of [80, 160, 240, 400]) {
  await js(`document.getElementById("lesson-text").scrollTop = ${at}`);
  await frame(at === 400 ? 2800 : 120);
}
await type(": CUBE DUP * ;");
await press("run", 700);
await press("lesson-check", 2600);
await press("lesson-hint", 2600);
await js(`document.getElementById("code").value = ""`);
await type(": CUBE DUP DUP * * ;\n2 CUBE .");
await press("run", 1200);
await press("lesson-check", 3500);

// Browser.close ends Chrome with all its processes; kill() is the fallback.
await Promise.race([send("Browser.close"), sleep(3000)]);
ws.close();
await Promise.race([new Promise((r) => chrome.once("exit", r)), sleep(3000)]);
chrome.kill();
server.close();
await sleep(500);
await rm(profile, { recursive: true, force: true }).catch(() => {});

// ---- PNG to pixels -------------------------------------------------------

function decodePng(png) {
  let width, height, colorType, idat = [];
  for (let at = 8; at < png.length; ) {
    const length = png.readUInt32BE(at);
    const kind = png.toString("latin1", at + 4, at + 8);
    const data = png.subarray(at + 8, at + 8 + length);
    if (kind === "IHDR") {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      colorType = data[9];
      if (data[8] !== 8 || data[12] !== 0) throw new Error("unexpected PNG format");
    } else if (kind === "IDAT") idat.push(data);
    at += 12 + length;
  }
  const channels = { 2: 3, 6: 4 }[colorType];
  if (!channels) throw new Error(`unexpected PNG color type ${colorType}`);
  const raw = inflateSync(Buffer.concat(idat));
  const stride = width * channels;
  const rgb = new Uint8Array(width * height * 3);
  let prev = new Uint8Array(stride);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)];
    const line = raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1));
    const cur = new Uint8Array(stride);
    for (let i = 0; i < stride; i++) {
      const a = i >= channels ? cur[i - channels] : 0, b = prev[i], c = i >= channels ? prev[i - channels] : 0;
      let p = line[i];
      if (filter === 1) p += a;
      else if (filter === 2) p += b;
      else if (filter === 3) p += (a + b) >> 1;
      else if (filter === 4) {
        const pa = Math.abs(b - c), pb = Math.abs(a - c), pc = Math.abs(a + b - 2 * c);
        p += pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
      }
      cur[i] = p & 255;
    }
    for (let x = 0; x < width; x++) {
      rgb.set(cur.subarray(x * channels, x * channels + 3), (y * width + x) * 3);
    }
    prev = cur;
  }
  return { width, height, rgb };
}

// ---- a palette for all frames --------------------------------------------

const images = frames.map((f) => decodePng(f.png));
const { width, height } = images[0];
const counts = new Map();
for (const { rgb } of images) {
  for (let i = 0; i < rgb.length; i += 3) {
    const c = (rgb[i] << 16) | (rgb[i + 1] << 8) | rgb[i + 2];
    counts.set(c, (counts.get(c) ?? 0) + 1);
  }
}
const parts = (c) => [c >> 16, (c >> 8) & 255, c & 255];
const distance = (p, q) => (p[0] - q[0]) ** 2 + (p[1] - q[1]) ** 2 + (p[2] - q[2]) ** 2;
// The most frequent colors first, skipping those close to one already taken.
const palette = [];
for (const min of [48, 12, 0]) {
  for (const [c] of [...counts].sort((a, b) => b[1] - a[1])) {
    if (palette.length === 256) break;
    const p = parts(c);
    if (palette.every((q) => distance(p, q) > min)) palette.push(p);
  }
}
while (palette.length < 256) palette.push([0, 0, 0]);
const nearest = new Map();
function index(c) {
  let best = nearest.get(c);
  if (best === undefined) {
    const p = parts(c);
    let d = Infinity;
    palette.forEach((q, i) => {
      const e = distance(p, q);
      if (e < d) [d, best] = [e, i];
    });
    nearest.set(c, best);
  }
  return best;
}
const indexed = images.map(({ rgb }) => {
  const px = new Uint8Array(width * height);
  for (let i = 0; i < px.length; i++) px[i] = index((rgb[i * 3] << 16) | (rgb[i * 3 + 1] << 8) | rgb[i * 3 + 2]);
  return px;
});

// ---- GIF -----------------------------------------------------------------

// LZW with 8-bit pixels, in the order of the well-known GIF encoders: a code
// is written, then the code size grows if the next free code no longer
// fits, then the new string gets that code; a full table is cleared.
function lzw(pixels) {
  const out = [];
  let acc = 0, bits = 0, size = 9, next = 258;
  const emit = (code) => {
    acc |= code << bits;
    bits += size;
    while (bits >= 8) {
      out.push(acc & 255);
      acc >>>= 8;
      bits -= 8;
    }
    if (code === 256) size = 9;
    else if (next > (1 << size) - 1 && size < 12) size++;
  };
  let dict = new Map();
  emit(256);
  let prefix = pixels[0];
  for (let i = 1; i < pixels.length; i++) {
    const key = (prefix << 8) | pixels[i];
    const code = dict.get(key);
    if (code !== undefined) {
      prefix = code;
      continue;
    }
    emit(prefix);
    if (next < 4096) {
      dict.set(key, next++);
    } else {
      dict = new Map();
      next = 258;
      emit(256);
    }
    prefix = pixels[i];
  }
  emit(prefix);
  emit(257);
  if (bits > 0) out.push(acc & 255);
  const blocks = [8];
  for (let i = 0; i < out.length; i += 255) {
    const n = Math.min(255, out.length - i);
    blocks.push(n);
    for (let j = 0; j < n; j++) blocks.push(out[i + j]);
  }
  blocks.push(0);
  return blocks;
}

const gif = [];
const word = (n) => gif.push(n & 255, n >> 8);
const append = (bytes) => {
  for (const b of bytes) gif.push(b);
};
gif.push(...Buffer.from("GIF89a"));
word(width);
word(height);
gif.push(0xf7, 0, 0);
for (const p of palette) gif.push(...p);
gif.push(0x21, 0xff, 11, ...Buffer.from("NETSCAPE2.0"), 3, 1, 0, 0, 0);
let shown = null;
let carried = 0;
indexed.forEach((px, n) => {
  // Only the rectangle that changed since the frame shown.
  let [x0, y0, x1, y1] = [width, height, -1, -1];
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const i = y * width + x;
      if (!shown || shown[i] !== px[i]) {
        if (x < x0) x0 = x;
        if (x > x1) x1 = x;
        if (y < y0) y0 = y;
        if (y > y1) y1 = y;
      }
    }
  }
  const delay = frames[n].delay + carried;
  if (x1 < 0) {
    // Nothing changed: the frame before stays longer.
    carried = delay;
    if (n < indexed.length - 1) return;
    [x0, y0, x1, y1] = [0, 0, 0, 0];
  }
  carried = 0;
  const w = x1 - x0 + 1, h = y1 - y0 + 1;
  const part = new Uint8Array(w * h);
  for (let y = 0; y < h; y++) part.set(px.subarray((y0 + y) * width + x0, (y0 + y) * width + x0 + w), y * w);
  gif.push(0x21, 0xf9, 4, 0x04);
  word(Math.round(delay / 10));
  gif.push(0, 0, 0x2c);
  word(x0);
  word(y0);
  word(w);
  word(h);
  gif.push(0);
  append(lzw(part));
  shown = px;
});
gif.push(0x3b);
await mkdir(dirname(out), { recursive: true });
await writeFile(out, Buffer.from(gif));
const seconds = frames.reduce((s, f) => s + f.delay, 0) / 1000;
console.log(`${out}: ${frames.length} frames, ${seconds.toFixed(1)} s, ${gif.length} bytes`);
