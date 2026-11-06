//! The IMAGE words (canvas, drawing and export), used from Forth.

use forthling::{Canvas, Cell, Forth, MemoryFs, Permission};
use std::io::Cursor;

fn forth() -> (Forth, MemoryFs) {
    let mut f = Forth::new();
    f.set_input(Box::new(Cursor::new(Vec::new())));
    let fs = MemoryFs::new();
    f.set_file_system(Box::new(fs.clone())).unwrap();
    f.install(&Canvas);
    f.interpret("ALSO IMAGE").unwrap();
    (f, fs)
}

fn run(f: &mut Forth, text: &str) -> String {
    let start = f.output().len();
    f.interpret(text).unwrap_or_else(|e| panic!("{text}: {e}"));
    String::from_utf8_lossy(&f.output()[start..]).into_owned()
}

fn code(f: &mut Forth, text: &str) -> Cell {
    f.interpret(text).expect_err(text).code
}

#[test]
fn pixels_and_colours() {
    let (mut f, _) = forth();
    assert_eq!(run(&mut f, "4 3 CANVAS CANVAS-SIZE . ."), "3 4 ");
    assert_eq!(run(&mut f, "1 2 255 128 0 RGB PIXEL 1 2 PIXEL@ ."), format!("{} ", 0xFF_8000));
    assert_eq!(run(&mut f, "0 0 PIXEL@ ."), "0 ");
    // Out of range pixels are ignored on writing and refused on reading.
    assert_eq!(run(&mut f, "-1 0 5 PIXEL 4 0 5 PIXEL 0 3 5 PIXEL DEPTH ."), "0 ");
    assert_eq!(code(&mut f, "4 0 PIXEL@"), -24);
    // RGB cuts a channel to 0..255.
    assert_eq!(run(&mut f, "300 -5 7 RGB ."), format!("{} ", 0xFF_0007));
    assert_eq!(run(&mut f, "9 CLEAR 3 2 PIXEL@ ."), "9 ");
}

#[test]
fn lines_include_both_ends_and_are_clipped() {
    let (mut f, _) = forth();
    run(&mut f, "5 5 CANVAS 0 0 4 4 1 LINE");
    assert_eq!(run(&mut f, "0 0 PIXEL@ . 2 2 PIXEL@ . 4 4 PIXEL@ . 1 0 PIXEL@ ."), "1 1 1 0 ");
    // A line that leaves the canvas draws the part inside.
    run(&mut f, "0 CLEAR -10 2 10 2 3 LINE");
    assert_eq!(run(&mut f, "0 2 PIXEL@ . 4 2 PIXEL@ . 4 1 PIXEL@ ."), "3 3 0 ");
    // A line of a single point, and a steep one.
    run(&mut f, "0 CLEAR 2 2 2 2 4 LINE 0 4 1 0 6 LINE");
    assert_eq!(run(&mut f, "2 2 PIXEL@ . 0 4 PIXEL@ . 1 0 PIXEL@ ."), "4 6 6 ");
    assert_eq!(code(&mut f, "0 0 20000000 0 1 LINE"), -24);
}

#[test]
fn the_palette_is_modulo_256() {
    let (mut f, _) = forth();
    assert_eq!(run(&mut f, "0 PALETTE@ . 256 PALETTE@ . -256 PALETTE@ ."), "0 0 0 ");
    assert_eq!(run(&mut f, "1 PALETTE@ 257 PALETTE@ = ."), "-1 ");
    assert_eq!(run(&mut f, "7 3 PALETTE! 3 PALETTE@ . 259 PALETTE@ ."), "7 7 ");
    assert_eq!(code(&mut f, "1 256 PALETTE!"), -24);
}

#[test]
fn without_a_canvas_or_with_a_bad_size_it_throws() {
    let (mut f, _) = forth();
    assert_eq!(run(&mut f, "CANVAS-SIZE . ."), "0 0 ");
    assert_eq!(code(&mut f, "0 0 1 PIXEL"), -24);
    assert_eq!(code(&mut f, "1 CLEAR"), -24);
    assert_eq!(code(&mut f, "S\" a.ppm\" SAVE-PPM"), -24);
    assert_eq!(code(&mut f, "0 5 CANVAS"), -24);
    assert_eq!(code(&mut f, "-3 5 CANVAS"), -24);
    assert_eq!(code(&mut f, "4097 4096 CANVAS"), -24);
    assert_eq!(run(&mut f, "4096 4096 CANVAS CANVAS-SIZE . ."), "4096 4096 ");
}

#[test]
fn ppm_file() {
    let (mut f, fs) = forth();
    run(&mut f, "2 2 CANVAS 0 0 255 0 0 RGB PIXEL 1 1 0 0 255 RGB PIXEL");
    assert_eq!(run(&mut f, "S\" a.ppm\" SAVE-PPM ."), "0 ");
    let mut want = b"P6\n2 2\n255\n".to_vec();
    want.extend_from_slice(&[255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255]);
    assert_eq!(fs.get("a.ppm").unwrap(), want);
    // Saving again replaces the file.
    run(&mut f, "1 1 CANVAS S\" a.ppm\" SAVE-PPM DROP");
    assert_eq!(fs.get("a.ppm").unwrap().len(), b"P6\n1 1\n255\n".len() + 3);
}

