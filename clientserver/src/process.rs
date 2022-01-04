use crate::common::*;
use tokio::sync::mpsc::{self, Receiver, Sender};

#[derive(Clone, Debug)]
pub struct Process {
    pub id: ulid::Ulid,
    pub listeners: Vec<Sender<ServerMessage>>,
}

impl Default for Process {
    fn default() -> Self {
        use ulid::Ulid;
        Self { id: Ulid::new(), listeners: vec![] }
    }
}


use std::os::unix::prelude::RawFd;
use tokio::fs::File;
async fn open_async_pty_slave(fd: RawFd) -> Result<File, std::io::Error> {
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


impl Process {
    pub async fn run(cmd: String, args: Vec<String>, tx: Sender<ServerMessage>, rx: Receiver<ServerMessage>) {
        log::info!("run");
        if let Err(e) = tx.send(ServerMessage::Message(Message::TestResponse)).await {
            log::error!("unable to send: {:?}", e);
        }
    }

    pub async fn run_pty(cmd: String, args: Vec<String>, tx: Sender<ServerMessage>, mut rx: Receiver<ServerMessage>) -> Result<(), failure::Error> {
        log::info!("run");
        if let Err(e) = tx.send(ServerMessage::Message(Message::TestResponse)).await {
            log::error!("unable to send: {:?}", e);
            return Err(e.into());
        }

        //use tokio_pty_process::{AsyncPtyMaster, Child, CommandExt};
        use tokio_pty_process::AsyncPtyMaster;
        use std::os::unix::io::AsRawFd;
        use failure::ResultExt;
        use tokio::io::{BufWriter, BufReader, AsyncBufReadExt};
        use tokio::process::Command;
        use std::process::{ExitStatus, Stdio};
        use futures::stream::Stream;
        use futures::{SinkExt, StreamExt};
        use tokio::io::AsyncWriteExt;
        use tokio_util::codec::{BytesCodec, FramedRead, FramedWrite, Decoder, Encoder};
        //let (tx_kill, rx_kill) = tokio::sync::oneshot::channel();

        let ptymaster = AsyncPtyMaster::open().context("failed to create PTY")?;
        let master_fd = ptymaster.as_raw_fd();
        let slave = open_async_pty_slave(master_fd).await?;
        let slave_fd = slave.as_raw_fd();

        //let child = std::process::Command::new(cmd)
        //.args(args)
        //.spawn_pty_async(&ptymaster)
        //.context("failed to launch pty command")?;
        let mut cmd = tokio::process::Command::new(cmd);
        //cmd.spawn_pty_async(&ptymaster)
        cmd.args(args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::piped());
        //let fd1: Stdio = slave.try_clone().await?.into_std().await.into();
        //let fd2: Stdio = slave.try_clone().await?.into_std().await.into();
        //let fd3: Stdio = slave.try_clone().await?.into_std().await.into();
        //log::info!("fd: {:?}", (&fd1, &fd2, &fd3));
        //cmd.stdin(fd1);
        //cmd.stdout(fd2);
        //cmd.stderr(fd3);

        let raw = false;
        unsafe {
            cmd.pre_exec(move || {
                if raw {
                    let mut attrs: libc::termios = std::mem::zeroed();

                    if libc::tcgetattr(slave_fd, &mut attrs as _) != 0 {
                        return Err(std::io::Error::last_os_error());
                    }

                    libc::cfmakeraw(&mut attrs as _);

                    if libc::tcsetattr(slave_fd, libc::TCSANOW, &attrs as _) != 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                }

                // This is OK even though we don't own master since this process is
                // about to become something totally different anyway.
                if libc::close(master_fd) != 0 {
                    return Err(std::io::Error::last_os_error());
                }

                if libc::setsid() < 0 {
                    return Err(std::io::Error::last_os_error());
                }

                //if libc::ioctl(0, libc::TIOCSCTTY.into(), 1) != 0 {
                //return Err(std::io::Error::last_os_error());
                //}
                Ok(())
            });
        }

        let mut child = cmd.spawn().expect("Unable to execute");
        let id: i32 = child.id().unwrap().try_into()?;
        log::info!("id: {:?}", id);


        log::info!("stdout: {:?}", child.stdout);

        let mut stdout = child.stdout.take().expect("child stdout");
        //let mut stdout = slave.try_clone()?;
        let mut stderr = child.stderr.take().expect("child stderr");
        let mut stdin = Some(child.stdin.take().expect("child stdin"));

        let mut r_stdout = FramedRead::new(stdout, BytesCodec::new());
        let mut r_stderr = FramedRead::new(stderr, BytesCodec::new());

        loop {
            tokio::select! {
                m = rx.recv() => {
                    log::info!("process rx: {:?}", m);
                    match m {
                        Some(ServerMessage::EOF) => {
                            // take ownership of stdin and shut it down
                            if let Some(mut s) = stdin.take() {
                                s.shutdown().await.unwrap();
                            }
                        }

                        Some(ServerMessage::Kill) => {
                            child.kill().await?;
                        }

                        Some(ServerMessage::SIGHUP) => {
                            use nix::unistd::Pid;
                            use nix::sys::signal::{self, Signal};
                            signal::kill(Pid::from_raw(id), Signal::SIGHUP).unwrap();
                        }

                        Some(ServerMessage::SIGTERM) => {
                            use nix::unistd::Pid;
                            use nix::sys::signal::{self, Signal};
                            signal::kill(Pid::from_raw(id), Signal::SIGTERM).unwrap();
                        }

                        Some(ServerMessage::Data(b)) => {
                            log::info!("stdin send: {:?}", b);
                            use std::borrow::BorrowMut;
                            // if stdin is still a thing, take ownership, then write to it
                            // put stdin back when done with it
                            if let Some(mut s) = stdin.take() {
                                let mut w_stdin = FramedWrite::new(s.borrow_mut(), BytesCodec::new());
                                w_stdin.send(b).await?;
                                let _ = stdin.insert(s);
                            }
                        }
                        _ => ()
                    }
                }

                result = r_stdout.next() => {
                    match result {
                        Some(Ok(v)) => {
                            log::info!("stdout: {:?}", v);
                            tx.send(ServerMessage::Data(bytes::Bytes::from(v))).await?;
                        }
                        Some(Err(e)) => {
                            log::error!("error: {:?}", e);
                            //break;
                        }
                        None => ()
                    }
                }

                result = r_stderr.next() => {
                    match result {
                        Some(Ok(v)) => {
                            log::error!("stderr: {:?}", v);
                            tx.send(ServerMessage::Data(bytes::Bytes::from(v))).await?;
                        }
                        Some(Err(e)) => {
                            log::error!("error: {:?}", e);
                            //break;
                        }
                        None => ()
                    }
                }

                x = child.wait() => {
                    log::info!("wait: {:?}", x);
                    use std::os::unix::process::ExitStatusExt;

                    match x {
                        Ok(status) => {
                            let success = status.success();
                            if let Some(sig) = status.signal() {
                                log::info!("caught signal: {:?}, core dumped: {}, continued: {}, stopped: {:?}",
                                           sig,
                                           // these features will be availble in the next version
                                           // 1.58
                                           // See: https://doc.rust-lang.org/nightly/std/os/unix/process/trait.ExitStatusExt.html
                                           false, // status.core_dumped()
                                           false, // status.continued()
                                           false //status.stopped_signal()
                                          );
                            }
                            if let Some(code) = status.code() {
                                log::info!("exit code: {}", code);
                            }
                        }
                        Err(err) => {
                            log::error!("error: {:?}", err);
                        }
                    }
                    break;
                }
            }
        }

        Ok(())
    }

}


