use super::TextBuffer;
use crate::frontend::DrawCommand;
use std::cmp::min;

impl TextBuffer {
    pub fn view_wrapped_lines(&mut self) -> Vec<String> {
        let mut out = Vec::new();
        let vsy = self.view.vsy as usize;
        for w in self.wrap_window_down(self.char_start, vsy) {
            out.push(w.to_string(&self));
        }
        out
    }

    pub fn render_view(&mut self) -> Vec<DrawCommand> {
        let mut out = Vec::new();
        let vsy = self.view.vsy as usize;
        let mut row = 0;
        let mut linex = 0;
        for w in self.view.wraps.iter() {
            let s = w.to_string(&self);
            if w.wrap0 == 0 || row == 0 {
                linex = w.line0 + 1;
            } else {
                linex = 0;
            }
            out.push(DrawCommand::Line(row as u16, linex, s.replace("\n", ".")));
            row += 1;
        }

        while out.len() < vsy {
            out.push(DrawCommand::Status(row, ";".to_string()));
            row += 1;
        }

        out.push(DrawCommand::Status(self.view.rInfo, format!("I: {}", self.view.debug)));
        out.push(DrawCommand::Status(self.view.rCmd, "".to_string()));
        let p = self.cursor();
        out.push(DrawCommand::Cursor(p.0 + 6, p.1));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_buf() -> TextBuffer {
        let mut buf = TextBuffer::from_str(r###"test
line2
estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estsx estst estst estst estst estst estst estst estst estst
asdf
1
2
3
4
"###);
        buf.set_size(20, 12);
        buf
    }

    #[test]
    fn test_wrapped_lines_end() {
        let mut buf = get_buf();
        buf.char_start = buf.text.line_to_char(buf.text.len_lines() - 1);
        buf.dump();
        let lines = buf.view_wrapped_lines();
        for line in &lines {
            println!("Line: {:?}", line);
        }
        assert_eq!(lines.len(), buf.view.vsy as usize);
    }

    #[test]
    fn test_wrapped_lines_empty() {
        let mut buf = TextBuffer::from_str("");
        buf.set_size(20, 12);
        let lines = buf.render_view();
        for line in &lines {
            println!("Line: {:?}", line);
        }
        assert_eq!(lines.len(), 13);
    }
}
