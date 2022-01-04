//use tokio_pty_process::AsyncPtyMaster;
use failure::ResultExt;
use tokio::io::{ReadBuf, BufWriter, BufReader, AsyncRead, AsyncBufReadExt, AsyncWrite };
use tokio::process::Command;
use core::task::Poll;
use core::result::Result;
use core::pin::Pin;
use std::process::{ExitStatus, Stdio};
use futures::stream::Stream;
use futures::{SinkExt, StreamExt};
use tokio::io::AsyncWriteExt;
use tokio_util::codec::{BytesCodec, FramedRead, FramedWrite, Decoder, Encoder};
use std::os::unix::prelude::{AsRawFd, RawFd};
use tokio::fs::File;
use std::os::unix::io::FromRawFd;
use std::io;
use futures::task::Context;

#[derive(Debug)]
pub struct PtyFile(File);

impl PtyFile {
    pub fn new(inner: File) -> Self {
        PtyFile(inner)
    }
}

impl AsRawFd for PtyFile {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl AsyncRead for PtyFile {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context, bytes: &mut ReadBuf) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_read(cx, bytes)
    }
}

impl AsyncWrite for PtyFile {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize, io::Error>> {
        Pin::new(&mut self.0).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>, 
        cx: &mut Context<'_>
    ) -> Poll<Result<(), io::Error>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>, 
        cx: &mut Context<'_>
    ) -> Poll<Result<(), io::Error>> {
        Pin::new(&mut self.0).poll_shutdown(cx)
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>, 
        cx: &mut Context<'_>, 
        bufs: &[io::IoSlice<'_>]
    ) -> Poll<Result<usize, io::Error>> {
        Pin::new(&mut self.0).poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        Pin::new(&self.0).is_write_vectored()
    }
}

pub struct Master {
    inner: PtyFile
}

impl AsRawFd for Master {
    fn as_raw_fd(&self) -> RawFd {
        self.inner.as_raw_fd()
    }
}

impl Master {
    pub fn open() -> Result<Self, io::Error> {
        let inner = unsafe {
            // On MacOS, O_NONBLOCK is not documented as an allowed option to
            // posix_openpt(), but it is in fact allowed and functional, and
            // trying to add it later with fcntl() is forbidden. Meanwhile, on
            // FreeBSD, O_NONBLOCK is *not* an allowed option to
            // posix_openpt(), and the only way to get a nonblocking PTY
            // master is to add the nonblocking flag with fcntl() later. So,
            // we have to jump through some #[cfg()] hoops.

            const APPLY_NONBLOCK_AFTER_OPEN: bool = cfg!(target_os = "freebsd");

            let fd = if APPLY_NONBLOCK_AFTER_OPEN {
                libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY)
            } else {
                libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY | libc::O_NONBLOCK)
            };

            if fd < 0 {
                return Err(io::Error::last_os_error());
            }

            if libc::grantpt(fd) != 0 {
                return Err(io::Error::last_os_error());
            }

            if libc::unlockpt(fd) != 0 {
                return Err(io::Error::last_os_error());
            }

            if APPLY_NONBLOCK_AFTER_OPEN {
                let flags = libc::fcntl(fd, libc::F_GETFL, 0);
                if flags < 0 {
                    return Err(io::Error::last_os_error());
                }

                if libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) == -1 {
                    return Err(io::Error::last_os_error());
                }
            }

            File::from_raw_fd(fd)
        };

        Ok(Self { inner: PtyFile::new(inner) })
    }

    pub async fn open_slave(&self) -> Result<File, std::io::Error> {
        Self::open_async_pty_slave(self.as_raw_fd()).await
    }

    pub async fn open_async_pty_slave(fd: RawFd) -> Result<File, std::io::Error> {
        use tokio::fs::OpenOptions;
        use std::ffi::{OsStr, CStr};
        use std::os::unix::ffi::OsStrExt;

        let mut buf: [libc::c_char; 512] = [0; 512];

        #[cfg(not(any(target_os = "macos", target_os = "freebsd")))]
        {
            if unsafe { libc::ptsname_r(fd, buf.as_mut_ptr(), buf.len()) } != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        unsafe {
            let st = libc::ptsname(fd);
            if st.is_null() {
                return Err(io::Error::last_os_error());
            }
            libc::strncpy(buf.as_mut_ptr(), st, buf.len());
        }

        let ptsname = OsStr::from_bytes(unsafe { CStr::from_ptr(&buf as _) }.to_bytes());
        OpenOptions::new().read(true).write(true).open(ptsname).await
    }

}
