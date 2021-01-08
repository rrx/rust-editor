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

    // return char index on the next lowest wrap boundary
    pub fn normalize_c(&self, c: usize) -> (usize, usize, usize, usize) {
        let vsx = self.view.vsx as usize;
        let line = self.text.char_to_line(c);
        let lc = self.text.line_to_char(line);
        let wrap = (c - lc) / vsx;
        let cn = lc + wrap * vsx;
        //println!("c: {:?}", (c, cn, lc, line, wrap));
        return (cn, line, wrap, c - cn);
    }

    // y=-1, is the last wrap
    // y = 0, is the first wrap, or normalized c
    // return wraps relative to c
    pub fn next_boundary(&self, c: usize, y: i32) -> (usize, usize, usize, usize) {
        let vsx = self.view.vsx as usize;
        let vsy = self.view.vsy as usize;
        let mut rows = vsy;
        let (mut c0, mut line, mut wrap, dx) = self.normalize_c(c);

        //println!("A{}/{}/{}", c0, line, wrap);
        if y < 0 {
            let mut y0 = -y;
            while y0 > 0 {
                let dy = min(y0, wrap as i32);
                wrap -= dy as usize;
                y0 -= dy;

                if y0 == 0 {
                    break;
                }
                if wrap == 0 {
                    if line == 0 {
                        break;
                    } else {
                        // go up to the next wrap
                        let (xc, xline, xwrap, _) = self.normalize_c(c0-1);
                        y0 -= 1;
                        c0 = xc;
                        line = xline;
                        wrap = xwrap;
                        //println!("B{}/{}/{}/{}", c0, line, wrap, y0);
                    }
                }
            }
            return (c0, line, wrap, dx);
        } else {
            let mut y0: usize = y as usize;
            let max_lines = self.text.len_lines() - 1;
            let len_chars = self.text.len_chars();
            //println!("C {:?}", (c, c0, line, wrap, len_chars, max_lines));
            if c >= len_chars {
                return (c0, line, wrap, dx);
            }

            if line >= max_lines {
                return (c0, line, wrap, dx);
            }

            let mut c1;
            while y0 > 0 && line < max_lines {
                let lc0 = self.text.line_to_char(line);
                let lc1 = self.text.line_to_char(line+1);
                // total wraps in line >= 1
                let wraps = (lc1 - lc0) / vsx + 1;
                c0 = lc0 + vsx * wrap;
                //println!("B {:?}", (y0, c0, lc0, lc1, wrap, wraps, line));
                wrap += 1;
                if wrap == wraps {
                    c1 = lc1;
                    line += 1;
                    wrap = 0;
                } else {
                    c1 = c0 + vsx;
                }
                //c1 = min(len_chars, c1);
                //println!("C {:?}", (len_chars, y0, c0, c1, lc0, lc1, wrap, wraps, line));
                if c1 != c0 {
                    //println!("S {:?}", self.text.slice(c0..c1).to_string());
                }

                y0 -= 1;
            }
            return (c0, line, wrap, dx);
        }
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

    #[derive(Debug)]
    struct BT(usize,i32,usize,usize,usize,usize);

    //#[test]
    fn test_boundary_1() {
        let mut buf = get_buf();
        dump(&mut buf);
        println!("len: {}", buf.text.len_chars());
        let end = buf.text.len_chars();
        let tests = vec![
            BT(0,0,0,0,0,0),
            BT(0,-1,0,0,0,0),
            BT(0,1,5,1,0,0),
            BT(5,-1,0,0,0,0),
            BT(0,2,11,2,0,0),
            BT(end,0,234,8,0,0),
            BT(end,-1,232,7,0,0),
            BT(end-1,0,232,7,0,1),
            BT(end-1,1,234,8,0,1),
            BT(end-1,-1,230,6,0,1),
        ];
        for bt in tests.iter() {
            let ci = bt.0;
            let dy = bt.1;
            let c0 = bt.2;
            let line0 = bt.3;
            let wrap0 = bt.4;
            let dx0 = bt.5;
            println!("TEST: {:?}", bt);
            let (c, line, wrap, dx) = buf.next_boundary(ci,dy);
            assert_eq!(c, c0);
            assert_eq!(line, line0);
            assert_eq!(wrap, wrap0);
            assert_eq!(dx, dx0);
        }
    }

    #[test]
    fn test_boundary_2() {
        let mut buf = get_buf();
        for i in -30..30 {
            let (c, line, wrap, dx) = buf.next_boundary(0,i);
            println!("B: {:?}", (i, c, line, wrap, dx, buf.text.len_chars(), buf.text.len_lines()) );
        }
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
