use super::TextBuffer;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct WrapValue {
    pub lc0: usize,
    pub lc1: usize,
    pub c0: usize,
    pub c1: usize,
    pub offset: usize,
    pub wrap0: usize,
    pub wrap1: usize,
    pub line0: usize,
    pub line1: usize,
    pub wraps: usize,
}

impl WrapValue {
    pub fn to_string(&self, buf: &TextBuffer) -> String {
        buf.text.slice(self.c0..self.c1).to_string()
    }
}

impl TextBuffer {
    pub fn get_wrap(&self) -> WrapValue {
        self.char_to_wrap(self.char_start).unwrap()
    }

    pub fn delta_wrap(&self, c: usize, dy: i32) -> WrapValue {
        let mut start = c;
        let mut w = self.char_to_wrap(start).unwrap();

        if dy > 0 {
            let mut count = dy;
            while count > 0 {
                //println!("X: {:?}", (count));
                match self.next_wrap(&w) {
                    Some(x) => {
                        w = x;
                        count -= 1;
                    }
                    _ => break
                }
            }
        }

        if dy < 0 {
            let mut count = (-dy) as usize;
            while count > 0 {
                //println!("Y: {:?}", (count));
                match self.prev_wrap(&w) {
                    Some(x) => {
                        w = x;
                        count -= 1;
                    }
                    _ => break
                }
            }
        }
        w
    }

    pub fn wrap_window_down(&self, c: usize, size: usize) -> Vec<WrapValue> {
        self.wrap_window(c, size, false)
    }

    pub fn wrap_window_up(&self, c: usize, size: usize) -> Vec<WrapValue> {
        self.wrap_window(c, size, true)
    }

    pub fn wrap_window(&self, c: usize, size: usize, reverse: bool) -> Vec<WrapValue> {
        let mut out = Vec::new();
        let ow = self.char_to_wrap(c);

        let r;
        if reverse {
            r = -1;
        } else {
            r = 1;
        }

        if ow.is_some() && size > 0 {
            let mut w = ow.unwrap();
            out.push(w);

            let mut count = 1;
            while out.len() < size {
                let w0 = self.delta_wrap(c, r*count);
                //println!("B: {:?}", (w0, out.len()));
                if w0.c0 == w.c0 {
                    break;
                }
                out.push(w0);
                w = w0;
                count += 1;
            }

            w = ow.unwrap();
            count = 1;
            while out.len() < size {
                let w0 = self.delta_wrap(c, -r*count);
                //println!("A: {:?}", (w0, out.len()));
                if w0.c0 == w.c0 {
                    break;
                }
                out.insert(0,w0);
                w = w0;
                count += 1;
            }
        }
        if reverse {
            out.reverse();
        }
        out
    }

    pub fn line_to_wrap(&self, line: usize) -> Option<WrapValue> {
        let len_lines = self.text.len_lines();
        if line >= len_lines {
            None
        } else {
            let c = self.text.line_to_char(line);
            self.char_to_wrap(c)
        }
    }

    pub fn prev_wrap(&self, w: &WrapValue) -> Option<WrapValue> {
        let vsx = self.view.vsx as usize;
        if w.wrap0 > 0 {
            let c0 = w.lc0 + (w.wrap0-1) * vsx;
            self.char_to_wrap(c0)
        } else if w.line0 > 0 {
            self.char_to_wrap(w.lc0-1)
        } else {
            None
        }
    }

    pub fn next_wrap(&self, w:  &WrapValue) -> Option<WrapValue> {
        let len_chars = self.text.len_chars();
        if w.c1 >= len_chars {
            None
        } else {
            self.char_to_wrap(w.c1)
        }
    }

