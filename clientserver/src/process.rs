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
    pub async fn run(tx: Sender<ServerMessage>, rx: Receiver<ServerMessage>) {
        log::info!("run");
        if let Err(e) = tx.send(ServerMessage::Message(Message::TestResponse)).await {
            log::error!("unable to send: {:?}", e);
        }
    }
}


