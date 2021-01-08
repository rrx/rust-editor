use super::TextBuffer;
use std::cmp::{min,max};

impl TextBuffer {
    pub fn scroll(&mut self, y: i32) {
        let w = self.delta_wrap(y);
        self.update_window(w.c0);
    }

    pub fn update_window(&mut self, c: usize) {
        let wraps = self.wrap_window(c, self.view.vsy as usize);
        println!("{:?}", (c, &wraps));
        let mut c0 = c;
        let mut c1 = c;
        if wraps.len() > 0 {
            c0 = wraps[0].c0;
            c1 = wraps[wraps.len()-1].c1;
        }
        self.view.wraps = wraps;
        self.char_start = c0;
        self.char_end = c1;
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
        buf.set_cursor(0,0);
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
