use super::*;
use log::*;

pub fn string_to_elements(s: &String) -> Vec<ViewChar> {
    use ViewChar::*;
    s.chars().fold(Vec::new(), |mut v, c| match c {
        '\t' => {
            v.extend_from_slice(&[NOP, NOP, NOP, Tab]);
            v
        }
        '\n' => {
            v.push(NL);
            v
        }
        _ => {
            v.push(Char(c));
            v
        }
    })
}

#[derive(Debug)]
pub struct FormatItem {
    len: usize,
    s: String,
    t: LineFormatType,
    format: LineFormatType,
}

pub struct FormatIterator<'a> {
    line: &'a String,
    inx: usize,
    format: LineFormatType,
    highlight: String,
}

impl<'a> Iterator for FormatIterator<'a> {
    type Item = FormatItem;
    fn next(&mut self) -> Option<Self::Item> {
        if self.inx >= self.line.len() {
            None
        } else {
            use LineFormatType::*;
            // search bounds for hightlight
            let search_start =
                std::cmp::max(0, self.inx as i32 - self.highlight.len() as i32 + 1) as usize;
            let search_end =
                std::cmp::min(self.line.len(), self.inx + self.highlight.len()) as usize;

            let mut highlight = false;
            if search_end > search_start && search_end - search_start >= self.highlight.len() {
                info!("search: {:?}", (search_start, search_end, self.line));
                match self.line.get(search_start..search_end) {
                    Some(range) => {
                        let matches = range.matches(&self.highlight).next().is_some();
                        //if self.highlight == range {
                        if matches {
                            highlight = true;
                        }
                    }
                    //None => unreachable!()
                    None => (),
                }
            }
            //info!("X:{:?}", (self.inx, search_start, search_end, highlight));
            match self.line.get(self.inx..self.inx + 1) {
                Some(ch) => {
                    self.inx += 1;
                    let (tt, len, s) = match ch {
                        "\t" => (Dim, 4, "   \u{2192}".to_string()), // right arrow
                        "\n" => (Dim, 1, "\u{00B6}".to_string()),    // paragraph symbol
                        _ => (Normal, 1, ch.to_string()),
                    };
                    let t = if highlight { Highlight } else { tt };

                    Some(FormatItem {
                        s,
                        len,
                        t,
                        format: self.format,
                    })
                }
                None => None, //None => unreachable!()
            }
        }
    }
}

impl<'a> FormatIterator<'a> {
    fn new(line: &'a String, inx: usize, highlight: String) -> Self {
        Self {
            line,
            inx,
            highlight,
            format: LineFormatType::Normal,
        }
    }
}

pub fn format_wrapped(line: &String, sx: usize, highlight: String) -> Vec<Vec<LineFormat>> {
    let mut it = FormatIterator::new(line, 0, highlight);
    let mut rx = 0;
    let mut out = vec![];
    let mut format = LineFormatType::Normal;
    let mut acc = String::from("");
    let mut row_count = 0;
    let mut row: Vec<LineFormat> = Vec::new();
    let end = line.len();
    while rx < end {
        let o = it.next();
        match o {
            Some(i) => {
                //println!("match: {:?}", (rx, start));
                rx += i.len;

                // make a row
                if row_count == sx {
                    if acc.len() > 0 {
                        row.push(LineFormat(format, acc.clone()));
                        acc.truncate(0);
                    }
                    out.push(row.clone());
                    row.truncate(0);
                    row_count = 0;
                }

                if format != i.t {
                    if acc.len() > 0 {
                        row.push(LineFormat(format, acc.clone()));
                        acc.truncate(0);
                    }
                    format = i.t;
                }
                acc.push_str(&i.s);
                row_count += 1;
            }
            None => break,
        }
    }

    // handle remainder
    if acc.len() > 0 {
        row.push(LineFormat(format, acc.clone()));
    }
    if row.len() > 0 {
        out.push(row);
    }

    out
}

pub fn format_line(line: &String, highlight: String) -> Vec<LineFormat> {
    format_range(line, 0, line.len(), highlight)
}

pub fn format_range(line: &String, start: usize, end: usize, highlight: String) -> Vec<LineFormat> {
    let mut it = FormatIterator::new(line, 0, highlight);
    let mut rx = 0;
    let mut out = vec![];
    let mut format = LineFormatType::Normal;
    let mut acc = String::from("");

    while rx < start {
        match it.next() {
            Some(i) => {
                //info!("skip: {:?}", (rx, start));
                rx += i.len;
            }
            None => break,
        }
    }

    while rx < end {
        match it.next() {
            Some(i) => {
                //info!("match: {:?}", (rx, start));
                rx += i.len;
                if format != i.t {
                    if acc.len() > 0 {
                        out.push(LineFormat(format, acc.clone()));
                    }
                    acc.truncate(0);
                    format = i.t;
                }
                acc.push_str(&i.s);
            }
            None => break,
        }
    }

    // handle remainder
    if acc.len() > 0 {
        out.push(LineFormat(format, acc.clone()));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use LineFormatType::*;
    use ViewChar::*;
    #[test]
    fn test_format_range_1() {
        let line = String::from("asdf");
        let r = format_range(&line, 0, line.len(), "asdf".into());
        println!("1:{:?}", r);
        assert_eq!(vec![LineFormat(Highlight, "asdf".into())], r);
    }
    #[test]
    fn test_format_range_2() {
        let line = String::from("xasdfx");
        let r = format_range(&line, 1, line.len() - 1, "sd".into());
        println!("1:{:?}", r);
        assert_eq!(
            vec![
                LineFormat(Normal, "a".into()),
                LineFormat(Highlight, "sd".into()),
                LineFormat(Normal, "f".into()),
            ],
            r
        );
    }
    #[test]
    fn test_format_range_3() {
        let line = String::from("asdf");
        let r = format_range(&line, 0, line.len(), "a".into());
        println!("1:{:?}", r);
        assert_eq!(
            vec![
                LineFormat(Highlight, "a".into()),
                LineFormat(Normal, "sdf".into()),
            ],
            r
        );
        let r = format_range(&line, 0, line.len(), "f".into());
        println!("1:{:?}", r);
        assert_eq!(
            vec![
                LineFormat(Normal, "asd".into()),
                LineFormat(Highlight, "f".into()),
            ],
            r
        );
    }

    #[test]
    fn test_format_range_4() {
        let line = String::from("asdf");
        let r = format_wrapped(&line, 2, "sd".into());
        println!("1:{:?}", r);
        assert_eq!(
            vec![
                vec![
                    LineFormat(Normal, "a".into()),
                    LineFormat(Highlight, "s".into())
                ],
                vec![
                    LineFormat(Highlight, "d".into()),
                    LineFormat(Normal, "f".into())
                ]
            ],
            r
        );
    }

    #[test]
    fn test_format_chinese() {
        let line = String::from("mèng 梦/夢");
        let r = format_wrapped(&line, 2, "sd".into());
        println!("1:{:?}", r);
        // TODO: chinese is not handled well
        //assert_eq!(vec![
        //vec![LineFormat(Normal, "a".into()), LineFormat(Highlight, "s".into())],
        //vec![LineFormat(Highlight, "d".into()), LineFormat(Normal, "f".into())]
        //], r);
    }
}
