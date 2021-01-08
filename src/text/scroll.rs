use super::TextBuffer;
use std::cmp::{max, Ordering};

impl TextBuffer {
    pub fn scroll(&mut self, y: i32) {
        //println!("x: {:?}", (y));
        let w = self.delta_wrap(self.char_start, y);
        //println!("Y: {:?}", (w));
        self.update_window(w.c0);
    }

    pub fn scroll_line(&mut self, line: i64) {
        // 0 is the start
        // negative lines is the number of lines from the end of the file
        let lines: usize = self.text.len_lines() - 1;
        let current: usize;
        let mut offset: usize;
        if line < 0 {
            current = lines - i64::abs(line) as usize;
        } else {
            current = line as usize;
        }

        let w = self.line_to_wrap(current).unwrap();
        self.update_window(w.c0);
    }

    pub fn update_window(&mut self, c: usize) {
        let mut start = self.char_start;
        let mut end = self.char_end;
        let vsy = self.view.vsy as usize;

        //if self.text.len_chars() == 0 {
            //self.char_start = 0;
            //self.char_end = 0;
            //self.char_current = 0;
            //self.set_cursor(0,0);
            ////return;
        //}

        if c >= end {
            start = c;
            end = c;
            self.view.wraps = self.wrap_window_up(c, vsy);
        } else {
            start = c;
            end = c;
            self.view.wraps = self.wrap_window_down(c, vsy);
        }

        if self.view.wraps.len() > 0 {
            start = self.view.wraps[0].c0;
            end = self.view.wraps[self.view.wraps.len()-1].c1;
        }
        self.char_start = start;
        self.char_end = end;
        self.char_current = c;
        assert!(c >= start);
        //assert!(c < end);

        let inx = self.view.wraps.iter().position(|&w| {
            w.c0 == w.c1 || (w.c0 <= c && c < w.c1)
        }).unwrap();

        //let inx = self.view.wraps.binary_search_by(|w| {
            //if c < w.c0 {
                //Ordering::Less
            //} else if c >= w.c1 {
                //Ordering::Greater
            //} else {
                //Ordering::Equal
            //}
        //}).unwrap();
        let w = self.view.wraps[inx];
        //let w = self.char_to_wrap(c).unwrap();
        let cx = w.offset as u16;
        let cy = inx as u16;
        self.set_cursor(cx,cy);
        //if (c as usize) < self.char_start {
            //self.update_window(c as usize);
        //} else if c as usize >= self.char_end {
            //self.update_window(c as usize);
        //}
        //
        //let wraps = self.wrap_window_down(c, self.view.vsy as usize);
        //let mut c0 = c;
        //let mut c1 = c;
        //if wraps.len() > 0 {
            //c0 = wraps[0].c0;
            //c1 = wraps[wraps.len()-1].c1;
        //}
        //self.view.wraps = wraps;
        //self.char_start = c0;
        //self.char_end = c1;
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
        buf.dump();
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
