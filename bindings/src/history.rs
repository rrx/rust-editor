use editor_core::Command;
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


