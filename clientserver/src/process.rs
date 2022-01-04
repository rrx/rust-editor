use crate::common::*;
use tokio::sync::mpsc::{self, Receiver, Sender};

#[derive(Clone, Debug)]
pub struct Process {
    pub id: ulid::Ulid,
    pub listeners: Vec<Sender<ServerMessage>>
}

impl Default for Process {
    fn default() -> Self {
        use ulid::Ulid;
        Self { id: Ulid::new(), listeners: vec![] }
    }
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
        //use failure::ResultExt;
        use tokio::io::{BufWriter, BufReader, AsyncBufReadExt};
        use tokio::process::Command;
        use tokio::io::AsyncReadExt;
        use std::process::{ExitStatus, Stdio};
        use futures::stream::Stream;
        use futures::{SinkExt, StreamExt};
        use tokio::io::AsyncWriteExt;
        //use futures::poll_fn)));
        use tokio_util::codec::{BytesCodec, FramedRead, FramedWrite, Decoder, Encoder};
        //let (tx_kill, rx_kill) = tokio::sync::oneshot::channel();

        //let ptymaster = AsyncPtyMaster::open().context("failed to create PTY")?;

        //let child = std::process::Command::new(cmd)
            //.args(args)
            //.spawn_pty_async(&ptymaster)
            //.context("failed to launch pty command")?;
        let mut cmd = tokio::process::Command::new(cmd);
        cmd.args(args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::piped());

        //let buf = BufWriter::new(Vec::new());
        //cmd.stdin(buf);//Stdio::piped());

        let mut child = cmd.spawn().expect("Unable to execute");
        let id = child.id();
        log::info!("id: {:?}", id);


        let mut stdout = child.stdout.take().expect("child stdout");
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
                            if let Some(mut s) = stdin.take() {
                                s.shutdown().await.unwrap();
                            }
                            //break;
                            //cmd.stdin(Stdio::null());
                        }

                        Some(ServerMessage::Data(b)) => {
                            log::info!("stdin send: {:?}", b);
                            use std::borrow::BorrowMut;
                            if let Some(mut s) = stdin.take() {
                                let mut w_stdin = FramedWrite::new(s.borrow_mut(), BytesCodec::new());
                                w_stdin.send(b).await;
                                stdin.insert(s);
                            }
                            //let mut w_stdin = FramedWrite::new(stdin.borrow_mut(), BytesCodec::new());
                            //w_stdin.send(b).await;
                        }
                        _ => ()
                    }
                }

                result = r_stdout.next() => {
                    match result {
                        Some(Ok(v)) => {
                            println!("stdout: {:?}", v);
                            tx.send(ServerMessage::Data(bytes::Bytes::from(v))).await;
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
                            println!("stderr: {:?}", v);
                            tx.send(ServerMessage::Data(bytes::Bytes::from(v))).await;
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
                    match x {
                       Ok(status) => {
                           let code = status.code();
                           log::info!("status: {:?}", code);
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


