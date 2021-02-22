use super::*;
use log::*;
use parking_lot::RwLock;
use ropey::Rope;
use std::fs::File;
use std::io;
use std::sync::Arc;
use std::collections::VecDeque;


#[derive(Debug)]
pub struct UndoList {
    ahead: VecDeque<Rope>,
    behind: VecDeque<Rope>
}

impl Default for UndoList {
    fn default() -> Self {
        Self { ahead: VecDeque::new(), behind: VecDeque::new() }
    }
}

impl UndoList {
    fn redo(&mut self, text: Rope) -> Option<Rope> {
        info!("redo:{:?}", (self.behind.len(), self.ahead.len()));
        let maybe_v = self.ahead.pop_front();
        match maybe_v {
            Some(v) => {
                self.behind.push_front(text);
                Some(v)
            }
            None => None
        }
    }

    fn undo(&mut self, text: Rope) -> Option<Rope> {
        info!("undo:{:?}", (self.behind.len(), self.ahead.len()));
        let maybe_v = self.behind.pop_front();
        match maybe_v {
            Some(v) => {
                self.ahead.push_front(text);
                Some(v)
            }
            None => None
        }
    }

    // push state before change
    fn push(&mut self, text: Rope) -> &mut Self {
        self.ahead.truncate(0);
        self.behind.push_front(text);
        self
    }
}


#[derive(Debug)]
pub struct FileBuffer {
    pub text: Rope,
    pub path: String,
    pub config: BufferConfig,
    version: u64,
    history: UndoList
}

#[derive(Debug, Clone)]
pub struct Buffer {
    buf: LockedFileBuffer
}

impl Buffer {
    pub fn from_path(path: &String) -> Self {
        let maybe_f = File::open(&path.clone());

        let text = match maybe_f {
            Ok(f) => Rope::from_reader(&mut io::BufReader::new(f)).unwrap(),
            Err(_) => Rope::from_str(""),
        };

        let config = BufferConfig::config_for(Some(path));
        info!("Add window: {:?}", config);
        Self { 
            buf: Arc::new(RwLock::new(FileBuffer {
                path: path.clone(),
                text,
                config,
                version: 0,
                history: UndoList::default()
            }))
        }
    }

    pub fn from_string(s: &String) -> Self {
        let text = Rope::from_str(s);
        Buffer { 
            buf: Arc::new(RwLock::new(FileBuffer {
                path: "".into(),
                config: BufferConfig::config_for(None),
                text,
                version: 0,
                history: UndoList::default()
            }))
        }
    }

    pub fn get_config(&self) -> BufferConfig {
        self.buf.read().config.clone()
    }

    pub fn get_text(&self) -> Rope {
        self.buf.read().text.clone()
    }

    pub fn get_path(&self) -> String {
        self.buf.read().path.clone()
    }

    pub fn replace_text(&mut self, s: &str) -> &mut Self {
        let mut fb = self.buf.write();
        let u = fb.text.clone();
        let end = fb.text.len_chars();
        fb.text.remove(0..end);
        fb.text.append(Rope::from_str(s));
        fb.history.push(u);
        drop(fb);
        self
    }

    pub fn set_path(&mut self, s: &str) -> &mut Self {
        let mut fb = self.buf.write();
        fb.path = String::from(s);
        drop(fb);
        self
    }

    pub fn remove_range(&mut self, start: usize, end: usize) -> &mut Self {
        let mut fb = self.buf.write();
        let u = fb.text.clone();
        let length = fb.text.len_chars();
        let mut end0 = end;
        if end0 > length {
            end0 = length;
        }

        if start < end0 {
            fb.text.remove(start..end0);
            fb.history.push(u);
        }
        drop(fb);
        self
    }

    pub fn insert_char(&mut self, c: usize, ch: char) -> usize {
        let mut fb = self.buf.write();
        let u = fb.text.clone();
        let s = match ch {
            '\t' => fb.config.indent(),
            '\n' => fb.config.line_sep().to_string(),
            _ => ch.to_string(),
        };
        fb.text.insert(c, &s);
        info!("insert: {:?}", (c, &s));
        fb.history.push(u);
        drop(fb);
        s.len()
    }

    pub fn insert_string(&mut self, c: usize, s: &str) -> usize {
        let mut fb = self.buf.write();
        let u = fb.text.clone();
        //let s = match ch {
            //'\t' => fb.config.indent(),
            //'\n' => fb.config.line_sep().to_string(),
            //_ => ch.to_string(),
        //};
        fb.text.insert(c, &s);
        info!("insert: {:?}", (c, &s));
        fb.history.push(u);
        drop(fb);
        s.len()
    }

    pub fn remove_char(&mut self, c: usize) -> &mut Self {
        if c > 0 {
            let mut fb = self.buf.write();
            let u = fb.text.clone();
            fb.text.remove(c - 1..c);
            info!("remove: {:?}", (c-1, c));
            fb.history.push(u);
            drop(fb);
        }
        self
    }

    // remove trailing newlines, to join with the next line
    pub fn join_line(&mut self, line_inx: usize) -> &mut Self {
        let mut fb = self.buf.write();
        let u = fb.text.clone();
        let line = fb.text.line(line_inx).to_string();
        let remove = if line.ends_with("\r\n") {
            2
        } else if line.ends_with("\n") {
            1
        } else {
            0
        };

        let lc0 = fb.text.line_to_char(line_inx);
        let end = lc0 + line.len();
        let start = end - remove;

        if remove > 0 {
            fb.text.remove(start..end);
            fb.history.push(u);
        }
        drop(fb);
        self
    }

    pub fn delete_line_range(&mut self, start_inx: usize, end_inx: usize) -> &mut Self {
        let mut fb = self.buf.write();
        let u = fb.text.clone();
        let c0 = fb.text.line_to_char(start_inx);
        let c1 = fb.text.line_to_char(end_inx);
        if c1 > c0 {
            fb.text.remove(c0..c1);
            fb.history.push(u);
        }
        drop(fb);
        self
    }

    pub fn undo(&mut self) -> &mut Self {
        let mut fb = self.buf.write();
        let save = fb.text.clone();
        if let Some(text) = fb.history.undo(save) {
            fb.text = text;
        }
        drop(fb);
        self
    }

    pub fn redo(&mut self) -> &mut Self {
        let mut fb = self.buf.write();
        let save = fb.text.clone();
        if let Some(text) = fb.history.redo(save) {
            fb.text = text;
        }
        drop(fb);
        self
    }
}

pub type LockedFileBuffer = Arc<RwLock<FileBuffer>>;



