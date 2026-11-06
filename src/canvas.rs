//! A canvas without a window: the word list IMAGE, installed as the extension `Canvas`.
//!
//! The canvas is a framebuffer of pixels, each 0xRRGGBB in a cell. `CANVAS ( w h -- )`
//! makes it; `PIXEL`, `LINE` and `CLEAR` draw; a palette of 256 colours turns an
//! iteration count into a colour. `SAVE-PPM` and `SAVE-PNG` write the picture through
//! the file system of the host (so a file in memory in MCP and the playground), which
//! needs `Permission::Files`. The PNG is not compressed (stored deflate blocks), so it
//! needs no dependency; every viewer reads it.

use crate::vfs::Access;
use crate::*;

/// The extension: `forth.install(&Canvas)`; ENVIRONMENT? knows it as FORTHLING-CANVAS.
pub struct Canvas;

/// The most pixels a canvas holds (4096 x 4096): 64 MB.
const MAX_PIXELS: usize = 1 << 24;
/// The largest coordinate a line accepts; a longer line is -24.
const MAX_COORD: Cell = 1 << 24;

struct CanvasState {
    width: usize,
    height: usize,
    pixels: Vec<u32>,
    palette: [u32; 256],
}

impl Default for CanvasState {
    fn default() -> Self {
        let mut palette = [0; 256];
        for (i, p) in palette.iter_mut().enumerate().skip(1) {
            *p = rainbow(i);
        }
        CanvasState { width: 0, height: 0, pixels: Vec::new(), palette }
    }
}

/// Entry 0 is black; the others go round the colour wheel.
#[allow(clippy::cast_possible_truncation, reason = "all values are below 256")]
fn rainbow(i: usize) -> u32 {
    let hue = (i - 1) * 6 * 256 / 255;
    let (sector, rest) = (hue / 256, (hue % 256) as u32);
    let (r, g, b) = match sector {
        0 => (255, rest, 0),
        1 => (255 - rest, 255, 0),
        2 => (0, 255, rest),
        3 => (0, 255 - rest, 255),
        4 => (rest, 0, 255),
        _ => (255, 0, 255 - rest),
    };
    (r << 16) | (g << 8) | b
}

fn state(f: &mut Forth) -> &mut CanvasState {
    f.state::<CanvasState>()
}

/// The canvas, or -24 when there is none yet.
fn canvas<'a>(f: &'a mut Forth, word: &str) -> Res<&'a mut CanvasState> {
    let st = state(f);
    if st.pixels.is_empty() {
        return Err(Throw::with(INVALID_ARGUMENT, format!("{word}: no canvas, say w h CANVAS")));
    }
    Ok(st)
}

#[allow(clippy::cast_sign_loss, reason = "the value is clamped to 0..=255")]
fn channel(v: Cell) -> u32 {
    v.clamp(0, 255) as u32
}

#[allow(clippy::cast_sign_loss, reason = "the mask leaves 24 bits")]
fn colour(v: Cell) -> u32 {
    (v & 0xFF_FFFF) as u32
}

/// The index of pixel (x, y), or None outside the canvas.
fn index(st: &CanvasState, x: Cell, y: Cell) -> Option<usize> {
    let (x, y) = (usize::try_from(x).ok()?, usize::try_from(y).ok()?);
    (x < st.width && y < st.height).then(|| y * st.width + x)
}

fn make_canvas(f: &mut Forth) -> Res {
    let h = f.pop()?;
    let w = f.pop()?;
    let size = usize::try_from(w)
        .ok()
        .zip(usize::try_from(h).ok())
        .filter(|&(w, h)| w > 0 && h > 0 && w.checked_mul(h).is_some_and(|n| n <= MAX_PIXELS));
    let Some((w, h)) = size else {
        return Err(Throw::with(
            INVALID_ARGUMENT,
            format!("CANVAS: {w} x {h}, at most {MAX_PIXELS} pixels"),
        ));
    };
    f.charge(w * h * 4)?;
    let st = state(f);
    st.pixels = vec![0; w * h];
    st.width = w;
    st.height = h;
    Ok(())
}

