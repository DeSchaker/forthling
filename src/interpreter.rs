//! The outer interpreter, input sources and the terminal.

use crate::*;
use std::io::Write;

/// An input source: where its text comes from, its line buffer in memory,
/// the number of lines read so far and, while a nested source runs, the
/// >IN to restore afterwards.
pub(crate) struct Source {
    pub(crate) origin: Origin,
    pub(crate) buf: usize,
    pub(crate) len: usize,
    pub(crate) line: usize,
    pub(crate) saved_in: Cell,
}

pub(crate) enum Origin {
    Terminal,
    Evaluate,
    // A file being included: its file id and all its lines; REFILL loads the next one.
    File { fid: Cell, lines: Rc<Vec<String>> },
    // A block being LOADed; the source's line number is the block number,
    // and REFILL loads the next block.
    Block,
}

impl Forth {
    /// Interprets text line by line, as if typed at the terminal.
    pub fn interpret(&mut self, src: &str) -> Res {
        let first = self.sources[0].line;
        let result = self.evaluation(|f| f.interpret_lines(src)).map_err(|e| self.report(e, first));
        if let Err(e) = &result {
            // QUIT leaves the data stack alone; every other error empties it.
            self.reset(e.code != QUIT);
        }
        result
    }

    pub(crate) fn interpret_lines(&mut self, src: &str) -> Res {
        for line in src.lines() {
            self.load_line(line)?;
            self.interpret_buffer()?;
            if self.bye {
                break;
            }
        }
        Ok(())
    }

    /// Puts a line in the current source's buffer and resets >IN.
    pub(crate) fn load_line(&mut self, line: &str) -> Res {
        // A byte-order mark (from Windows editors and pipes) is not input.
        let bytes = line.trim_start_matches('\u{feff}').as_bytes();
        let s = self.sources.last_mut().unwrap();
        s.line += 1;
        if bytes.len() > LINE_MAX {
            // None of the line is read, so an error report shows no word.
            s.len = 0;
            self.set(IN_ADDR, 0);
            return Err(Throw::with(PARSED_OVERFLOW, "input line too long"));
        }
        s.len = bytes.len();
        let buf = s.buf;
        self.mem[buf..buf + bytes.len()].copy_from_slice(bytes);
        self.set(IN_ADDR, 0);
        Ok(())
    }

    pub(crate) fn interpret_buffer(&mut self) -> Res {
        while !self.bye {
            let Some(w) = self.word() else { break };
            self.handle(&w)?;
            self.check_stack()?;
        }
        Ok(())
    }

    pub(crate) fn handle(&mut self, w: &str) -> Res {
        if let Some(i) = self.find_local(w) {
            self.code.push(Instr::Local(i));
            Ok(())
        } else if let Some(xt) = self.find(w) {
            if self.compiling() && !self.dict[xt].immediate {
                self.code.push(Instr::Call(xt));
                Ok(())
            } else {
                self.execute(xt)
            }
        } else if let Some((n, double)) = self.number(w) {
            // A double cell is its low cell followed by its high cell.
            let cells = if double { vec![n as u64, (n >> 64) as u64] } else { vec![n as u64] };
            for c in cells {
                if self.compiling() {
                    self.code.push(Instr::Lit(c as Cell));
                } else {
                    self.push(c as Cell);
                }
            }
            Ok(())
        } else if let Some(r) = self.float_literal(w) {
            if self.compiling() {
                self.code.push(Instr::FLit(r));
                Ok(())
            } else {
                self.fpush(r)
            }
        } else {
            Err(self.undefined(w))
        }
    }

    /// Interprets a string in memory as a nested input source (EVALUATE).
    pub(crate) fn evaluate(&mut self, a: usize, n: usize) -> Res {
        self.push_source(Origin::Evaluate, a, n)?;
        self.interpret_buffer()?;
        self.pop_source();
        Ok(())
    }

    /// Loads the next line of the current source (REFILL); false when there
    /// is none, which is always the case for an EVALUATEd string.
    pub(crate) fn refill(&mut self) -> Res<bool> {
        let s = self.sources.last().unwrap();
        let line = match &s.origin {
            Origin::Block => return self.load_block(s.line as Cell + 1).map(|_| true),
            Origin::Evaluate => None,
            Origin::File { lines, .. } => lines.get(s.line).cloned(),
            Origin::Terminal => self.read_line().map(|l| String::from_utf8_lossy(&l).into_owned()),
        };
        match line {
            Some(line) => self.load_line(&line).map(|_| true),
            None => Ok(false),
        }
    }

