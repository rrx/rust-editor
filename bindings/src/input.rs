use super::*;
use editor_core::Command;
use log::*;

pub struct InputReader {
    pub q: Vec<Elem>,
    pub quit: bool,
    pub state: ModeState,
    pub history: history::SimpleHistory,
}
impl Default for InputReader {
    fn default() -> Self {
        Self {
            q: Vec::new(),
            quit: false,
            history: history::SimpleHistory::default(),
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

    pub fn add(&mut self, e: Elem) -> Vec<Command> {
        self.q.push(e);
        let result = self.state.command(self.q.as_slice());
        let mut out = vec![];
        match result {
            Ok((_, commands)) => {
                for c in commands.iter() {
                    info!("Mode Command {:?}", c);
                    match c {
                        Command::Quit => {
                            info!("Quit");
                            self.quit = true;
                            return vec![Command::Quit];
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
                                    out.push(cc.clone());
                                });
                            self.q.clear();
                        }
                        Command::Mode(m) => {
                            self.state.macros_add(c.clone());
                            self.state.mode = *m;
                            out.push(Command::Mode(self.state.mode));
                            self.history.add_elem(&c);
                            self.q.clear();
                        }
                        _ => {
                            info!("[{:?}] Ok: {:?}\r", self.state.mode, (&self.q, &c));
                            self.state.macros_add(c.clone());
                            self.history.add_elem(&c);
                            self.q.clear();
                            out.push(c.clone());
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
        out
    }
}
