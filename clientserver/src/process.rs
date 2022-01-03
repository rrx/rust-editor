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

    pub async fn run_pty(cmd: String, args: Vec<String>, tx: Sender<ServerMessage>, rx: Receiver<ServerMessage>) -> Result<(), failure::Error> {
        log::info!("run");
        if let Err(e) = tx.send(ServerMessage::Message(Message::TestResponse)).await {
            log::error!("unable to send: {:?}", e);
            return Err(e.into());
        }

        //use tokio_pty_process::{AsyncPtyMaster, Child, CommandExt};
        //use failure::ResultExt;
        use tokio::io::{BufReader, AsyncBufReadExt};
        use tokio::process::Command;
        use tokio::io::AsyncReadExt;
        use std::process::{ExitStatus, Stdio};
        use futures::stream::Stream;
        use futures::StreamExt;
        //use futures::poll_fn;
        use tokio_util::codec::{BytesCodec, FramedRead, Decoder};
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

        let mut child = cmd.spawn().expect("Unable to execute");
        let id = child.id();

        let mut stdout = child.stdout.take().expect("child stdout");
        let mut stderr = child.stderr.take().expect("child stderr");
        let mut stdin = child.stdin.take().expect("child stdin");

        let mut r = FramedRead::new(stdout, BytesCodec::new());//.into_future();
        loop {
            tokio::select! {
                result = r.next() => {
                    match result {
                        Some(Ok(v)) => println!("{:?}", v),//log::info!("{:?}", v),
                        Some(Err(e)) => {
                            log::error!("error: {:?}", e);
                            break;
                        }
                        None => break
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
                            log::error!("error: {:?}", e);
                       }
                    }
                    break;
                }
            }
        }

        Ok(())
    }

}


