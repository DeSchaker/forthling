//! Terminal input: standard input, read by a background thread so that KEY?
//! can tell whether input is waiting without blocking.

use crate::*;
use std::io::Read;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

/// What the terminal reads from: buffered input that can also tell, without
/// blocking, whether input is waiting.
pub trait Input: BufRead {
    /// Whether input is waiting. The default looks ahead with `fill_buf`,
    /// which is right for input that never blocks, such as a string.
    fn ready(&mut self) -> bool {
        self.fill_buf().is_ok_and(|b| !b.is_empty())
    }
}

impl<T: AsRef<[u8]>> Input for io::Cursor<T> {}

/// Standard input, one line at a time. The thread that reads it starts on
/// first use, so a Forth that never reads the terminal leaves stdin alone.
#[derive(Default)]
pub(crate) struct Stdin {
    lines: Option<Receiver<Vec<u8>>>,
    buf: Vec<u8>,
    pos: usize,
    ended: bool,
}

impl Stdin {
    fn lines(&mut self) -> &Receiver<Vec<u8>> {
        self.lines.get_or_insert_with(|| {
            let (tx, rx) = mpsc::channel();
            thread::spawn(move || {
                let mut stdin = io::stdin().lock();
                loop {
                    let mut line = Vec::new();
                    match stdin.read_until(b'\n', &mut line) {
                        Ok(n) if n > 0 && tx.send(line).is_ok() => {}
                        _ => break, // end of input, an error, or nobody listens
                    }
                }
            });
            rx
        })
    }

    /// Takes the next line into the buffer; false when there is none (yet).
    fn next_line(&mut self, wait: bool) -> bool {
        if self.ended {
            return false;
        }
        let line = if wait {
            self.lines().recv().map_err(|_| TryRecvError::Disconnected)
        } else {
            self.lines().try_recv()
        };
        match line {
            Ok(line) => {
                (self.buf, self.pos) = (line, 0);
                true
            }
            Err(TryRecvError::Empty) => false,
            Err(TryRecvError::Disconnected) => {
                self.ended = true;
                false
            }
        }
    }
}

impl Read for Stdin {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let n = {
            let buf = self.fill_buf()?;
            let n = buf.len().min(out.len());
            out[..n].copy_from_slice(&buf[..n]);
            n
        };
        self.consume(n);
        Ok(n)
    }
}

impl BufRead for Stdin {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.pos == self.buf.len() {
            self.buf.clear();
            self.pos = 0;
            self.next_line(true);
        }
        Ok(&self.buf[self.pos..])
    }

    fn consume(&mut self, n: usize) {
        self.pos = (self.pos + n).min(self.buf.len());
    }
}

impl Input for Stdin {
    /// True once a whole line has been typed: the terminal sends input per line.
    fn ready(&mut self) -> bool {
        self.pos < self.buf.len() || self.next_line(false)
    }
}
