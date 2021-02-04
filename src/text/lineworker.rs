use log::*;
use ropey::Rope;
use super::*;

pub struct LineWorker { }
impl LineWorker {
    //pub fn render(text: &Rope, spec: &ViewSpec, start: &Cursor, cursor: &Cursor) -> (Cursor, Vec<DrawCommand>) {
        //let sx = spec.sx as usize;
        //let sy = spec.sy as usize;
        //let header = spec.header as usize;

        //let (cx, cy, rows) = Self::screen_from_cursor(text, sx, sy, start, cursor);
        //let commands = vec![];
        //let start = rows[0].cursor.clone();
        //(start, commands)
    //}

    pub fn screen_from_start(text: &Rope, sx: usize, sy: usize, start: &Cursor, cursor: &Cursor) -> Vec<RowItem> {
        // start with the current position, iterate back until we find the start, or we fill up the
        // screen
        // iterate next until we fill up the screen
        let mut out = Vec::new();
        let mut c = start.clone();
        out.push(cursor_to_row(start, sx));
        while out.len() < sy {
            match cursor_visual_next_line(&text, sx, &c) {
                Some(x) => {
                    c = x;
                    out.push(cursor_to_row(&c, sx));
                }
                None => break
            }
        }
        out
    }

    pub fn screen_from_cursor(text: &Rope, sx: usize, sy: usize, start: &Cursor, cursor: &Cursor) -> (u16, u16, Vec<RowItem>) {
        // start with the current position, iterate back until we find the start, or we fill up the
        // screen
        // iterate next until we fill up the screen
        let mut out = Vec::new();
        let rx = cursor.rx(sx);
        let mut ry = 0;

        out.push(cursor_to_row(cursor, sx));

        let mut cp = cursor.clone();
        while out.len() < sy {
            if cp.c <= start.c {
                break;
            }
            match cursor_visual_prev_line(text, sx, &cp) {
                Some(x) => {
                    cp = x;
                    out.insert(0, cursor_to_row(&cp, sx));
                    ry += 1;
                }
                None => break
            }
        }

        let mut cn = cursor.clone();
        while out.len() < sy {
            match cursor_visual_next_line(text, sx, &cn) {
                Some(x) => {
                    cn = x;
                    out.push(cursor_to_row(&cn, sx));
                }
                None => break
            }
        }
        (rx as u16, ry, out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ViewChar::*;

    #[test]
    fn test_lineworker_1() {
        let mut text = Rope::from_str("a\nb\nc");
        let (sx, sy) = (10, 10);
        let mut c = cursor_start(&text, sx);
        let mut start = c.clone();
        let lines: usize = text.len_lines() - 1;
        c = cursor_eof(&text, sx);
        let (cx, cy, rows) = LineWorker::screen_from_cursor(&text, sx, sy, &start, &c);
        start = rows[0].cursor.clone();
        println!("r2:{:?}", (&c, &start));

        c = cursor_from_line(&text, sx, text.len_lines());
        let (cx, cy, rows) = LineWorker::screen_from_cursor(&text, sx, sy, &start, &c);
        start = rows[0].cursor.clone();
        println!("r2:{:?}", (&c, &start));
    }
}

