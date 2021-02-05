use super::*;
use crate::bindings::parser::Elem;
use crossbeam::channel;
use log::*;

pub struct InputReader {
    pub q: Vec<Elem>,
    pub quit: bool,
    pub state: ModeState,
    pub tx: channel::Sender<Command>,
    pub rx: channel::Receiver<Command>,
}
impl Default for InputReader {
    fn default() -> Self {
        let (tx, rx) = channel::unbounded();
        Self {
            q: Vec::new(),
            quit: false,
            tx,
            rx,
            state: ModeState::default(),
        }
    }
}
impl InputReader {
    pub fn is_quit(&self) -> bool {
        self.quit
    }

    pub fn add(&mut self, e: Elem) {
        if let Elem::Control('r') = e {
            info!("Refresh");
            self.q.clear();
            self.state.clear();
            self.tx.send(Command::Resume).unwrap();
            return;
        }

        self.q.push(e);
        let result = self.state.command(self.q.as_slice());
        match result {
            Ok((_, commands)) => {
                for c in commands.iter() {
                    info!("Mode Command {:?}", c);
                    match c {
                        Command::Quit => {
                            info!("Quit");
                            self.tx.send(Command::Quit).unwrap();
                            self.quit = true;
                            return;
                        }
                        Command::MacroStart(id) => {
                            self.state.record.replace(*id);
                        }
                        Command::MacroEnd => {
                            self.state.record = None;
                        }
                        Command::Mode(m) => {
                            self.state.macros_add(c.clone());
                            self.state.mode = *m;
                            self.tx.send(Command::Mode(self.state.mode)).unwrap();
                            self.q.clear();
                        }
                        _ => {
                            info!("[{:?}] Ok: {:?}\r", self.state.mode, (&self.q, &c));
                            self.state.macros_add(c.clone());
                            self.q.clear();
                            self.tx.send(c.clone()).unwrap();
                        }
                    }
                }
            }
            Err(nom::Err::Incomplete(e)) => {
                info!("Incomplete: {:?}\r", (&self.q, e));
            }
            Err(e) => {
                info!("Error: {:?}\r", (e, &self.q));
                self.q.clear();
            }
        }
    }
}
