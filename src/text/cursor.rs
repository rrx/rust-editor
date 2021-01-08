use super::TextBuffer;

impl TextBuffer {
    pub fn move_cursor_x(&mut self, c0: usize, dx: i32) {
        let mut c = c0 as i32 + dx;
        if c < 0 {
            c = 0;
        } else if c > self.text.len_chars() as i32 {
            c = self.text.len_chars() as i32;
        }

        if c as usize != c0 {
            self.update_window(c as usize);
        }
    }

    pub fn move_cursor_y(&mut self, c0: usize, dy: i32) {
        let mut w = self.delta_wrap(c0, dy);
        let c = w.c0 + w.offset;
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
