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

    pub async fn run_pty(cmd: String, args: Vec<String>, tx: Sender<ServerMessage>, rx: Receiver<ServerMessage>) {
        log::info!("run");
        if let Err(e) = tx.send(ServerMessage::Message(Message::TestResponse)).await {
            log::error!("unable to send: {:?}", e);
        }
        use tokio_pty_process::{AsyncPtyMaster, Child, CommandExt};
        use failure::ResultExt;

        //let (tx_kill, rx_kill) = tokio::sync::oneshot::channel();

        let ptymaster = AsyncPtyMaster::open().context("failed to create PTY").unwrap();

        let child = std::process::Command::new(cmd)
            .args(args)
            .spawn_pty_async(&ptymaster)
            .context("failed to launch pty command").unwrap();
    }

}


