//! The FILE word set; the ior of a failing file word is its THROW code.

use crate::*;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};

pub(crate) fn install(f: &mut Forth) {
    f.prim("OPEN-FILE", |f| {
        let fam = f.pop()?;
        let name = f.pop_name()?;
        f.open_with(&name, fam, false, OPEN_FILE_FAILED);
        Ok(())
    });
    f.prim("CREATE-FILE", |f| {
        let fam = f.pop()?;
        let name = f.pop_name()?;
        f.open_with(&name, fam, true, CREATE_FILE_FAILED);
        Ok(())
    });
    f.prim("CLOSE-FILE", |f| {
        let fid = f.pop()?;
        let closed = f.close_file(fid);
        f.push(ior(closed.then_some(()), CLOSE_FILE_FAILED));
        Ok(())
    });
    f.prim("DELETE-FILE", |f| {
        let name = f.pop_name()?;
        let removed = f.permits(Permission::Files).then(|| fs::remove_file(name).ok());
        f.push(permitted_ior(removed, DELETE_FILE_FAILED));
        Ok(())
    });
    f.prim("RENAME-FILE", |f| {
        let to = f.pop_name()?;
        let from = f.pop_name()?;
        let renamed = f.permits(Permission::Files).then(|| fs::rename(from, to).ok());
        f.push(permitted_ior(renamed, RENAME_FILE_FAILED));
        Ok(())
    });
    f.prim("FILE-STATUS", |f| {
        let name = f.pop_name()?;
        f.push(0);
        let status = f.permits(Permission::Files).then(|| fs::metadata(name).ok());
        f.push(permitted_ior(status, FILE_STATUS_FAILED));
        Ok(())
    });
    f.prim("FILE-POSITION", |f| {
        let fid = f.pop()?;
        let pos = f.with_file(fid, |file| file.stream_position());
        f.push_size(pos, FILE_POSITION_FAILED);
        Ok(())
    });
    f.prim("FILE-SIZE", |f| {
        let fid = f.pop()?;
        let size = f.with_file(fid, |file| Ok(file.metadata()?.len()));
        f.push_size(size, FILE_SIZE_FAILED);
        Ok(())
    });
    f.prim("REPOSITION-FILE", |f| {
        let fid = f.pop()?;
        let pos = u64::try_from(f.pop_ud()?).unwrap_or(u64::MAX);
        let done = f.with_file(fid, |file| file.seek(SeekFrom::Start(pos)));
        f.push(ior(done, REPOSITION_FILE_FAILED));
        Ok(())
    });
    f.prim("RESIZE-FILE", |f| {
        let fid = f.pop()?;
        let size = u64::try_from(f.pop_ud()?).unwrap_or(u64::MAX);
        let done = f.with_file(fid, |file| file.set_len(size));
        f.push(ior(done, RESIZE_FILE_FAILED));
        Ok(())
    });
    f.prim("FLUSH-FILE", |f| {
        let fid = f.pop()?;
        let done = f.with_file(fid, |file| file.flush());
        f.push(ior(done, FLUSH_FILE_FAILED));
        Ok(())
    });
    f.prim("READ-FILE", |f| {
        let fid = f.pop()?;
        let (i, n) = f.pop_str()?;
        let mut data = Vec::new();
        let done = f.with_file(fid, |file| file.take(n as u64).read_to_end(&mut data));
        f.mem[i..i + data.len()].copy_from_slice(&data);
        f.push(data.len() as Cell);
        f.push(ior(done, READ_FILE_FAILED));
        Ok(())
    });
    f.prim("READ-LINE", |f| {
        let fid = f.pop()?;
        let (i, n) = f.pop_str()?;
        let line = f.with_file(fid, |file| read_line_from(file, n));
        let (data, more) = match &line {
            Some(Some(data)) => (&data[..], true),
            _ => (&[][..], false),
        };
        f.mem[i..i + data.len()].copy_from_slice(data);
        f.push(data.len() as Cell);
        f.push(flag(more));
        f.push(ior(line, READ_LINE_FAILED));
        Ok(())
    });
    f.prim("WRITE-FILE", |f| {
        let fid = f.pop()?;
        let (a, n) = f.pop2()?;
        let data = f.bytes(a, n)?;
        let done = f.with_file(fid, |file| file.write_all(&data));
        f.push(ior(done, WRITE_FILE_FAILED));
        Ok(())
    });
    f.prim("WRITE-LINE", |f| {
        let fid = f.pop()?;
        let (a, n) = f.pop2()?;
        let mut data = f.bytes(a, n)?;
        data.push(b'\n');
        let done = f.with_file(fid, |file| file.write_all(&data));
        f.push(ior(done, WRITE_LINE_FAILED));
        Ok(())
    });
}

/// The ior of a file operation that the sandbox may forbid: None when it
/// was not permitted, else the result of the operation.
fn permitted_ior<T>(result: Option<Option<T>>, code: Cell) -> Cell {
    result.map_or(NOT_PERMITTED, |r| ior(r, code))
}
