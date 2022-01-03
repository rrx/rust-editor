use clientserver::*;
use tokio::sync::mpsc;

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

    Process::run_pty(cmd.get(0).unwrap().into(), args, app_tx, process_rx).await.unwrap();
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