fn draw_line(f: &mut Forth) -> Res {
    let c = colour(f.pop()?);
    let (y2, x2) = (f.pop()?, f.pop()?);
    let (y1, x1) = (f.pop()?, f.pop()?);
    if [x1, y1, x2, y2].iter().any(|v| v.abs() > MAX_COORD) {
        return Err(Throw::with(INVALID_ARGUMENT, "LINE: a coordinate beyond 2^24"));
    }
    let (dx, dy) = ((x2 - x1).abs(), -(y2 - y1).abs());
    let steps = dx.max(-dy) + 1;
    f.charge(usize::try_from(steps).unwrap_or(0) * 4)?;
    let st = canvas(f, "LINE")?;
    let (sx, sy) = (if x1 < x2 { 1 } else { -1 }, if y1 < y2 { 1 } else { -1 });
    let (mut x, mut y, mut err) = (x1, y1, dx + dy);
    loop {
        if let Some(i) = index(st, x, y) {
            st.pixels[i] = c;
        }
        if x == x2 && y == y2 {
            return Ok(());
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// The picture as rows of RGB bytes.
fn rgb_bytes(st: &CanvasState) -> Vec<u8> {
    let mut out = Vec::with_capacity(st.pixels.len() * 3);
    for &p in &st.pixels {
        out.extend_from_slice(&p.to_be_bytes()[1..]);
    }
    out
}

fn ppm(st: &CanvasState) -> Vec<u8> {
    let mut out = format!("P6\n{} {}\n255\n", st.width, st.height).into_bytes();
    out.extend(rgb_bytes(st));
    out
}

fn crc32(parts: &[&[u8]]) -> u32 {
    let mut crc = u32::MAX;
    for &b in parts.iter().copied().flatten() {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 == 1 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

fn chunk(out: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
    out.extend_from_slice(&u32::try_from(data.len()).unwrap_or(u32::MAX).to_be_bytes());
    out.extend_from_slice(&kind);
    out.extend_from_slice(data);
    out.extend_from_slice(&crc32(&[&kind, data]).to_be_bytes());
}

/// A PNG with stored (uncompressed) deflate blocks.
fn png(st: &CanvasState) -> Vec<u8> {
    let rgb = rgb_bytes(st);
    let mut raw = Vec::with_capacity(rgb.len() + st.height);
    for row in rgb.chunks(st.width * 3) {
        raw.push(0); // filter type: none
        raw.extend_from_slice(row);
    }
    let mut z = vec![0x78, 0x01];
    let blocks = raw.chunks(65535).count();
    for (i, block) in raw.chunks(65535).enumerate() {
        z.push(u8::from(i + 1 == blocks));
        let len = u16::try_from(block.len()).unwrap_or(u16::MAX);
        z.extend_from_slice(&len.to_le_bytes());
        z.extend_from_slice(&(!len).to_le_bytes());
        z.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in &raw {
        a = (a + u32::from(byte)) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut header = Vec::new();
    header.extend_from_slice(&u32::try_from(st.width).unwrap_or(0).to_be_bytes());
    header.extend_from_slice(&u32::try_from(st.height).unwrap_or(0).to_be_bytes());
    header.extend_from_slice(&[8, 2, 0, 0, 0]); // 8 bits, truecolour, no interlace
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut out, *b"IHDR", &header);
    chunk(&mut out, *b"IDAT", &z);
    chunk(&mut out, *b"IEND", &[]);
    out
}

/// `SAVE-PPM ( c-addr u -- ior )` and `SAVE-PNG`: ior is 0, or -259 without
/// `Permission::Files`, or the code of the file word that failed.
fn save(f: &mut Forth, word: &str, encode: fn(&CanvasState) -> Vec<u8>) -> Res {
    let name = f.pop_name()?;
    let bytes = encode(canvas(f, word)?);
    f.charge(bytes.len())?;
    let ior = if f.permits(Permission::Files) {
        let access = Access { write: true, create: true, truncate: true, ..Access::default() };
        match f.vfs.open(&name, access) {
            Err(_) => CREATE_FILE_FAILED,
            Ok(mut file) => {
                if file.write_all(&bytes).and_then(|()| file.flush()).is_ok() {
                    0
                } else {
                    FILE_IO
                }
            }
        }
    } else {
        NOT_PERMITTED
    };
    f.push(ior);
    Ok(())
}

impl Extension for Canvas {
    fn name(&self) -> &'static str {
        "FORTHLING-CANVAS"
    }

    #[allow(clippy::many_single_char_names, reason = "x, y, r, g, b are the usual names")]
    fn install(&self, f: &mut Forth) {
        let wid = f.wordlist("IMAGE");
        let mut add_word = |name: &str, word: fn(&mut Forth) -> Res| {
            f.add_word(wid, name, word).expect("IMAGE is a word list");
        };
        add_word("CANVAS", make_canvas);
        add_word("CANVAS-SIZE", |f| {
            let st = state(f);
            let (w, h) = (to_cell(st.width), to_cell(st.height));
            f.push(w);
            f.push(h);
            Ok(())
        });
        add_word("RGB", |f| {
            let b = channel(f.pop()?);
            let g = channel(f.pop()?);
            let r = channel(f.pop()?);
            f.push(Cell::from((r << 16) | (g << 8) | b));
            Ok(())
        });
        add_word("CLEAR", |f| {
            let c = colour(f.pop()?);
            canvas(f, "CLEAR")?.pixels.fill(c);
            Ok(())
        });
        add_word("PIXEL", |f| {
            let c = colour(f.pop()?);
            let y = f.pop()?;
            let x = f.pop()?;
            let st = canvas(f, "PIXEL")?;
            if let Some(i) = index(st, x, y) {
                st.pixels[i] = c;
            }
            Ok(())
        });
        add_word("PIXEL@", |f| {
            let y = f.pop()?;
            let x = f.pop()?;
            let st = canvas(f, "PIXEL@")?;
            let i = index(st, x, y).ok_or_else(|| {
                Throw::with(INVALID_ARGUMENT, format!("PIXEL@: ({x}, {y}) is outside the canvas"))
            })?;
            let c = st.pixels[i];
            f.push(Cell::from(c));
            Ok(())
        });
        add_word("LINE", draw_line);
        add_word("PALETTE!", |f| {
            let i = f.pop()?;
            let c = colour(f.pop()?);
            let slot = usize::try_from(i).ok().filter(|&i| i < 256).ok_or_else(|| {
                Throw::with(INVALID_ARGUMENT, format!("PALETTE!: index {i}, 0 to 255"))
            })?;
            state(f).palette[slot] = c;
            Ok(())
        });
        add_word("PALETTE@", |f| {
            // An index is taken modulo 256, so an iteration count can go straight in.
            let i = f.pop()?;
            let c = state(f).palette[usize::try_from(i.rem_euclid(256)).unwrap_or(0)];
            f.push(Cell::from(c));
            Ok(())
        });
        add_word("SAVE-PPM", |f| save(f, "SAVE-PPM", ppm));
        add_word("SAVE-PNG", |f| save(f, "SAVE-PNG", png));
    }
}
