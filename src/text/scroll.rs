use super::TextBuffer;
use std::cmp::{max, Ordering};

impl TextBuffer {
    pub fn insert(&mut self, ch: char) {
        //let vsy = self.view.vsy as usize;
        let mut c = self.char_current;
        let w0 = self.char_to_wrap(c).unwrap();
        self.text.insert_char(c, ch);
        c += 1;
        self.char_current = c;
        let w1 = self.char_to_wrap(c).unwrap();
        self.window_reset(c);
        if w1.wraps != w0.wraps || w1.lc0 != w0.lc0 {
        } else {
            for (i, w) in self.view.wraps.iter_mut().enumerate() {
                w.dirty = i == self.view.wrap_current as usize;
            }
        }
        //} else {
            //self.view.wraps.remove(self.view.wrap_current as usize);
            //self.view.wraps.insert(self.view.wrap_current as usize, w1);
            //self._update_cursor(c);
        //}

    }

    pub fn scroll(&mut self, y: i32) {
        let vsy = self.view.vsy as usize;
        let w = self.delta_wrap(self.char_start, y);
        if w.c0 != self.char_start {
            self.char_start = w.c0;
            self.view.wraps = self.wrap_window_down(w.c0, vsy);
            self.char_start = self.view.wraps[0].c0;
            self.char_end = self.view.wraps[self.view.wraps.len()-1].c1;

            let mut c = self.char_current;
            let start = w.c0;
            if c < start {
                c = start;
            } else if c >= self.char_end {
                let w = self.view.wraps[self.view.wraps.len()-1];
                c = w.c0;
            }
            self._update_cursor(c);
        }
    }

    pub fn scroll_line(&mut self, line: i64) {
        // 0 is the start
        // negative lines is the number of lines from the end of the file
        let lines: usize = self.text.len_lines() - 1;
        let current: usize;
        if line < 0 {
            current = lines - i64::abs(line) as usize;
        } else {
            current = line as usize;
        }

        let w = self.line_to_wrap(current).unwrap();
        self.update_window(w.c0);
    }

    pub fn update_line_current(&mut self, c: usize) {
        let mut wrap = self.view.wraps.get_mut(self.view.wrap_current as usize).unwrap();
        wrap.dirty = true;
        //let (cx, cy) = self.cursor_from_char(wrap.lc0);
    }

    pub fn update_window(&mut self, c: usize) {
        let oob = c < self.char_start || c >= self.char_end;
        if oob {
            self.window_reset(c);
        }

        self._update_cursor(c);
    }

    pub fn window_reset(&mut self, c: usize) {
        let mut start = self.char_start;
        let mut end = self.char_end;
        let vsy = self.view.vsy as usize;

        // handle viewport change
        if c >= end {
            self.view.wraps = self.wrap_window_up(c, vsy);
        } else if c < start {
            self.view.wraps = self.wrap_window_down(c, vsy);
        } else {
            self.view.wraps = self.wrap_window_down(self.char_start, vsy);
        }

        start = self.view.wraps[0].c0;
        end = self.view.wraps[self.view.wraps.len()-1].c1;
        self.char_start = start;
        self.char_end = end;
        self._update_cursor(c);
    }

    pub fn cursor_from_char(&self, c: usize) -> (u16, u16) {
        // find and set cursor
        let inx = self.view.wraps.iter().position(|&w| {
            w.c0 == w.c1 || (w.c0 <= c && c < w.c1)
        }).unwrap();
        let w = self.view.wraps[inx];
        let cx = (c - w.c0) as u16;
        let cy = inx as u16;
        (cx, cy)
    }

    pub fn _update_cursor(&mut self, c: usize) {
        self.char_current = c;

        self.view.wrap_current = self.wrap_index_from_char(c);

        // find and set cursor
        let (cx, cy) = self.cursor_from_char(c);
        self.set_cursor(cx as u16,cy);
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_buf() -> TextBuffer {
        let mut buf = TextBuffer::from_str(r###"test
line2
estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst
asdf
1
2
3
4
"###);
        buf.set_size(20, 12);
        buf
    }

    fn dump(buf: &mut TextBuffer) {
        buf.dump();
    }

    #[test]
    fn test_scroll_up() {
        let mut buf = get_buf();
        buf.dump();
        assert_eq!(0, buf.char_start);
        buf.scroll(1);
        //buf.dump();
        assert_eq!(5, buf.char_start);
        buf.scroll(1);
        assert_eq!(11, buf.char_start);
        buf.scroll(1);
        assert_eq!(11+buf.view.vsx as usize, buf.char_start);
        buf.scroll(1);
        buf.dump();
        assert_eq!(11+2*buf.view.vsx as usize, buf.char_start);
        buf.scroll(-1);
        assert_eq!(11+buf.view.vsx as usize, buf.char_start);
        buf.scroll(-1);
        assert_eq!(11, buf.char_start);

        // scroll back to the top
        buf.scroll(-20);
        buf.dump();
        assert_eq!(0, buf.char_start);

        // scroll to the end
        buf.scroll(20);
        //assert_eq!(11+buf.view.vsx as usize, buf.char_start);
    }

    #[test]
    fn test_scroll_down() {
        let mut buf = get_buf();
        dump(&mut buf);
        buf.scroll(1);
        dump(&mut buf);
        buf.scroll(1);
        dump(&mut buf);
        buf.scroll(1);
        dump(&mut buf);
        buf.scroll(1);
        dump(&mut buf);
    }
}
