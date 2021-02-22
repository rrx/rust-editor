use super::*;
use crate::bindings::parser::Elem;
use crossbeam::channel;
use log::*;
use std::collections::VecDeque;

/// Store Change History
/// Keep track of changes, so we can repeat them later
pub struct SimpleHistory {
    h: VecDeque<Vec<Command>>,
    acc: Vec<Command>,
    size: usize,
    record: bool,
}

impl Default for SimpleHistory {
    fn default() -> Self {
        Self {
            h: VecDeque::new(),
            size: 10,
            acc: vec![],
            record: false,
        }
    }
}

impl SimpleHistory {
    pub fn add(&mut self, v: Vec<Command>) {
        self.h.push_front(v);
        if self.h.len() > self.size {
            self.h.pop_back().unwrap();
        }
    }

    pub fn front(&self) -> Option<Vec<Command>> {
        self.h.front().cloned()
    }

    pub fn add_elem(&mut self, e: &Command) {
        if self.record {
            self.acc.push(e.clone());
        }
    }

    pub fn add_acc(&mut self, es: &[Command]) {
        if self.record {
            info!("History: inc {:?}", es);
            self.acc.append(&mut es.to_vec());
        }
    }

    pub fn change_end(&mut self) {
        if self.acc.len() > 0 {
            info!("History: add {:?}", &self.acc);
            self.add(self.acc.clone());
            self.acc.truncate(0);
        }
        self.record = false;
    }

    pub fn change_start(&mut self) {
        self.record = true;
    }
}

pub struct InputReader {
    pub q: Vec<Elem>,
    pub quit: bool,
    pub state: ModeState,
    pub history: SimpleHistory,
    pub tx: channel::Sender<Command>,
    pub rx: channel::Receiver<Command>,
}
impl Default for InputReader {
    fn default() -> Self {
        let (tx, rx) = channel::unbounded();
        Self {
            q: Vec::new(),
            quit: false,
            history: SimpleHistory::default(),
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

    pub fn reset(&mut self) {
        self.q.clear();
        self.state.clear();
    }

    pub fn add(&mut self, e: Elem) {
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
                        Command::Reset => {
                            self.reset();
                        }
                        Command::MacroStart(id) => {
                            self.state.record.replace(*id);
                        }
                        Command::MacroEnd => {
                            self.state.record = None;
                        }
                        Command::ChangeStart => {
                            self.history.change_start();
                        }
                        Command::ChangeEnd => {
                            self.history.change_end();
                        }
                        Command::ChangeRepeat => {
                            info!("Repeat: {:?}", self.history.front());
                            self.history
                                .front()
                                .unwrap_or(vec![])
                                .iter()
                                .for_each(|cc| {
                                    self.tx.send(cc.clone()).unwrap();
                                });
                            self.q.clear();
                        }
                        Command::Mode(m) => {
                            self.state.macros_add(c.clone());
                            self.state.mode = *m;
                            self.tx.send(Command::Mode(self.state.mode)).unwrap();
                            self.history.add_elem(c);
                            self.q.clear();
                        }
                        _ => {
                            info!("[{:?}] Ok: {:?}\r", self.state.mode, (&self.q, &c));
                            self.state.macros_add(c.clone());
                            self.history.add_elem(c);
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
