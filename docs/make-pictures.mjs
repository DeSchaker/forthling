// Makes the pictures of the README in docs/img/ from real runs of the programs in examples/.
//
//   cargo build --release
//   node docs/make-pictures.mjs [path-to-forthling]
//
// - mandelbrot.png, julia.png: the files that book002_example001_mandelbrot.fs writes
//   (mandelbrot.ppm and julia.png), packed with compression, since SAVE-PNG stores its data
//   uncompressed.
// - maze.svg, sudoku.svg: the output of the maze and the sudoku program as a terminal window.
//   The text comes from the .out files, which the tests compare with a real run.
//
// Needs Node 18 or newer and no packages.

import { spawnSync } from "node:child_process";
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import zlib from "node:zlib";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const out = join(root, "docs", "img");
const exe = process.argv[2] ?? join(root, "target", "release", process.platform === "win32" ? "forthling.exe" : "forthling");
mkdirSync(out, { recursive: true });

// ---- PNG ----------------------------------------------------------------------------------

const crcTable = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});
function crc32(buf) {
  let c = 0xffffffff;
  for (const b of buf) c = crcTable[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}
function chunk(type, data) {
  const head = Buffer.alloc(8);
  head.writeUInt32BE(data.length, 0);
  head.write(type, 4, "latin1");
  const tail = Buffer.alloc(4);
  tail.writeUInt32BE(crc32(Buffer.concat([head.subarray(4), data])), 0);
  return Buffer.concat([head, data, tail]);
}
// rgb: width*height*3 bytes, no filtering
function encodePng(width, height, rgb) {
  const rows = Buffer.alloc((width * 3 + 1) * height);
  for (let y = 0; y < height; y++) {
    rows[y * (width * 3 + 1)] = 0;
    rgb.copy(rows, y * (width * 3 + 1) + 1, y * width * 3, (y + 1) * width * 3);
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr.set([8, 2, 0, 0, 0], 8); // 8 bits, RGB, deflate, no filter, no interlace
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", zlib.deflateSync(rows, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}
// reads an 8-bit RGB PNG without filtering, as SAVE-PNG writes it
function decodePng(png) {
  let pos = 8, width = 0, height = 0;
  const idat = [];
  while (pos < png.length) {
    const len = png.readUInt32BE(pos);
    const type = png.toString("latin1", pos + 4, pos + 8);
    const data = png.subarray(pos + 8, pos + 8 + len);
    if (type === "IHDR") {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      if (data[8] !== 8 || data[9] !== 2) throw new Error("not an 8-bit RGB PNG");
    } else if (type === "IDAT") idat.push(data);
    pos += 12 + len;
  }
  const raw = zlib.inflateSync(Buffer.concat(idat));
  const rgb = Buffer.alloc(width * height * 3);
  for (let y = 0; y < height; y++) {
    if (raw[y * (width * 3 + 1)] !== 0) throw new Error("filtered PNG rows are not supported");
    raw.copy(rgb, y * width * 3, y * (width * 3 + 1) + 1, (y + 1) * (width * 3 + 1));
  }
  return { width, height, rgb };
}
function decodePpm(ppm) {
  // P6 <width> <height> <max> <whitespace> data
  const m = /^P6\s+(\d+)\s+(\d+)\s+255\s/.exec(ppm.toString("latin1", 0, 40));
  if (!m) throw new Error("not a P6 PPM");
  return { width: +m[1], height: +m[2], rgb: ppm.subarray(m[0].length) };
}

// ---- Mandelbrot and Julia -----------------------------------------------------------------

const work = mkdtempSync(join(tmpdir(), "forthling-pictures-"));
try {
  const run = spawnSync(exe, [join(root, "examples", "book002_example001_mandelbrot.fs")], { cwd: work, encoding: "utf8" });
  if (run.status !== 0) throw new Error("the Mandelbrot program failed: " + run.stdout + run.stderr);
  const m = decodePpm(readFileSync(join(work, "mandelbrot.ppm")));
  writeFileSync(join(out, "mandelbrot.png"), encodePng(m.width, m.height, m.rgb));
  const j = decodePng(readFileSync(join(work, "julia.png")));
  writeFileSync(join(out, "julia.png"), encodePng(j.width, j.height, j.rgb));
} finally {
  rmSync(work, { recursive: true, force: true });
}

// ---- Text as a terminal window ------------------------------------------------------------

function terminal(title, lines) {
  const cw = 9.6, lh = 20, padX = 20, padTop = 44, padBottom = 18;
  const cols = Math.max(...lines.map((l) => [...l].length), title.length + 8);
  const width = Math.ceil(cols * cw + padX * 2);
  const height = padTop + lines.length * lh + padBottom;
  const esc = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  const text = lines
    .map((l, i) => `<text x="${padX}" y="${padTop + (i + 1) * lh - 5}" xml:space="preserve">${esc(l)}</text>`)
    .join("\n    ");
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" role="img" aria-label="${esc(title)}">
  <rect width="${width}" height="${height}" rx="8" fill="#1e1e2e"/>
  <rect width="${width}" height="30" rx="8" fill="#313244"/>
  <rect y="22" width="${width}" height="8" fill="#313244"/>
  <circle cx="18" cy="15" r="5" fill="#f38ba8"/><circle cx="36" cy="15" r="5" fill="#f9e2af"/><circle cx="54" cy="15" r="5" fill="#a6e3a1"/>
  <text x="${width / 2}" y="20" text-anchor="middle" font-family="Consolas, Menlo, 'DejaVu Sans Mono', monospace" font-size="13" fill="#a6adc8">${esc(title)}</text>
  <g font-family="Consolas, Menlo, 'DejaVu Sans Mono', monospace" font-size="16" fill="#cdd6f4">
    ${text}
  </g>
</svg>
`;
}
const outLines = (name) => readFileSync(join(root, "examples", name), "utf8").replace(/\r/g, "").trimEnd().split("\n");

// the maze with its shortest path: from the line "Solved." on
const maze = outLines("book001_example001_maze.out");
const solved = maze.findIndex((l) => l.startsWith("Solved"));
writeFileSync(join(out, "maze.svg"), terminal("forthling book001_example001_maze.fs", maze.slice(solved)));
// the sudoku, puzzle and answer
writeFileSync(join(out, "sudoku.svg"), terminal("forthling book001_example002_sudoku.fs", outLines("book001_example002_sudoku.out")));

console.log("pictures written to docs/img/");
