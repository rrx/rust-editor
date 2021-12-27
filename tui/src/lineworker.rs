use super::*;
use editor_core::BufferConfig;

use ropey::Rope;

pub fn to_row(cursor: &Cursor) -> RowItem {
    RowItem {
        cursor: cursor.clone(),
    }
}

pub struct LineWorker {}
impl LineWorker {
    pub fn screen_from_start(
        text: &Rope,
        sx: usize,
        sy: usize,
        start: &Cursor,
        _cursor: &Cursor
    ) -> Vec<RowItem> {
        // start with the current position, iterate back until we find the start, or we fill up the
        // screen
        // iterate next until we fill up the screen
        let mut out = Vec::new();
        let mut c = start.clone();
        out.push(to_row(start));
        while out.len() < sy {
            match cursor_visual_next_line(&text, sx, &c) {
                Some(x) => {
                    c = x;
                    out.push(to_row(&c));
                }
                None => break,
            }
        }
        out
    }

    pub fn screen_from_cursor(
        text: &Rope,
        sx: usize,
        sy: usize,
        start: &Cursor,
        cursor: &Cursor
    ) -> (u16, u16, Vec<RowItem>) {
        // start with the current position, iterate back until we find the start, or we fill up the
        // screen
        // iterate next until we fill up the screen
        let mut out = Vec::new();
        let rx = cursor.rx(sx);
        let mut ry = 0;

        out.push(to_row(cursor));

        let mut cp = cursor.clone();
        while out.len() < sy {
            if cp.c <= start.c {
                break;
            }
            match cursor_visual_prev_line(text, sx, &cp) {
                Some(x) => {
                    cp = x;
                    out.insert(0, to_row(&cp));
                    ry += 1;
                }
                None => break,
            }
        }

        let mut cn = cursor.clone();
        while out.len() < sy {
            match cursor_visual_next_line(text, sx, &cn) {
                Some(x) => {
                    cn = x;
                    out.push(to_row(&cn));
                }
                None => break,
            }
        }
        (rx as u16, ry, out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lineworker_1() {
        let config = BufferConfig::config_for(None);
        let text = Rope::from_str("a\nb\nc");
        let (sx, sy) = (10, 10);
        let mut c = cursor_start(&text, sx, &config);
        let mut start = c.clone();
        let _lines: usize = text.len_lines() - 1;
        c = cursor_eof(&text, sx, &config);
        let config = BufferConfig::config_for(None);
        let (_cx, _cy, rows) = LineWorker::screen_from_cursor(&text, sx, sy, &start, &c);
        start = rows[0].cursor.clone();
        println!("r2:{:?}", (&c, &start));

        c = cursor_from_line(&text, sx, &config, text.len_lines());
        let (_cx, _cy, rows) = LineWorker::screen_from_cursor(&text, sx, sy, &start, &c);
        start = rows[0].cursor.clone();
        println!("r2:{:?}", (&c, &start));
    }
}
