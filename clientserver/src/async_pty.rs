//use tokio_pty_process::AsyncPtyMaster;
use failure::ResultExt;
use tokio::io::{ReadBuf, BufWriter, BufReader, AsyncRead, AsyncBufReadExt, AsyncWrite };
use tokio::process::Command;
use core::task::Poll;
use core::result::Result;
use core::pin::Pin;
use std::process::{ExitStatus, Stdio};
use futures::stream::Stream;
use futures::{SinkExt, StreamExt, Future};
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
///
/// A child process that can be interacted with through a pseudo-TTY.
#[must_use = "futures do nothing unless polled"]
#[derive(Debug)]
pub struct Child {
    pub inner: tokio::process::Child,
    kill_on_drop: bool,
    reaped: bool,
    pub slave: File
    //sigchld: FlattenStream<IoFuture<Signal>>,
}
impl Child {
    fn new(inner: tokio::process::Child, slave: File) -> Child {
        Child {
            inner: inner,
            kill_on_drop: true,
            reaped: false,
            slave
            //sigchld: Signal::new(libc::SIGCHLD).flatten_stream(),
        }
    }

    pub async fn wait(&mut self) -> io::Result<ExitStatus> {
        self.inner.wait().await
    }
}

impl Master {
    pub async fun status(&mut self, mut command: Command) {
        let master_fd = self.as_raw_fd();
        let slave = self.open_slave().await?;
        let slave_fd = slave.as_raw_fd();

    //command.stdout(Stdio::piped());
    //command.stderr(Stdio::piped());
    //command.stdin(Stdio::piped());
        //command.stdin(slave.try_clone().await?.into_std().await);
        //command.stdout(slave.try_clone().await?.into_std().await);
        //command.stderr(slave.try_clone().await?.into_std().await);

        let child = command.status()?;
        log::info!("spawn: {:?}", (&child));

        Ok(child)
    }

    pub async fn spawn_pty_sync_full(&mut self, mut command: std::process::Command, raw: bool) -> Result<std::process::Child, io::Error> {
        let master_fd = self.as_raw_fd();
        let slave = self.open_slave().await?;
        let slave_fd = slave.as_raw_fd();

    //command.stdout(Stdio::piped());
    //command.stderr(Stdio::piped());
    //command.stdin(Stdio::piped());
        //command.stdin(slave.try_clone().await?.into_std().await);
        //command.stdout(slave.try_clone().await?.into_std().await);
        //command.stderr(slave.try_clone().await?.into_std().await);

        let child = command.spawn()?;
        log::info!("spawn: {:?}", (&child));

        Ok(child)
    }

    pub async fn spawn_pty_async_full(&mut self, mut command: Command, raw: bool) -> Result<Child, io::Error> {
        let master_fd = self.as_raw_fd();
        let slave = self.open_slave().await?;
        let slave_fd = slave.as_raw_fd();
        
        log::info!("spawn: {:?}", (&slave, &slave_fd, &master_fd));
        //
        //let fd1: Stdio = slave.try_clone().await?.into_std().await.into();
        //let fd2: Stdio = slave.try_clone().await?.into_std().await.into();
        //let fd3: Stdio = slave.try_clone().await?.into_std().await.into();

        //log::info!("fd: {:?}", (&fd1, &fd2, &fd3));
        let fd1 = slave.try_clone().await?;//.into_std().await;
        let fd2 = slave.try_clone().await?;//.into_std().await;
        log::info!("fd: {:?}", (&fd1, &fd2));

        //command.stdin(fd1);
        //command.stdout(fd2);
        //command.stderr(fd3);
        //command.stdout(Stdio::piped());
        //command.stderr(Stdio::piped());
        //command.stdin(Stdio::piped());


        command.stdin(slave.try_clone().await?.into_std().await);
        command.stdout(slave.try_clone().await?.into_std().await);
        command.stderr(slave.try_clone().await?.into_std().await);
        log::info!("command: {:?}", (&command));

        // XXX any need to close slave handles in the parent process beyond
        // what's done here?

        unsafe {
            command.pre_exec(move || {
                if raw {
                    let mut attrs: libc::termios = std::mem::zeroed();

                    if libc::tcgetattr(slave_fd, &mut attrs as _) != 0 {
                        return Err(io::Error::last_os_error());
                    }

                    libc::cfmakeraw(&mut attrs as _);

                    if libc::tcsetattr(slave_fd, libc::TCSANOW, &attrs as _) != 0 {
                        return Err(io::Error::last_os_error());
                    }
                }

                // This is OK even though we don't own master since this process is
                // about to become something totally different anyway.
                if libc::close(master_fd) != 0 {
                    return Err(io::Error::last_os_error());
                }

                if libc::setsid() < 0 {
                    return Err(io::Error::last_os_error());
                }

                if libc::ioctl(0, libc::TIOCSCTTY.into(), 1) != 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }

        Ok(Child::new(command.spawn()?, slave))
        //Ok(command.spawn()?)
    }
}
