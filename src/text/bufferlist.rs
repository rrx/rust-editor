use log::*;
use super::*;
use std::collections::VecDeque;

#[derive(Debug)]
pub struct BufferList {
    buffers: VecDeque<Buffer>
}

impl Default for BufferList {
    fn default() -> Self {
        Self { buffers: VecDeque::new() }
    }
}

impl BufferList {
    pub fn get_mut(&mut self) -> &mut Buffer {
        self.buffers.iter_mut().next().unwrap()
    }

    pub fn get(&mut self) -> &Buffer {
        self.buffers.iter().next().unwrap()
    }
    pub fn add(&mut self, b: Buffer) {
        info!("Adding {:?}", b);
        self.buffers.push_front(b);
    }

    pub fn next(&mut self) {
        if let Some(b) = self.buffers.pop_front() {
            self.buffers.push_back(b);
        }
    }

    pub fn prev(&mut self) {
        if let Some(b) = self.buffers.pop_back() {
            self.buffers.push_front(b);
        }
    }

    pub fn resize(&mut self, w: usize, h: usize, x0: usize, y0: usize) {
        // each buffer needs to be resized on resize event
        // because each one caches things that depend on the size
        self.buffers.iter_mut().for_each(|b| {
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


