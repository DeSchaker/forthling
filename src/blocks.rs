//! Blocks: 1024-byte blocks in a file, read through a few buffers in Forth
//! memory. LOAD makes a block an input source (see interpreter.rs).

use crate::*;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

pub(crate) const BLOCK_SIZE: usize = 1024;
pub(crate) const BLOCK_BUFFERS: usize = 8;
pub(crate) const MAX_BLOCK: u64 = 1 << 20; // so the block file stays below 1 GiB
pub(crate) const BLOCK_LINE: usize = 64; // a block is shown and commented as 16 lines of 64

/// What one block buffer holds.
#[derive(Clone, Copy, Default)]
pub(crate) struct BlockBuffer {
    block: u64, // 0: none
    dirty: bool,
    used: u64, // when it was last used, to replace the least recently used one
}

pub(crate) struct Blocks {
    path: PathBuf,
    file: Option<File>,
    bufs: [BlockBuffer; BLOCK_BUFFERS],
    current: Option<usize>, // the buffer UPDATE marks
    clock: u64,
}

impl Blocks {
    pub(crate) fn new(path: PathBuf) -> Self {
        Blocks { path, file: None, bufs: Default::default(), current: None, clock: 0 }
    }
}

fn buffer_addr(i: usize) -> usize {
    BLOCK_BUFS + i * BLOCK_SIZE
}

impl Forth {
    /// Uses another block file from now on. The buffers are written back
    /// to the old file first.
    pub fn set_block_file(&mut self, path: impl Into<PathBuf>) -> Result<(), Throw> {
        self.save_buffers()?;
        self.empty_buffers();
        self.blocks.file = None;
        self.blocks.path = path.into();
        Ok(())
    }

    /// BLOCK (`read` true) and BUFFER: the address of a buffer that holds
    /// block u, reading it from the file only for BLOCK.
    pub(crate) fn block(&mut self, u: Cell, read: bool) -> Res<usize> {
        let u = match u64::try_from(u) {
            Ok(u) if (1..=MAX_BLOCK).contains(&u) => u,
            _ => return Err(Throw::with(INVALID_BLOCK, u.to_string())),
        };
        let i = match self.blocks.bufs.iter().position(|b| b.block == u) {
            Some(i) => i,
            None => {
                let (i, _) =
                    self.blocks.bufs.iter().enumerate().min_by_key(|(_, b)| b.used).unwrap();
                self.write_buffer(i)?;
                if read {
                    let data = self.read_block(u)?;
                    self.mem[buffer_addr(i)..][..BLOCK_SIZE].copy_from_slice(&data);
                }
                self.blocks.bufs[i] = BlockBuffer { block: u, dirty: false, used: 0 };
                i
            }
        };
        self.blocks.clock += 1;
        self.blocks.bufs[i].used = self.blocks.clock;
        self.blocks.current = Some(i);
        Ok(buffer_addr(i))
    }

    pub(crate) fn update(&mut self) {
        if let Some(i) = self.blocks.current {
            self.blocks.bufs[i].dirty = true;
        }
    }

    /// SAVE-BUFFERS: writes every updated buffer to the file.
    pub(crate) fn save_buffers(&mut self) -> Res {
        for i in 0..BLOCK_BUFFERS {
            self.write_buffer(i)?;
        }
        if let Some(file) = &mut self.blocks.file {
            file.flush().map_err(|e| Throw::with(BLOCK_WRITE, e.to_string()))?;
        }
        Ok(())
    }

    /// EMPTY-BUFFERS: forgets all buffers without writing them.
    pub(crate) fn empty_buffers(&mut self) {
        self.blocks.bufs = Default::default();
        self.blocks.current = None;
    }

