use log::*;
use ropey::Rope;
use super::*;

pub struct LineWorker { }
impl LineWorker {
    pub fn render_rows(text: &Rope, spec: &ViewSpec, cx: u16, cy: u16, rows: &Vec<RowItem>, cursor: &Cursor) -> Vec<DrawCommand> {
        //let sx = spec.sx as usize;
        let sy = spec.sy as usize;
        let header = spec.header as usize;
        //info!("rows: {:?}", rows);
        let start = rows[0].cursor.clone();

        let mut out = Vec::new();
        if spec.header > 0 {
            out.push(DrawCommand::Status(out.len() as u16, format!("Rust-Editor-{} {:width$}", clap::crate_version!(), cursor.simple_format(), width=spec.w as usize).into()));
        }

        let row_inx = out.len();
        rows.iter().enumerate().map(|(inx, row)| {
            let mut line_display = 0; // zero means leave line blank
            if row.cursor.wrap0 == 0 || inx == 0 {
                line_display = row.cursor.line_inx + 1; // display one based
            }
            //DrawCommand::Line(row_inx + inx as u16, line_display, row.to_string())
            let mut parts = Vec::new();
            let fs;
            if line_display > 0 {
                fs = format!("{:5}\u{23A5}", line_display)
            } else {
                fs = format!("{:5}\u{23A5}", " ")
            }
            use ViewChar::*;
            use LineFormatType::*;
            parts.push(LineFormat(Dim, fs));
            let mut format = Some(Normal);
            let mut acc = String::from("");

            row.elements.iter().for_each(|v| {
                let (f, translated) = match v {
                    NOP => (Dim, ' '),
                    Tab => (Dim, '\u{2192}'), // right arrow
                    NL => (Dim, '\u{00B6}'), // paragraph symbol
                    ViewChar::Char(x) => (Normal, *x),
                    OOB => (Highlight, 'O')
                };

                // initialize format
                if format.is_none() {
                    format = Some(f)
                }

                let f0 = format.unwrap();
                if f0 != f {
                    parts.push(LineFormat(f0, acc.clone()));
                    acc.truncate(0);
                    format = Some(f);
                }

                acc.push(translated);
            });

            if acc.len() > 0 {
                parts.push(LineFormat(format.unwrap(), acc.clone()));
            }

            //parts.push(LineFormat(LineFormatType::Normal, row.to_string()));
            DrawCommand::Format(9, row_inx + inx, parts)
        }).for_each(|c| {
            out.push(c);
        });

        while out.len() < sy + header {
            out.push(DrawCommand::Row(0, out.len() as u16, ";".into()));
        }

        if spec.status > 0 {
            out.push(DrawCommand::Status(out.len() as u16, format!(
                        "DEBUG: [{},{}] S:{} C:{:width$}",
                        cx, cy,
                        &cursor.simple_format(),
                        &start.simple_format(),
                        width=spec.w as usize)
                    .into()));
        }

        if spec.footer > 0 {
            out.push(DrawCommand::Clear(0, out.len()));
        }

        out.push(DrawCommand::Cursor(cx + spec.x0, cy + spec.y0));
        out
    }

    pub fn render(text: &Rope, spec: &ViewSpec, start: &Cursor, cursor: &Cursor) -> (Cursor, Vec<DrawCommand>) {
        let sx = spec.sx as usize;
        let sy = spec.sy as usize;
        let header = spec.header as usize;

        let (cx, cy, rows) = Self::screen_from_cursor(text, sx, sy, start, cursor);
        let commands = Self::render_rows(text, spec, cx, cy, &rows, cursor);
        let start = rows[0].cursor.clone();
        (start, commands)
    }

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

        //rx = cursor.rx;
        out.push(cursor_to_row(cursor, sx));

        //let mut count = 0;
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
                    //count += 1;
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

    pub fn current(text: &Rope, sx: usize, cursor: &Cursor) -> RowItem {
        let mut iter = Self::iter(text.clone(), sx, cursor.clone());
        iter.next().unwrap()
    }

    pub fn iter(text: Rope, sx: usize, cursor: Cursor) -> LineIter {
        LineIter::new(text, sx, cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ViewChar::*;

    //#[test]
    fn test_rowiter_2() {
        let mut text = Rope::from_str("123456789\nabcdefghijk\n");
        let (sx, sy) = (5, 2);
        let mut c = cursor_start(&text, sx);
        let start = c.clone();
        let mut it = LineWorker::iter(text.clone(), sx, c.clone());
        println!("2:{:?}", (text.len_lines()));
        while let Some(x) = it.next() {
            println!("next: {:?}", x.to_string());
        }

        let (cx, cy, rows) = LineWorker::screen_from_cursor(&text, sx, sy, &start, &c);
        assert_eq!(sy, rows.len());
        rows.iter().for_each(|row| {
            println!("R: {:?}", (row.to_string(), row));
        });
    }

    #[test]
    fn test_rowiter_last_line() {
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

