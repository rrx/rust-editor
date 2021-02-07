use log::*;
use std::collections::HashMap;

#[derive(Eq, Hash, PartialEq, Debug, Copy, Clone)]
pub struct Register(pub char);

pub struct Registers {
    regs: HashMap<Register, String>,
}
impl Default for Registers {
    fn default() -> Self {
        Self {
            regs: HashMap::new(),
        }
    }
}
impl Registers {
    pub fn update(&mut self, r: &Register, s: &String) {
        info!("Reg[{:?}] = {}", r, &s);
        self.regs.insert(*r, s.clone());
    }
    pub fn get(&self, r: &Register) -> String {
        self.regs.get(r).unwrap_or(&String::from("")).clone()
    }
}
