use super::TextBuffer;

impl TextBuffer {
    pub fn line_move(&mut self, x: i32) {
        let mut w = self.char_to_wrap(self.char_current).unwrap();
        let mut lc = x;
        let line_length = w.lc1 - w.lc0;
        if x < 0 {
            lc += line_length as i32;
        }
        if lc < 0 || line_length == 0 {
            lc = 0;
        } else if lc >= line_length as i32 {
            lc = line_length as i32 - 1;
        }
        let c = w.lc0 + lc as usize;
        if c != self.char_current {
            self.update_window(c as usize);
        }
    }

    pub fn move_cursor_x(&mut self, c0: usize, dx: i32) {
        self._move_cursor_x(c0, dx, false);
    }

    pub fn _move_cursor_x(&mut self, c0: usize, dx: i32, constrain: bool) {
        let mut c = c0 as i32 + dx;
        if c < 0 {
            c = 0;
        } else if c > self.text.len_chars() as i32 {
            c = self.text.len_chars() as i32;
        }

        let mut c1 = c as usize;
        if constrain {
            // restrict x movement to the specific line
            let mut w = self.char_to_wrap(c0).unwrap();
            let line_length = w.lc1 - w.lc0;
            if c1 < w.lc0 {
                c1 = w.lc0;
            } else if c1 >= w.lc1 {
                if line_length > 0 {
                    c1 = w.lc1 - 1;
                } else {
                    c1 = w.lc0;
                }
            }
        }

        if c0 != c1 {
            let mut w = self.char_to_wrap(c1).unwrap();
            let hint = c1 - w.c0;
            self.view.cursor_x_hint = hint as u16;
            self.update_window(c1);
        }
    }

    pub fn move_cursor_y(&mut self, c0: usize, dy: i32) {
        let mut w = self.delta_wrap(c0, dy);

        // use x hint
        let mut c = w.c0 + self.view.cursor_x_hint as usize;
        if c >= w.lc1 && w.lc0 < w.lc1 {
            c = w.lc1 - 1;
        }
        self.update_window(c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_buf() -> TextBuffer {
        let mut buf = TextBuffer::from_str(r###"0123456
0123456
0123456
0123456
0123456 "###);
        buf.set_size(20, 8);
        buf
    }

    #[test]
    fn test_wrap_x() {
        let mut buf = get_buf();
        let mut c = 0;
        assert_eq!(0, buf.view.cursor.0);
        buf.move_cursor_x(c,1);
        buf.dump();
        println!("W: {:?}", (buf.view.cursor));
        assert_eq!(1, buf.view.cursor.0);
    }
}

