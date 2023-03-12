use std::{os::fd::AsRawFd, io};

use crate::io::UringFuture;

pub struct TcpStream {
    socket: socket2::Socket,
}

impl TcpStream {
    pub(crate) fn new(socket: socket2::Socket) -> TcpStream {
        TcpStream { socket }
    }

    pub async fn read(&self, buf: Vec<u8>) -> io::Result<(i32, Vec<u8>)> {
        let len = buf.len() as u32;
        let capacity = buf.capacity();
        let buf = buf.leak();
        let ptr = buf.as_mut_ptr();
        let entry = io_uring::opcode::Recv::new(io_uring::types::Fd(self.socket.as_raw_fd()), ptr, len);
        let entry = entry.build();
        let result = UringFuture::new(entry).await?;

        Ok((result, unsafe { Vec::from_raw_parts(ptr, result.try_into().unwrap(), capacity) }))
    }

    pub async fn write(&self, buf: Vec<u8>) -> io::Result<(i32, Vec<u8>)> {
        let len = buf.len() as u32;
        let capacity = buf.capacity();
        let buf = buf.leak();
        let ptr = buf.as_ptr();
        let entry = io_uring::opcode::Send::new(io_uring::types::Fd(self.socket.as_raw_fd()), ptr, len);
        let entry = entry.build();
        let result = UringFuture::new(entry).await?;

        Ok((result, unsafe { Vec::from_raw_parts(ptr as *mut u8, len.try_into().unwrap(), capacity) }))
    }
}