use log::*;
use std::collections::HashMap;

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct Variable(pub String);

pub struct Variables {
    vars: HashMap<Variable, String>,
}
impl Default for Variables {
    fn default() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }
}
impl Variables {
    pub fn update(&mut self, r: &Variable, s: &String) {
        info!("Var[{:?}] = {}", r, &s);
        self.vars.insert(r.clone(), s.clone());
    }
    pub fn get(&self, r: &Variable) -> String {
        self.vars.get(r).unwrap_or(&String::from("")).clone()
    }
}