    /// Writes buffer i to the file if it was updated.
    fn write_buffer(&mut self, i: usize) -> Res {
        let b = self.blocks.bufs[i];
        if b.block == 0 || !b.dirty {
            return Ok(());
        }
        let data = self.mem[buffer_addr(i)..][..BLOCK_SIZE].to_vec();
        let file = self.block_file()?;
        let err = |e: io::Error| Throw::with(BLOCK_WRITE, e.to_string());
        // Blocks between the end of the file and this one become blank.
        let offset = (b.block - 1) * BLOCK_SIZE as u64;
        let len = file.metadata().map_err(err)?.len();
        if len < offset {
            file.seek(SeekFrom::End(0)).map_err(err)?;
            file.write_all(&vec![b' '; (offset - len) as usize]).map_err(err)?;
        }
        file.seek(SeekFrom::Start(offset)).map_err(err)?;
        file.write_all(&data).map_err(err)?;
        self.blocks.bufs[i].dirty = false;
        Ok(())
    }

    /// The contents of block u; a block beyond the end of the file is blank.
    fn read_block(&mut self, u: u64) -> Res<Vec<u8>> {
        let file = self.block_file()?;
        let err = |e: io::Error| Throw::with(BLOCK_READ, e.to_string());
        let mut data = vec![b' '; BLOCK_SIZE];
        file.seek(SeekFrom::Start((u - 1) * BLOCK_SIZE as u64)).map_err(err)?;
        let mut n = 0;
        while n < BLOCK_SIZE {
            match file.read(&mut data[n..]).map_err(err)? {
                0 => break,
                read => n += read,
            }
        }
        data[n..].fill(b' ');
        Ok(data)
    }

    /// The block file, opened (and created if need be) on first use.
    fn block_file(&mut self) -> Res<&mut File> {
        if self.blocks.file.is_none() {
            let path = &self.blocks.path;
            let file = OpenOptions::new().read(true).write(true).create(true).open(path);
            let file =
                file.map_err(|e| Throw::with(BLOCK_READ, format!("{}: {e}", path.display())))?;
            self.blocks.file = Some(file);
        }
        Ok(self.blocks.file.as_mut().unwrap())
    }

    /// LOAD: interprets block u, then goes back to the current input.
    pub(crate) fn load(&mut self, u: Cell) -> Res {
        let buf = SOURCE_AREA + self.sources.len() * LINE_MAX;
        self.push_source(Origin::Block, buf, 0)?;
        self.load_block(u)?;
        // REFILL in the text moves on to the next block, within the same run.
        let block = |f: &Forth| f.sources.last().unwrap().line;
        self.interpret_buffer().map_err(|e| e.at("block", block(self)))?;
        self.pop_source();
        Ok(())
    }

    /// Makes block u the text of the current (block) input source; its
    /// line number is the block number, so SAVE-INPUT records it.
    pub(crate) fn load_block(&mut self, u: Cell) -> Res {
        let a = self.block(u, true)?;
        let s = self.sources.last_mut().unwrap();
        let buf = s.buf;
        s.len = BLOCK_SIZE;
        s.line = u as usize;
        self.mem.copy_within(a..a + BLOCK_SIZE, buf);
        self.set(IN_ADDR, 0);
        self.sync_blk();
        Ok(())
    }

    /// BLK is the block being interpreted, or 0.
    pub(crate) fn sync_blk(&mut self) {
        let s = self.sources.last().unwrap();
        let blk = if let Origin::Block = s.origin { s.line as Cell } else { 0 };
        self.set(BLK_ADDR, blk);
    }

    /// LIST: shows block u as 16 numbered lines of 64 characters.
    pub(crate) fn list(&mut self, u: Cell) -> Res {
        let a = self.block(u, true)?;
        self.set(SCR_ADDR, u);
        let mut text = format!("\nScreen {u}\n");
        for (n, line) in self.mem[a..a + BLOCK_SIZE].chunks(BLOCK_LINE).enumerate() {
            let line: String =
                line.iter().map(|&c| if c < b' ' { ' ' } else { c as char }).collect();
            text += format!("{n:2} {line}").trim_end();
            text.push('\n');
        }
        self.emit_str(&text);
        Ok(())
    }
}
