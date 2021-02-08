

use std::collections::VecDeque;

#[derive(Debug)]
pub struct RotatingList<T> {
    pub elements: VecDeque<T>,
}

impl<T> Default for RotatingList<T> {
    fn default() -> Self {
        Self {
            elements: VecDeque::new(),
        }
    }
}

impl<T> RotatingList<T>
where
    T: std::fmt::Debug,
{
    pub fn get_mut(&mut self) -> &mut T {
        self.elements.iter_mut().next().unwrap()
    }

    pub fn get(&mut self) -> &T {
        self.elements.iter().next().unwrap()
    }

    pub fn add(&mut self, b: T) -> &mut Self {
        //info!("Adding {:?}", &b);
        self.elements.push_front(b);
        self
    }

    pub fn next(&mut self) -> &mut Self {
        if let Some(b) = self.elements.pop_front() {
            self.elements.push_back(b);
        }
        self
    }

    pub fn prev(&mut self) -> &mut Self {
        if let Some(b) = self.elements.pop_back() {
            self.elements.push_front(b);
        }
        self
    }
}