    /// SOURCE-ID: 0 for the terminal, -1 for EVALUATE, else the file id.
    pub(crate) fn source_id(&self) -> Cell {
        match self.sources.last().unwrap().origin {
            Origin::Terminal | Origin::Block => 0,
            Origin::Evaluate => -1,
            Origin::File { fid, .. } => fid,
        }
    }

    /// Makes `buf` the current input source, remembering the >IN of the
    /// source it interrupts.
    pub(crate) fn push_source(&mut self, origin: Origin, buf: usize, len: usize) -> Res {
        if self.sources.len() == MAX_SOURCES {
            return Err(Throw::new(NESTING_TOO_DEEP));
        }
        let saved_in = self.get(IN_ADDR);
        self.sources.last_mut().unwrap().saved_in = saved_in;
        self.sources.push(Source { origin, buf, len, line: 0, saved_in: 0 });
        self.set(IN_ADDR, 0);
        self.sync_blk();
        Ok(())
    }

    /// SAVE-INPUT: >IN, the line number and the nesting depth.
    pub(crate) fn save_input(&mut self) {
        let s = self.sources.last().unwrap();
        let (line, depth) = (s.line as Cell, self.sources.len() as Cell);
        self.push(self.get(IN_ADDR));
        self.push(line);
        self.push(depth);
        self.push(3);
    }

    /// The core of RESTORE-INPUT; returns whether the input was restored.
    /// A file can go back to any line, the terminal and EVALUATE only within
    /// the current line.
    pub(crate) fn restore_input(&mut self) -> Res<bool> {
        let n = self.pop()?;
        if n != 3 {
            for _ in 0..n.max(0) {
                self.pop()?;
            }
            return Ok(false);
        }
        let depth = self.pop()?;
        let line = self.pop()?;
        let to_in = self.pop()?;
        let s = self.sources.last().unwrap();
        if depth != self.sources.len() as Cell {
            return Ok(false);
        }
        if line != s.line as Cell {
            if let Origin::Block = s.origin {
                self.load_block(line)?;
                self.set(IN_ADDR, to_in);
                return Ok(true);
            }
            let Origin::File { lines, .. } = &s.origin else { return Ok(false) };
            let Some(text) = usize::try_from(line - 1).ok().and_then(|i| lines.get(i)) else {
                return Ok(false);
            };
            let text = text.clone();
            self.sources.last_mut().unwrap().line = line as usize - 1;
            self.load_line(&text)?;
        }
        self.set(IN_ADDR, to_in);
        Ok(true)
    }

    pub(crate) fn pop_source(&mut self) {
        self.sources.pop();
        let saved_in = self.sources.last().unwrap().saved_in;
        self.set(IN_ADDR, saved_in);
        self.sync_blk();
    }

    /// Recovers from an error: clears the stacks (the data stack only if asked),
    /// returns to the terminal and drops a half-finished definition.
    pub(crate) fn reset(&mut self, clear_stack: bool) {
        if clear_stack {
            self.stack.clear();
            self.fstack.clear();
        }
        self.rstack.clear();
        self.loops.clear();
        self.frames.clear();
        self.locals.clear();
        self.ctrl.clear();
        self.end_local_scope();
        self.sources.truncate(1);
        self.sync_blk();
        if let Some(xt) = self.current.take() {
            self.forget_from(xt);
        }
        self.set_state(false);
        self.code.clear();
        self.reset_json();
    }

    pub fn flush(&mut self) {
        io::stdout().write_all(&self.out).ok();
        self.out.clear();
        io::stdout().flush().ok();
    }

    /// Reads a line from the terminal without its line ending; None at end of input.
    pub fn read_line(&mut self) -> Option<Vec<u8>> {
        if self.interactive {
            self.flush();
        }
        let mut line = Vec::new();
        if self.input.read_until(b'\n', &mut line).unwrap_or(0) == 0 {
            return None;
        }
        while matches!(line.last(), Some(b'\n' | b'\r')) {
            line.pop();
        }
        Some(line)
    }

    /// Reads one character from the terminal; None at end of input.
    pub(crate) fn read_key(&mut self) -> Option<u8> {
        if self.interactive {
            self.flush();
        }
        let c = *self.input.fill_buf().ok()?.first()?;
        self.input.consume(1);
        Some(c)
    }
}