/// Reads the chunks of a PNG, checking each CRC; returns (type, data) of each.
fn chunks(png: &[u8]) -> Vec<(String, Vec<u8>)> {
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    let mut at = 8;
    let mut out = Vec::new();
    while at < png.len() {
        let len = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
        let kind = &png[at + 4..at + 8];
        let data = &png[at + 8..at + 8 + len];
        let crc = u32::from_be_bytes(png[at + 8 + len..at + 12 + len].try_into().unwrap());
        assert_eq!(crc, crc32(&[kind, data].concat()), "crc of {kind:?}");
        out.push((String::from_utf8(kind.to_vec()).unwrap(), data.to_vec()));
        at += 12 + len;
    }
    out
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &b in bytes {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 == 1 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// Undoes the stored deflate blocks of a zlib stream and checks the Adler-32.
fn inflate_stored(z: &[u8]) -> Vec<u8> {
    assert_eq!(&z[..2], &[0x78, 0x01]);
    let (mut at, mut out) = (2, Vec::new());
    loop {
        let last = z[at] & 1 == 1;
        assert_eq!(z[at] >> 1, 0, "stored block");
        let len = u16::from_le_bytes([z[at + 1], z[at + 2]]);
        let nlen = u16::from_le_bytes([z[at + 3], z[at + 4]]);
        assert_eq!(nlen, !len);
        out.extend_from_slice(&z[at + 5..at + 5 + len as usize]);
        at += 5 + len as usize;
        if last {
            break;
        }
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in &out {
        a = (a + u32::from(byte)) % 65521;
        b = (b + a) % 65521;
    }
    assert_eq!(&z[at..], &((b << 16) | a).to_be_bytes());
    out
}

#[test]
fn png_file() {
    let (mut f, fs) = forth();
    run(&mut f, "3 2 CANVAS 0 0 10 20 30 RGB PIXEL 2 1 40 50 60 RGB PIXEL");
    assert_eq!(run(&mut f, "S\" a.png\" SAVE-PNG ."), "0 ");
    let parts = chunks(&fs.get("a.png").unwrap());
    let names: Vec<_> = parts.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["IHDR", "IDAT", "IEND"]);
    assert_eq!(parts[0].1, [0, 0, 0, 3, 0, 0, 0, 2, 8, 2, 0, 0, 0]);
    let raw = inflate_stored(&parts[1].1);
    let mut want = vec![0, 10, 20, 30, 0, 0, 0, 0, 0, 0];
    want.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 40, 50, 60]);
    assert_eq!(raw, want);
}

#[test]
fn a_big_png_uses_several_blocks() {
    let (mut f, fs) = forth();
    run(&mut f, "200 200 CANVAS 5 CLEAR");
    assert_eq!(run(&mut f, "S\" big.png\" SAVE-PNG ."), "0 ");
    let parts = chunks(&fs.get("big.png").unwrap());
    let raw = inflate_stored(&parts[1].1);
    assert_eq!(raw.len(), 200 * (1 + 600));
    assert_eq!(&raw[1..4], &[0, 0, 5]);
}

#[test]
fn saving_needs_the_permission_for_files() {
    let (mut f, fs) = forth();
    run(&mut f, "2 2 CANVAS");
    f.permit(Permission::Files, false);
    assert_eq!(run(&mut f, "S\" a.ppm\" SAVE-PPM ."), "-259 ");
    assert_eq!(run(&mut f, "S\" a.png\" SAVE-PNG ."), "-259 ");
    assert!(fs.get("a.ppm").is_none());
}

#[test]
fn environment_knows_the_extension() {
    let (mut f, _) = forth();
    assert_eq!(run(&mut f, "S\" FORTHLING-CANVAS\" ENVIRONMENT? . DROP"), "-1 ");
}

#[test]
fn a_host_takes_the_picture_once_after_drawing() {
    let (mut f, _) = forth();
    assert_eq!(f.take_canvas(), None, "no canvas yet");
    run(&mut f, "2 1 CANVAS 0 0 255 128 0 RGB PIXEL");
    let (w, h, rgba) = f.take_canvas().expect("drawn on");
    assert_eq!((w, h), (2, 1));
    assert_eq!(rgba, [255, 128, 0, 255, 0, 0, 0, 255]);
    assert_eq!(f.take_canvas(), None, "nothing drawn since");
    run(&mut f, "1 PALETTE@ DROP 5 5 0 0 0 RGB PIXEL"); // outside, and not a change
    assert_eq!(f.take_canvas(), None);
    run(&mut f, "0 0 1 0 0 0 255 RGB LINE");
    assert_eq!(f.take_canvas().expect("a line").2[..4], [0, 0, 255, 255]);
    run(&mut f, "0 0 255 RGB CLEAR");
    assert!(f.take_canvas().is_some());
}

#[test]
fn show_prints_the_canvas_in_the_terminal() {
    let (mut f, _) = forth();
    assert_eq!(code(&mut f, "SHOW"), -24, "no canvas yet");
    // Colour or shades, depending on NO_COLOR; either way two rows of pixels are one line.
    let text = run(&mut f, "3 4 CANVAS 1 CLEAR SHOW");
    assert_eq!(text.lines().count(), 2);
    assert!(text.ends_with('\n'));
}
