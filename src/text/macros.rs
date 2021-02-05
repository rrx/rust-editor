use super::*;
use log::*;
use std::collections::HashMap;

#[derive(Eq, Hash, PartialEq, Debug, Copy, Clone)]
pub struct MacroId(pub char);

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct Macros {
    h: HashMap<MacroId, Vec<Command>>,
}

impl Default for Macros {
    fn default() -> Self {
        Self { h: HashMap::new() }
    }
}

impl Macros {
    pub fn clear_all(&mut self) {
        self.h.clear();
    }

    pub fn add(&mut self, id: &MacroId, c: &Command) {
        match self.h.get_mut(id) {
            Some(v) => {
                v.push(c.clone());
            }
            None => {
                self.h.insert(*id, vec![]);
            }
        }
        info!("Macro[{:?}] = {:?}", id, self.h.get(id));
    }

    pub fn clear(&mut self, id: &MacroId) {
        self.h.remove(id);
    }

    pub fn get(&self, id: &MacroId) -> Vec<Command> {
        self.h.get(id).unwrap_or(&vec![]).clone()
    }
}
