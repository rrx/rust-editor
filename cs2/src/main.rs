use futures::prelude::*;
use tokio_pty_process::*;
use tokio_util::codec::{BytesCodec, Decoder, FramedRead};

#[tokio::main]
async fn main() {
    env_logger::init();
    let mut command = std::process::Command::new("ls");

    let pty = match AsyncPtyMaster::open() {
        Err(e) => {
            log::error!("Unable to open PTY: {:?}", e);
            return;
        }
        Ok(pty) => pty
    };
    let child = match command.spawn_pty_async(&pty) {
        Err(e) => {
            log::error!("Unable to spawn child: {:?}", e);
            return;
        }
        Ok(child) => child,
    };

    log::info!("Spawned new child process with PID {}", child.id());

    let (pty_read, pty_write) = pty.split();

    //self.pty_write = Some(pty_write);
    //self.pty_read = Some(pty_read);
    //self.child = Some(child);

    let mut reader = FramedRead::new(pty_read, BytesCodec::new());
    use futures::future::Future;

    //reader n
    //tokio::select! {
        //x = reader.recv() => {
            //log::info!("{:?}", x);
        //}
    //}
    let result = child.wait();
    log::info!("wait: {:?}", result);

    //Self::add_stream(FramedRead::new(pty_read, BytesCodec::new()), ctx);
}

//pub struct P2 {
    //pty_write: Option<AsyncPtyMasterWriteHalf>,
    ////pty_read: Option<AsyncPtyMasterReadHalf>,
    //child: Option<Child>,
    //command: std::process::Command,
//}

//impl P2 {
    //pub fn new(command: std::process::Command) -> Self {
        //Self { pty_write: None,
            ////pty_read: None,
            //child: None, command }
    //}

    //pub async fn start(&mut self) {
    //}

    //fn asdf(&mut self) {
        ////let stream = FramedRead::new(self.pty_read.unwrap(), BytesCodec::new());
    //}
//}

