use clientserver::*;
use tokio::sync::mpsc;
use tokio::time;

#[tokio::main]
async fn main() {
    env_logger::init();
    let (app_tx, mut app_rx) = mpsc::channel(10);
    let (process_tx, process_rx) = mpsc::channel(10);
    let mut argv = std::env::args().collect::<Vec<_>>();
    //.split_at(1);
    let mut cmd = argv.split_off(1);
    let args = cmd.split_off(1);
    log::info!("{:?}", (&cmd,&args));

    tokio::spawn(Process::run_pty(cmd.get(0).unwrap().into(), args, app_tx, process_rx));

    let mut interval = time::interval(time::Duration::from_secs(2));

    //process_tx.send(ServerMessage::Kill).await;
    //process_tx.send(ServerMessage::SIGHUP).await;
    //process_tx.send(ServerMessage::SIGTERM).await;

    process_tx.send(ServerMessage::Data(bytes::Bytes::from("asdf\n"))).await;
    //process_tx.send(ServerMessage::EOF).await;
    //process_tx.send(ServerMessage::EOF).await;
    process_tx.send(ServerMessage::Data(bytes::Bytes::from("asdf\n"))).await;

    loop {
        tokio::select! {
            x = app_rx.recv() => {
                match x {
                    Some(v) => {
                        log::info!("rx: {:?}", v);
                    }
                    None => break
                }
            }
        }
    }
}


