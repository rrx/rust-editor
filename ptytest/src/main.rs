#![allow(warnings)]

use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use anyhow::anyhow;
//use futures::prelude::*;
use crossbeam::channel::{Receiver, Sender, tick, select, unbounded};
use crossbeam::thread;
use std::time::{Duration, Instant};
use std::ffi::OsString;
use popol::{Events, Sources};
use std::io::prelude::*;

enum ReadResult {
    Data(usize),
    Status(portable_pty::ExitStatus),
    Unknown
}

#[derive(Debug)]
enum Command {
    Start(Vec<OsString>),
    Quit
}

#[derive(Eq, PartialEq, Clone, Debug)]
enum Source {
    /// An event from a connected peer.
    Process(String),
    /// An event on the listening socket. Most probably a new peer connection.
    Input,
}


fn input_loop(tx: Sender<Command>) {
    let mut stdin = std::io::stdin();
    let mut buf = [0; 1024];
    let mut done = false;
    while(!done) {
        match stdin.read(&mut buf[..]) {
            Ok(n) => {
                println!("X: {:?}", (&buf[..n]));
            }
            Err(err) => {
                done = true;
            }
        }
    }
}

fn process_loop(tx: Sender<Command>, rx: Receiver<Command>) {
    let pty_system = NativePtySystem::default();
    while let Ok(c) = rx.recv() {
        println!("Y: {:?}", (c));
        match c {
            Command::Start(args) => {
                println!("{:?}", args);
                //items.push(reader);
                //drop(pair.slave);
            }
            Command::Quit => break,
            _ => break
        }

    }
}

fn command_loop(tx: Sender<Command>) -> anyhow::Result<()> {
    let pty_system = NativePtySystem::default();
    let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;
    let cmd = CommandBuilder::new("ls");
    let slave = pair.slave;
    let mut child = slave.spawn_command(cmd)?;
    let mut reader = pair.master.try_clone_reader()?;
    drop(pair.master);
    let mut buffer = String::new();
    println!("s");
    println!("child status: {:?}", child.wait().unwrap());
    let result = reader.read_to_string(&mut buffer)?;
    println!("C: {:?}", (result, &buffer));
    for c in buffer.escape_debug() {
        print!("{}", c);
    }
    Ok(())
}

