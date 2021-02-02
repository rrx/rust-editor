use log::*;
use super::*;
use std::collections::VecDeque;

#[derive(Debug)]
pub struct RotatingList<T> {
    pub elements: VecDeque<T>
}

pub type BufferList = RotatingList<Buffer>;

impl<T> Default for RotatingList<T> {
    fn default() -> Self {
        Self { elements: VecDeque::new() }
    }
}

impl<T> RotatingList<T> where T: std::fmt::Debug {
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

impl BufferList {
    pub fn resize(&mut self, w: usize, h: usize, x0: usize, y0: usize) {
        // each buffer needs to be resized on resize event
        // because each one caches things that depend on the size
        self.elements.iter_mut().for_each(|b| {
            b.resize(w, h, x0, y0);
        });
    }

    pub fn command(&mut self, c: &Command) {
        match c {
            Command::BufferNext => {
                self.next();
                self.get_mut().update_view();
                info!("Next: {}", self.get().path);
            }
            Command::BufferPrev => {
                self.prev();
                self.get_mut().update_view();
                info!("Prev: {}", self.get().path);
            }
            _ => {
                self.get_mut().command(c);
            }
        }
    }
}