    pub fn char_to_wrap(&self, c: usize) -> Option<WrapValue> {
        let len_chars = self.text.len_chars();
        if c >= len_chars && len_chars > 0 {
            self.char_to_wrap(len_chars-1)
        } else {
            let vsx = self.view.vsx as usize;
            let line = self.text.char_to_line(c);
            let lc0 = self.text.line_to_char(line);
            let lc1 = self.text.line_to_char(line+1);
            let wrap0 = (c - lc0) / vsx;
            let c0 = lc0 + wrap0 * vsx;
            let mut wrap1 = wrap0 + 1;
            let wraps = (lc1 - lc0) / vsx + 1;
            let c1;
            if wrap1 == wraps {
                c1 = lc1;
                wrap1 = 0;
            } else {
                c1 = c0 + vsx;
            }
            Some(WrapValue {
                lc0: lc0,
                lc1: lc1,
                c0: c0,
                c1: c1,
                offset: c - c0,
                wrap0: wrap0,
                wrap1: wrap1,
                line0: line,
                line1: line+1,
                wraps: wraps,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_buf() -> TextBuffer {
        let mut buf = TextBuffer::from_str(r###"test
line2
0stst estst estst estst estsX
1stst estst estst estst estssX
2stst estst estst estst estsstX
1
2
3
4

"###);
        buf.set_size(20, 12);
        buf
    }

    fn get_empty_buf() -> TextBuffer {
        let mut buf = TextBuffer::from_str("");
        buf.set_size(20, 12);
        buf
    }

    #[test]
    fn test_normalize() {
        let mut buf = get_buf();
        let mut c = 0;
        let len_chars = buf.text.len_chars();
        while let Some(w) = buf.char_to_wrap(c) {
            let w = buf.char_to_wrap(c).unwrap();
            let s = w.to_string(&buf);
            c = w.c1;
            println!("W: {:?}", (s, w.lc1 - w.lc0, w.c1-w.c0, len_chars, c));
            if c >= len_chars {
                break;
            }
        }
    }

    #[test]
    fn test_render() {
        let mut buf = get_buf();
        let mut c = 0;
        let len_chars = buf.text.len_chars();
        let w = buf.char_to_wrap(len_chars-1).unwrap();
        println!("W: {:?}", w);
        buf.char_start = w.c0;
        buf.dump();
        buf.scroll(-20);
        buf.dump();
    }

    #[test]
    fn test_next_prev_wrap() {
        let mut buf = get_buf();
        let mut c = 0;
        let len_chars = buf.text.len_chars();
        let w0 = buf.char_to_wrap(c).unwrap();
        let w1 = buf.next_wrap(&w0).unwrap();
        let w2 = buf.prev_wrap(&w1).unwrap();
        let w3 = buf.prev_wrap(&w2);
        println!("W0: {:?}", w0);
        println!("W1: {:?}", w1);
        println!("W2: {:?}", w2);
        assert_eq!(w0.c0,w2.c0);
        assert_eq!(None,w3);
    }

    #[test]
    fn test_window() {
        let mut buf = get_buf();
        let mut c = 0;
        assert_eq!(0, buf.wrap_window_down(c, 0).len());
        assert_eq!(1, buf.wrap_window_down(c, 1).len());
        assert_eq!(10, buf.wrap_window_down(c, 10).len());
        assert_eq!(buf.view.vsy as usize, buf.wrap_window_down(c, buf.view.vsy as usize).len());
        assert_eq!(15, buf.wrap_window_down(c, 100).len());
        buf.scroll(1);

    }

    #[test]
    fn test_scroll_end() {
        let mut buf = get_buf();
        assert_eq!(buf.view.vsy as usize, buf.wrap_window_down(buf.char_start, buf.view.vsy as usize).len());
        buf.scroll(100);
        assert_eq!(buf.view.vsy as usize, buf.wrap_window_down(buf.char_start, buf.view.vsy as usize).len());
        assert_eq!(buf.view.wraps.len(), buf.view.vsy as usize);
    }

    #[test]
    fn test_wrap_empty() {
        let mut buf = get_empty_buf();
        buf.dump();
        assert_eq!(1, buf.view.wraps.len());
        buf.scroll(1);
        buf.dump();
    }

}

