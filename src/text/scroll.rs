use super::TextBuffer;
use std::cmp::min;

impl TextBuffer {
    pub fn scroll(&mut self, y: i32) {
        let vsx = self.view.vsx as usize;
        let vsy = self.view.vsy as usize;
        let (mut c, mut line, mut wrap, mut dx) = self.next_boundary(self.char_start, y);

        // compute end
        let size_chars = self.text.len_chars() - 1;
        let (mut c1, mut line1, mut wrap1, mut dx1) = self.next_boundary(size_chars, -(vsy as i32));
        let (mut c2, mut line2, mut wrap2, mut dx2) = self.next_boundary(size_chars, 0);

        if c > c1 {
            c = c1;
        }
        println!("x: {}/{}", self.char_start, c);
        self.char_start = c;
    }

    // return char index on the next lowest wrap boundary
    fn normalize_c(&self, c: usize) -> (usize, usize, usize, usize) {
        let vsx = self.view.vsx as usize;
        let line = self.text.char_to_line(c);
        let lc = self.text.line_to_char(line);
        let wrap = (c - lc) / vsx;
        let cn = lc + wrap * vsx;
        println!("c: {:?}", (c, cn, lc, line, wrap));
        return (cn, line, wrap, c - cn);
    }

    // y=-1, is the last wrap
    // y = 0, is the first wrap, or normalized c
    // return wraps relative to c
    fn next_boundary(&self, c: usize, y: i32) -> (usize, usize, usize, usize) {
        let vsx = self.view.vsx as usize;
        let vsy = self.view.vsy as usize;
        let mut rows = vsy;
        let (mut c0, mut line, mut wrap, mut dx) = self.normalize_c(c);

        println!("A{}/{}/{}", c0, line, wrap);
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
                        println!("B{}/{}/{}/{}", c0, line, wrap, y0);
                    }
                }
            }
            return (c0, line, wrap, dx);
        } else {
            let mut y0: usize = y as usize;
            let max_lines = self.text.len_lines() - 1;
            if line >= max_lines {
                return (c0, line, wrap, dx);
            }

            while y0 > 0 {
                let lc0 = self.text.line_to_char(line);
                let lc1 = self.text.line_to_char(line+1);
                let line_wraps = (lc1 - lc0) / vsx;
                let min_wraps = min(y0, line_wraps - wrap);
                y0 -= min_wraps;
                if min_wraps == line_wraps {
                    if y0 > 0 {
                        line += 1;
                        y0 -= 1;
                        wrap = 0;
                        c0 = lc1;
                    } else {
                        wrap = min_wraps;
                        c0 = lc0 + vsx * min_wraps;
                    }
                } else {
                    c0 = lc0 + vsx * min_wraps;
                    wrap = min_wraps;
                }
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
        buf.set_size(20, 10);
        buf.set_cursor(0,0);
        buf
    }

    fn dump(buf: &mut TextBuffer) {
        for command in buf.render_view() {
            println!("{:?}", command);
        }
        println!("{:?}", buf);
    }

    #[derive(Debug)]
    struct BT(usize,i32,usize,usize,usize,usize);

    #[test]
    fn test_boundary() {
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
    fn test_scroll_up() {
        let mut buf = get_buf();
        dump(&mut buf);
        assert_eq!(0, buf.char_start);
        buf.scroll(1);
        dump(&mut buf);
        assert_eq!(5, buf.char_start);
        buf.scroll(1);
        assert_eq!(11, buf.char_start);
        buf.scroll(1);
        assert_eq!(11+buf.view.vsx as usize, buf.char_start);
        buf.scroll(1);
        assert_eq!(11+buf.view.vsx as usize, buf.char_start);
        buf.scroll(10);
        dump(&mut buf);
        assert_eq!(11+buf.view.vsx as usize, buf.char_start);
    }

    #[test]
    fn test_scroll_down() {
        let mut buf = get_buf();
        dump(&mut buf);
        buf.scroll(1);
    }
}
