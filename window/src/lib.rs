//! A native window for the canvas of forthling: the word list WINDOW. `SHOW-WINDOW` opens a
//! window with the picture of the canvas (the word list IMAGE, extension `Canvas`) and waits
//! until the window is closed or Esc is pressed. A window reaches the machine, so it needs
//! `Permission::System`, which `Forth::sandboxed` denies.
//!
//! ```no_run
//! use forthling::{Canvas, Forth};
//!
//! let mut f = Forth::new();
//! f.install(&Canvas);
//! f.install(&forthling_window::Window);
//! f.interpret("ALSO IMAGE  64 64 CANVAS 0 0 63 63 16776960 LINE  ALSO WINDOW SHOW-WINDOW")?;
//! # Ok::<(), forthling::Throw>(())
//! ```

use forthling::{Cell, Extension, Forth, NOT_PERMITTED, Permission, Res, Throw};
use minifb::{Key, Scale, ScaleMode, WindowOptions};

/// The throw of a window that cannot be opened or drawn.
pub const WINDOW_ERROR: Cell = -263;

/// A window is opened at most this large when the canvas is small: the canvas is enlarged by
/// a whole factor as long as it fits.
const FIT: (usize, usize) = (1280, 800);
/// The largest factor a small canvas is enlarged by.
const MAX_FACTOR: usize = 16;

/// The extension: `forth.install(&Window)`. It needs the extension `Canvas` for a picture.
pub struct Window;

/// The factor by which a canvas of `width` x `height` is enlarged in the window.
#[must_use]
pub fn factor(width: usize, height: usize) -> usize {
    (FIT.0 / width.max(1)).min(FIT.1 / height.max(1)).clamp(1, MAX_FACTOR)
}

/// RGBA bytes as the 0RGB pixels the window takes.
fn pixels(rgba: &[u8]) -> Vec<u32> {
    rgba.chunks_exact(4)
        .map(|p| (u32::from(p[0]) << 16) | (u32::from(p[1]) << 8) | u32::from(p[2]))
        .collect()
}

fn failure(what: impl std::fmt::Display) -> Throw {
    Throw::with(WINDOW_ERROR, format!("window: {what}"))
}

/// Opens the window and keeps it until it is closed or Esc is pressed.
fn show(f: &mut Forth) -> Res {
    if !f.permits(Permission::System) {
        return Err(Throw::with(NOT_PERMITTED, "a window is not permitted".to_string()));
    }
    let Some((width, height, rgba)) = f.canvas_rgba() else {
        return Err(Throw::with(-24, "SHOW-WINDOW: there is no canvas, make one with CANVAS"));
    };
    let buffer = pixels(&rgba);
    let n = factor(width, height);
    let options = WindowOptions {
        resize: true,
        scale: Scale::X1,
        scale_mode: ScaleMode::AspectRatioStretch,
        ..WindowOptions::default()
    };
    let mut window = minifb::Window::new("forthling (Esc closes)", width * n, height * n, options)
        .map_err(failure)?;
    window.set_target_fps(60);
    while window.is_open() && !window.is_key_down(Key::Escape) {
        window.update_with_buffer(&buffer, width, height).map_err(failure)?;
    }
    Ok(())
}

impl Extension for Window {
    fn name(&self) -> &'static str {
        "FORTHLING-WINDOW"
    }

    fn install(&self, f: &mut Forth) {
        f.add_help(include_str!("help.txt"));
        let wid = f.wordlist("WINDOW");
        // SHOW-WINDOW ( -- ): open a window with the canvas; wait until it is closed or Esc
        // is pressed.
        f.add_word(wid, "SHOW-WINDOW", show).expect("WINDOW is a word list");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_canvas_is_enlarged_by_a_whole_factor() {
        assert_eq!(factor(128, 80), 10);
        assert_eq!(factor(64, 64), 12);
        assert_eq!(factor(8, 8), MAX_FACTOR);
    }

    #[test]
    fn large_canvas_is_not_shrunk() {
        assert_eq!(factor(4096, 4096), 1);
        assert_eq!(factor(0, 0), MAX_FACTOR);
    }

    #[test]
    fn rgba_becomes_0rgb() {
        assert_eq!(pixels(&[1, 2, 3, 255, 255, 0, 0, 255]), vec![0x01_02_03, 0xFF_00_00]);
    }
}
