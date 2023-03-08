use std::{os::{fd::RawFd, unix::prelude::OsStrExt}, path::{Path}};

use libc::AT_FDCWD;

use crate::io::UringFuture;

pub struct File {
    fd: RawFd,
    offset: i64,
}

impl File {
    pub async fn create<T: AsRef<Path>>(path: T) -> Self {
        let fd = io_uring::types::Fd(AT_FDCWD);
        let entry = io_uring::opcode::OpenAt::new(fd, path.as_ref().as_os_str().as_bytes().as_ptr());
        let entry = entry.flags(libc::O_CREAT | libc::O_RDWR | libc::O_TRUNC);
        let entry = entry.mode(0o644);
        let entry = entry.build();
        let result = UringFuture::new(entry).await;

        File {
            fd: result,
            offset: 0,
        }
    }

    pub async fn open<T: AsRef<Path>>(path: T) -> Self {
        let fd = io_uring::types::Fd(AT_FDCWD);
        let entry = io_uring::opcode::OpenAt::new(fd, path.as_ref().as_os_str().as_bytes().as_ptr());
        let entry = entry.flags(libc::O_RDWR);
        let entry = entry.mode(0o644);
        let entry = entry.build();
        let result = UringFuture::new(entry).await;

        File {
            fd: result,
            offset: 0,
        }
    }

    pub async fn read(&mut self, mut buf: Vec<u8>) -> Vec<u8> {
        let len = buf.len() as u32;
        let entry = io_uring::opcode::Read::new(io_uring::types::Fd(self.fd), buf.as_mut_ptr() as *mut u8, len)
            .offset(self.offset)
            .build();
        let result = UringFuture::new(entry).await;
        self.offset += result as i64;
        buf
    }
}