fn main_loop(tx: Sender<Command>, rx: Receiver<Command>) -> anyhow::Result<()> {
    let pty_system = NativePtySystem::default();
    let mut sources = Sources::new();
    let mut events = Events::new();


    //let stdin = std::io::stdin();
    //sources.register(Source::Input, &stdin, popol::interest::READ);
    let mut done = false;
    while(!done) {
        sources.wait(&mut events).unwrap();
        for (key, event) in events.iter() {
            if event.errored {
                println!("ERROR: {:?}", (event));
                done = true;
                break;
            }
            match key {
                Source::Process(s) => {
                    println!("PROCESS: {:?}", (s));
                }
                Source::Input => {
                    let mut buf = [0; 1024];
                    match std::io::stdin().read(&mut buf[..]) {
                        Ok(n) => {
                            println!("INPUT: {:?}", (key, &buf[..n]));
                        }
                        Err(err) => {
                            done = true;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

//fn main_loop2(tx: Sender<Command>, rx: Receiver<Command>) {
//let pty_system = NativePtySystem::default();
//let ticker = tick(Duration::from_millis(100));    
//let mut items: Vec<Box<dyn std::io::Read + Send>> = vec![];
//loop {
//select! {
//recv(ticker) -> t => {
//println!("tick");
//items.iter_mut().for_each(|i| {
//let mut buffer = [0; 32];
//match i.read(&mut buffer) {
//Ok(n) => {
//if n > 0 {
//println!("X{:?}", (n, &buffer[..n]));
//}
//}
//_ => ()
//}
//});
//}

//recv(rx) -> c => {
//println!("C: {:?}", c);
//match c {
//Ok(Command::Start(args)) => {
//println!("args: {:?}", args);
//let pair = pty_system
//.openpty(PtySize {
//rows: 24,
//cols: 80,
//pixel_width: 0,
//pixel_height: 0,
//})
//.unwrap();
//let cmd = CommandBuilder::from_argv(args);
//let slave = pair.slave;
//let mut child = slave.spawn_command(cmd);
//let mut reader = pair.master.try_clone_reader().unwrap();
//drop(pair.master);
//items.push(reader);
////drop(pair.slave);
//}
//Ok(Command::Quit) => break,
//_ => break
//}
//}
//}
//}
//}

//#[derive(Clone)]
struct Process {
    h: usize,
    args: Vec<OsString>,
    //pty: Box<portable_pty::Child>//Box<portable_pty::MasterPty>
    //reader: Box<dyn std::io::Read + Send>,
    //writer: Box<dyn std::io::Write + Send>
}

impl Process {
    fn new(h: usize, args: Vec<OsString>) -> Self {
        let pty_system = NativePtySystem::default();
        println!("P:{:?}", h);
        println!("args: {:?}", args);
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
        .unwrap();
        let cmd = CommandBuilder::from_argv(args.clone());
        let slave = pair.slave;
        let mut child = slave.spawn_command(cmd);
        let reader = pair.master.try_clone_reader().unwrap();
        //let writer = pair.master.try_clone_writer().unwrap();
        Self { h, args } //, reader }//, pty: child }//reader, writer }
    }
}

fn process_thread(p: Process) {
    let mut buf = String::new();
    println!("{:?}", (p.h, p.args));
    //loop {
        //match p.reader.read_to_string(&mut buf) {
            //Ok(n) => {
                //println!("X: {:?}", (&p.h, &p.args, &buf));
            //}
            //Err(e) => {
                //break;
            //}
        //}
    //}
}

fn main() -> anyhow::Result<()> {
    let (tx, rx) = unbounded();
    command_loop(tx.clone())?;
    Ok(())
}

fn main2() -> anyhow::Result<()> {
    let (tx, rx) = unbounded();
    command_loop(tx.clone())?;
    
    //let (p_tx, p_rx) = unbounded();
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    //p_tx.send(Process::new(0, args.clone()))?;
    //p_tx.send(Process::new(1, args))?;

    thread::scope(|s| {
        // display
        s.spawn(|_| main_loop(tx.clone(), rx.clone()));
        s.spawn(|_| {
            let result = process_loop(tx.clone(), rx.clone());
            println!("process: {:?}", result);
        });
        s.spawn(|_| input_loop(tx.clone()));
        s.spawn(|_| {
            println!("asdf");
            let result = command_loop(tx.clone());
            println!("command: {:?}", result);
        });
        //loop {
            //match p_rx.recv() {
                //Ok(p) => {
                    //let p = p.clone();
                    //s.spawn(|_| process_thread(p));
                //}
                //Err(e) => {
                    //break;
                //}
            //}
        //}
    }).unwrap();
    println!("end");
    Ok(())
}

//fn main2() -> anyhow::Result<()> {
//let (s, ctrl_c) = async_channel::bounded(100);
//let handle = move || {
//s.try_send(()).ok();
//};
//ctrlc::set_handler(handle).unwrap();

//smol::block_on(async {
//let pty_system = NativePtySystem::default();

//let pair = pty_system
//.openpty(PtySize {
//rows: 24,
//cols: 80,
//pixel_width: 0,
//pixel_height: 0,
//})
//.unwrap();

//let args = std::env::args_os().skip(1).collect();
//println!("args: {:?}", args);
//let cmd = CommandBuilder::from_argv(args);
//let slave = pair.slave;
//let mut child = smol::unblock(move || slave.spawn_command(cmd)).await?;
//let mut reader = pair.master.try_clone_reader().unwrap();
//drop(pair.master);


//let mut r = smol::Unblock::new(reader);
//let mut stdout = smol::Unblock::new(std::io::stdout());
//let mut buffer = [0; 32];

//let mut done = false;
//loop {
//match r.read(&mut buffer).await {
//Ok(n) => {
//if n > 0 {
//println!("X{:?}", (n, &buffer[..n]));
//} else if done {
//println!("done");
//break;
//}
//}
//_ => break
//}


//match child.try_wait() {
//Ok(Some(code)) => {
//println!("Exit: {:?}", code);
//done = true;
//}
//Ok(None) => {
//// continue
//}
//_ => {
//println!("Error");
//break;
//}
//}

//match ctrl_c.try_recv() {
//Ok(_) => {
//done = true;
//}
//Err(async_channel::TryRecvError::Empty) => {
//}
//_ => {
//println!("Error");
//break;
//}
//}
//}
//Ok(())
//})
//}
