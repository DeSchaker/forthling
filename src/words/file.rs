//! The FILE word set; the ior of a failing file word is its THROW code.

use crate::*;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};

pub(crate) fn install(f: &mut Forth) {
    f.prim("R/O", |f| {
        f.push(R_O);
        Ok(())
    });
    f.prim("W/O", |f| {
        f.push(W_O);
        Ok(())
    });
    f.prim("R/W", |f| {
        f.push(R_W);
        Ok(())
    });
    f.prim("BIN", |f| {
        let fam = f.pop()?;
        f.push(fam | 4);
        Ok(())
    });
    f.prim("OPEN-FILE", |f| {
        let fam = f.pop()?;
        let name = f.pop_name()?;
        f.open_with(&name, fam, false, -69);
        Ok(())
    });
    f.prim("CREATE-FILE", |f| {
        let fam = f.pop()?;
        let name = f.pop_name()?;
        f.open_with(&name, fam, true, -63);
        Ok(())
    });
    f.prim("CLOSE-FILE", |f| {
        let fid = f.pop()?;
        let closed = f.close_file(fid);
        f.push(ior(closed.then_some(()), -62));
        Ok(())
    });
    f.prim("DELETE-FILE", |f| {
        let name = f.pop_name()?;
        f.push(ior(fs::remove_file(name).ok(), -64));
        Ok(())
    });
    f.prim("RENAME-FILE", |f| {
        let to = f.pop_name()?;
        let from = f.pop_name()?;
        f.push(ior(fs::rename(from, to).ok(), -72));
        Ok(())
    });
    f.prim("FILE-STATUS", |f| {
        let name = f.pop_name()?;
        f.push(0);
        f.push(ior(fs::metadata(name).ok(), -67));
        Ok(())
    });
    f.prim("FILE-POSITION", |f| {
        let fid = f.pop()?;
        let pos = f.with_file(fid, |file| file.stream_position());
        f.push_size(pos, -65);
        Ok(())
    });
    f.prim("FILE-SIZE", |f| {
        let fid = f.pop()?;
        let size = f.with_file(fid, |file| Ok(file.metadata()?.len()));
        f.push_size(size, -66);
        Ok(())
    });
    f.prim("REPOSITION-FILE", |f| {
        let fid = f.pop()?;
        let pos = u64::try_from(f.pop_ud()?).unwrap_or(u64::MAX);
        let done = f.with_file(fid, |file| file.seek(SeekFrom::Start(pos)));
        f.push(ior(done, -73));
        Ok(())
    });
    f.prim("RESIZE-FILE", |f| {
        let fid = f.pop()?;
        let size = u64::try_from(f.pop_ud()?).unwrap_or(u64::MAX);
        let done = f.with_file(fid, |file| file.set_len(size));
        f.push(ior(done, -74));
        Ok(())
    });
    f.prim("FLUSH-FILE", |f| {
        let fid = f.pop()?;
        let done = f.with_file(fid, |file| file.flush());
        f.push(ior(done, -68));
        Ok(())
    });
    f.prim("READ-FILE", |f| {
        let fid = f.pop()?;
        let (a, n) = f.pop2()?;
        let i = f.check(a, n.max(0))?;
        let mut data = Vec::new();
        let done = f.with_file(fid, |file| file.take(n.max(0) as u64).read_to_end(&mut data));
        f.mem[i..i + data.len()].copy_from_slice(&data);
        f.push(data.len() as Cell);
        f.push(ior(done, -70));
        Ok(())
    });
    f.prim("READ-LINE", |f| {
        let fid = f.pop()?;
        let (a, n) = f.pop2()?;
        let i = f.check(a, n.max(0))?;
        let line = f.with_file(fid, |file| read_line_from(file, n.max(0) as usize));
        let (data, more) = match &line {
            Some(Some(data)) => (&data[..], true),
            _ => (&[][..], false),
        };
        f.mem[i..i + data.len()].copy_from_slice(data);
        f.push(data.len() as Cell);
        f.push(flag(more));
        f.push(ior(line, -71));
        Ok(())
    });
    f.prim("WRITE-FILE", |f| {
        let fid = f.pop()?;
        let (a, n) = f.pop2()?;
        let data = f.bytes(a, n)?;
        let done = f.with_file(fid, |file| file.write_all(&data));
        f.push(ior(done, -75));
        Ok(())
    });
    f.prim("WRITE-LINE", |f| {
        let fid = f.pop()?;
        let (a, n) = f.pop2()?;
        let mut data = f.bytes(a, n)?;
        data.push(b'\n');
        let done = f.with_file(fid, |file| file.write_all(&data));
        f.push(ior(done, -76));
        Ok(())
    });
}
