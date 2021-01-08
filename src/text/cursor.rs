use super::TextBuffer;

impl TextBuffer {
    pub fn move_cursor_x(&mut self, dx: i32) {
    }

    pub fn move_cursor_y(&mut self, dy: i32) {
        let mut w = self.delta_wrap(dy);
        let vsy = self.view.vsy as i32;
        let c = w.c0;
        let y = self.view.cursor.1 as i32 + dy;
        let mut cy = self.view.cursor.1;
        if y <= 0 {
            cy = 0;
            self.char_start = c;
        } else if y >= vsy as i32 {
            cy = vsy as u16 - 1;
            self.char_start = c;
        } else {
            cy = y as u16;
        }
        self.view.cursor.1 = cy;
    }
}

