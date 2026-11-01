//! The FILE word set; the ior of a failing file word is its THROW code.

use crate::*;
use std::io::{Read, SeekFrom};

pub(crate) fn install(f: &mut Forth) {
    install_management(f);
    install_io(f);
}

fn install_management(f: &mut Forth) {
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
        f.push(ior(closed, CLOSE_FILE_FAILED));
        Ok(())
    });
    f.prim("DELETE-FILE", |f| {
        let name = f.pop_name()?;
        let removed = f.permits(Permission::Files).then(|| f.vfs.delete(&name).ok());
        f.push(permitted_ior(removed, DELETE_FILE_FAILED));
        Ok(())
    });
    f.prim("RENAME-FILE", |f| {
        let to = f.pop_name()?;
        let from = f.pop_name()?;
        let renamed = f.permits(Permission::Files).then(|| f.vfs.rename(&from, &to).ok());
        f.push(permitted_ior(renamed, RENAME_FILE_FAILED));
        Ok(())
    });
    f.prim("FILE-STATUS", |f| {
        let name = f.pop_name()?;
        f.push(0);
        let status = f.permits(Permission::Files).then(|| f.vfs.exists(&name).then_some(()));
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
        let size = f.with_file(fid, |file| file.size());
        f.push_size(size, FILE_SIZE_FAILED);
        Ok(())
    });
    f.prim("REPOSITION-FILE", |f| {
        let fid = f.pop()?;
        let pos = u64::try_from(f.pop_ud()?).unwrap_or(u64::MAX);
        let done = f.with_file(fid, |file| file.seek(SeekFrom::Start(pos)));
        f.push(ior(done.is_some(), REPOSITION_FILE_FAILED));
        Ok(())
    });
    f.prim("RESIZE-FILE", |f| {
        let fid = f.pop()?;
        let size = u64::try_from(f.pop_ud()?).unwrap_or(u64::MAX);
        let done = f.with_file(fid, |file| file.set_size(size));
        f.push(ior(done.is_some(), RESIZE_FILE_FAILED));
        Ok(())
    });
    f.prim("FLUSH-FILE", |f| {
        let fid = f.pop()?;
        let done = f.with_file(fid, |file| file.flush());
        f.push(ior(done.is_some(), FLUSH_FILE_FAILED));
        Ok(())
    });
}

fn install_io(f: &mut Forth) {
    f.prim("READ-FILE", |f| {
        let fid = f.pop()?;
        let (i, n) = f.pop_str()?;
        f.charge(n)?;
        let mut data = Vec::new();
        let done = f.with_file(fid, |file| file.take(n as u64).read_to_end(&mut data));
        f.mem[i..i + data.len()].copy_from_slice(&data);
        f.push(to_cell(data.len()));
        f.push(ior(done.is_some(), READ_FILE_FAILED));
        Ok(())
    });
    f.prim("READ-LINE", |f| {
        let fid = f.pop()?;
        let (i, n) = f.pop_str()?;
        f.charge(n)?;
        let line = f.with_file(fid, |file| read_line_from(file, n));
        let (data, more) = match &line {
            Some(Some(data)) => (&data[..], true),
            _ => (&[][..], false),
        };
        f.mem[i..i + data.len()].copy_from_slice(data);
        f.push(to_cell(data.len()));
        f.push(flag(more));
        f.push(ior(line.is_some(), READ_LINE_FAILED));
        Ok(())
    });
    f.prim("WRITE-FILE", |f| {
        let fid = f.pop()?;
        let (a, n) = f.pop2()?;
        let data = f.bytes(a, n)?;
        f.charge(data.len())?;
        let done = f.with_file(fid, |file| file.write_all(&data));
        f.push(ior(done.is_some(), WRITE_FILE_FAILED));
        Ok(())
    });
    f.prim("WRITE-LINE", |f| {
        let fid = f.pop()?;
        let (a, n) = f.pop2()?;
        let mut data = f.bytes(a, n)?;
        data.push(b'\n');
        f.charge(data.len())?;
        let done = f.with_file(fid, |file| file.write_all(&data));
        f.push(ior(done.is_some(), WRITE_LINE_FAILED));
        Ok(())
    });
}

/// The ior of a file operation that the sandbox may forbid: None when it
/// was not permitted, else the result of the operation.
fn permitted_ior<T>(result: Option<Option<T>>, code: Cell) -> Cell {
    result.map_or(NOT_PERMITTED, |r| ior(r.is_some(), code))
}
