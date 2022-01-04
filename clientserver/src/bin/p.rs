use clientserver::*;
use tokio::sync::mpsc;
//use tokio::time;
use std::process::Stdio;
use tokio::io::AsyncReadExt;

#[tokio::main]
async fn main() -> Result<(), failure::Error> {
    env_logger::init();
    let mut argv = std::env::args().collect::<Vec<_>>();
    //.split_at(1);
    let mut cmd = argv.split_off(1);
    let args = cmd.split_off(1);
    log::info!("{:?}", (&cmd,&args));

    let arg0 = cmd.get(0).unwrap();

    let mut command = tokio::process::Command::new(arg0);
    command.args(args.clone());

    let mut command_sync = std::process::Command::new(arg0);
    command_sync.args(args);

    let mut master = Master::open()?;
    let mut slave = master.open_slave().await?;

    //let fd1: Stdio = slave.try_clone().await?.into_std().await.into();
    //let fd2: Stdio = slave.try_clone().await?.into_std().await.into();
    //let fd3: Stdio = slave.try_clone().await?.into_std().await.into();

    //log::info!("fd: {:?}", (&fd1, &fd2, &fd3));
    //command.stdin(fd1);
    //command.stdout(fd2);
    //command.stderr(fd3);
    //command.stdout(Stdio::piped());
    //command.stderr(Stdio::piped());
    //command.stdin(Stdio::piped());


    let mut child = master.spawn_pty_async_full(command, true).await?;
    //let mut child = master.spawn_pty_sync_full(command_sync, true).await?;
    //log::info!("child: {:?}", (&child));

    //let mut child = command.spawn()?;
    //let mut stdout = child.stdout.take().expect("child stdout");
    //let mut s = slave.try_clone().await?;
    //let buf = tokio::io::ReadBuf::cap
    let mut buffer = [0; 1];

    //child.wait().await;
    //let mut slave = child.slave.try_clone().await?;
    let ecode = child.wait().await?;

    //loop {
        //tokio::select! {
            ////x = slave.read(&mut buffer[..]) => {
                ////log::info!("read: {:?}", (x, buffer));
            ////}

            //x = child.wait() => {
                //log::info!("child reaped");
                //break;
            //}
        //}
    //}

    Ok(())
}

async fn asdf() -> Result<(), failure::Error> {
    env_logger::init();
    let (app_tx, mut app_rx) = mpsc::channel(10);
    let (process_tx, process_rx) = mpsc::channel(10);
    let mut argv = std::env::args().collect::<Vec<_>>();
    //.split_at(1);
    let mut cmd = argv.split_off(1);
    let args = cmd.split_off(1);
    log::info!("{:?}", (&cmd,&args));

    tokio::spawn(Process::run_pty(cmd.get(0).unwrap().into(), args, app_tx, process_rx));

    //let mut interval = time::interval(time::Duration::from_secs(2));

    //process_tx.send(ServerMessage::Kill).await;
    //process_tx.send(ServerMessage::SIGHUP).await;
    //process_tx.send(ServerMessage::SIGTERM).await;

    process_tx.send(ServerMessage::Data(bytes::Bytes::from("asdf\n"))).await?;
    //process_tx.send(ServerMessage::EOF).await;
    //process_tx.send(ServerMessage::EOF).await;
    process_tx.send(ServerMessage::Data(bytes::Bytes::from("asdf\n"))).await?;

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
    Ok(())
}


