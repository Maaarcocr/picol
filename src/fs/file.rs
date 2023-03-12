use std::{os::{fd::RawFd, unix::ffi::OsStrExt}, path::{Path}, ffi::{CString}, io};

use libc::AT_FDCWD;

use crate::io::UringFuture;

pub struct File {
    fd: RawFd,
    offset: i64,
}

fn cstr<T: AsRef<Path>>(p: T) -> CString {
    CString::new(p.as_ref().as_os_str().as_bytes()).unwrap()
}

impl File {
    pub async fn create<T: AsRef<Path>>(path: T) -> io::Result<Self> {
        let fd = io_uring::types::Fd(AT_FDCWD);
        let path = cstr(path);
        let entry = io_uring::opcode::OpenAt::new(fd, path.as_ptr());
        let entry = entry.flags(libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC);
        let entry = entry.mode(0o644);
        let entry = entry.build();
        let result = UringFuture::new(entry).await?;

        Ok(File {
            fd: result,
            offset: 0,
        })
    }

    pub async fn open<T: AsRef<Path>>(path: T) -> io::Result<Self> {
        let fd = io_uring::types::Fd(AT_FDCWD);
        let path = cstr(path);
        let entry = io_uring::opcode::OpenAt::new(fd, path.as_ptr());
        let entry = entry.flags(libc::O_RDWR);
        let entry = entry.mode(0o644);
        let entry = entry.build();
        let result = UringFuture::new(entry).await?;

        Ok(File {
            fd: result,
            offset: 0,
        })
    }

    pub async fn read(&mut self, buf: Vec<u8>) -> io::Result<(i32, Vec<u8>)> {
        let len = buf.len() as u32;
        let capacity = buf.capacity();
        let buf = buf.leak();
        let ptr = buf.as_mut_ptr();
        let entry = io_uring::opcode::Read::new(io_uring::types::Fd(self.fd), ptr, len)
            .offset(self.offset)
            .build();
        let result = UringFuture::new(entry).await?;

        self.offset += result as i64;
        Ok((result, unsafe { Vec::from_raw_parts(ptr, len.try_into().unwrap(), capacity) }))
    }

    pub async fn write(&mut self, buf: Vec<u8>) -> io::Result<(i32, Vec<u8>)> {
        let len = buf.len() as u32;
        let capacity = buf.capacity();
        let buf = buf.leak();
        let ptr = buf.as_ptr();
        let entry = io_uring::opcode::Write::new(io_uring::types::Fd(self.fd), ptr, len)
            .offset(self.offset)
            .build();
        let result = UringFuture::new(entry).await?;

        self.offset += result as i64;
        Ok((result, unsafe { Vec::from_raw_parts(ptr as *mut u8, len.try_into().unwrap(), capacity) }))
    }
}