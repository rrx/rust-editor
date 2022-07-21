use std::collections::VecDeque;

#[derive(Debug)]
pub struct RotatingList<T> {
    pub elements: VecDeque<T>,
}

impl<T> RotatingList<T>
where
    T: std::fmt::Debug,
{
    pub fn new(t: T) -> Self {
        Self {
            elements: VecDeque::from(vec![t]),
        }
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.elements.iter_mut().next().unwrap()
    }

    pub fn get(&mut self) -> &T {
        self.elements.iter().next().unwrap()
    }

    pub fn add(&mut self, b: T) -> &mut Self {
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
