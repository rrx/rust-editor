use super::*;
use log::*;
use parking_lot::RwLock;
use ropey::Rope;
use std::collections::VecDeque;
use std::convert::From;
use std::fs::File;
use std::io;
use std::sync::Arc;

#[derive(Debug)]
pub struct UndoList {
    ahead: VecDeque<Rope>,
    behind: VecDeque<Rope>,
}

impl Default for UndoList {
    fn default() -> Self {
        Self {
            ahead: VecDeque::new(),
            behind: VecDeque::new(),
        }
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
            None => None,
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
            None => None,
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
    history: UndoList,
}

#[derive(Debug, Clone)]
pub struct Buffer {
    buf: LockedFileBuffer,
}

#[derive(Debug)]
pub enum BufferError {
    FileNotFound,
    InvalidUnicode,
}

impl From<std::io::Error> for BufferError {
    fn from(_: std::io::Error) -> Self {
        BufferError::FileNotFound
    }
}

impl Buffer {
    pub fn from_path_or_empty(path: &String) -> Self {
        match Self::from_path(path) {
            Ok(b) => b,
            Err(_) => Self::from_string(&"".to_string()),
        }
    }

    pub fn from_path(path: &String) -> Result<Self, BufferError> {
        let f = File::open(&path.clone())?;
        let text = Rope::from_reader(&mut io::BufReader::new(f))?;
        let config = BufferConfig::config_for(Some(path));
        info!("Add window: {:?}", config);
        Ok(Self {
            buf: Arc::new(RwLock::new(FileBuffer {
                path: path.clone(),
                text,
                config,
                version: 0,
                history: UndoList::default(),
            })),
        })
    }

    pub fn from_string(s: &String) -> Self {
        let text = Rope::from_str(s);
        Buffer {
            buf: Arc::new(RwLock::new(FileBuffer {
                path: "".into(),
                config: BufferConfig::config_for(None),
                text,
                version: 0,
                history: UndoList::default(),
            })),
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

    pub fn replace_buffer(&mut self, s: &str) -> &mut Self {
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

    pub fn insert_string(&mut self, c: usize, s: &str) -> usize {
        let mut fb = self.buf.write();
        let u = fb.text.clone();
        let out: String = s
            .chars()
            .map(|x| match x {
                '\t' => fb.config.indent(),
                '\n' => fb.config.line_sep().to_string(),
                _ => x.to_string(),
            })
            .collect::<Vec<String>>()
            .join("");
        fb.text.insert(c, &out);
        info!("insert: {:?}", (c, &out));
        fb.history.push(u);
        drop(fb);
        s.len()
    }

    pub fn remove_char(&mut self, c: usize) -> &mut Self {
        if c > 0 {
            let mut fb = self.buf.write();
            let u = fb.text.clone();
            fb.text.remove(c - 1..c);
            info!("remove: {:?}", (c - 1, c));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_utf8() {
        let mut fb = Buffer::from_string(&"地球".to_string());
        {
            let text = fb.get_text();
            let n_chars = text.len_chars();
            let n_bytes = text.len_bytes();
            assert_eq!(n_chars, 2);
            assert_eq!(n_bytes, 6);
            println!("{:?}", (&fb, n_chars, n_bytes));
        }

        {
            fb.remove_range(0, 1);
            let text = fb.get_text();
            let n_chars = text.len_chars();
            let n_bytes = text.len_bytes();
            assert_eq!(n_chars, 1);
            assert_eq!(n_bytes, 3);
            println!("{:?}", (&fb, n_chars, n_bytes));
        }

        fb.remove_range(0, 1);
        println!("{:?}", fb);
    }
}
