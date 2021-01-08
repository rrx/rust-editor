use super::TextBuffer;

impl TextBuffer {
    pub fn move_cursor_x(&mut self, c0: usize, dx: i32) {
        let mut c = c0 as i32 + dx;
        if c < 0 {
            c = 0;
        } else if c > self.text.len_chars() as i32 {
            c = self.text.len_chars() as i32;
        }

        self.update_window(c as usize);
    }

    pub fn move_cursor_y(&mut self, c0: usize, dy: i32) {
        //let mut c = self.char_current as i32 + dx;
        let mut w = self.delta_wrap(c0, dy);
        //let vsy = self.view.vsy as i32;
        let c = w.c0 + w.offset;
        //let y = self.view.cursor.1 as i32 + dy;
        //let mut cy = self.view.cursor.1;
        //if y < 0 {
            //cy = 0;
        //} else if y >= vsy as i32 {
            //cy = vsy as u16 - 1;
        //} else {
            //cy = y as u16;
        //}
        self.update_window(c);
        //self.view.cursor.1 = cy;
    }
}

