use super::*;
use log::*;

fn expand_tab(config: &BufferConfig) -> Vec<ViewChar> {
    use ViewChar::*;
    match config.indent_style {
        IndentStyle::Tab => {
            let mut v = Vec::new();
            (0..config.tab_width - 1).for_each(|_| v.push(NOP));
            v.push(Tab);
            v
        }
        IndentStyle::Space => {
            let spaces = match config.indent_size {
                IndentSize::Tab => config.tab_width,
                IndentSize::Size(n) => n,
            } as usize;
            let mut v = Vec::new();
            (0..spaces - 1).for_each(|_| v.push(NOP));
            v.push(Tab);
            v
        }
    }
}

fn expand_newline(config: &BufferConfig) -> Vec<ViewChar> {
    use ViewChar::*;
    match config.end_of_line {
        EndOfLine::Lf => vec![NL],
        EndOfLine::CrLf => vec![NL],
        EndOfLine::Cr => vec![NL],
    }
}

pub fn string_to_elements(s: &String, config: &BufferConfig) -> Vec<ViewChar> {
    use ViewChar::*;
    s.chars().fold(Vec::new(), |mut v, c| match c {
        '\t' => {
            v.append(&mut expand_tab(config));
            v
        }
        '\n' => {
            v.append(&mut expand_newline(config));
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
    //t: LineFormatType,
    format: LineFormatType,
}

pub struct FormatIterator<'a> {
    line: &'a String,
    inx: usize,
    format: LineFormatType,
    highlight: String,
    config: BufferConfig,
}

fn format_tab(tab_size: usize) -> String {
    let mut s = " ".repeat(tab_size - 1);
    s.push_str("\u{2192}"); // right arrow
    s
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
                debug!("search: {:?}", (search_start, search_end, self.line));
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
                    let (tt, s) = match ch {
                        "\t" => (Dim, format_tab(self.config.tab_width as usize)), // right arrow
                        "\n" => (Dim, "\u{00B6}".to_string()), // paragraph symbol
                        _ => (Normal, ch.to_string()),
                    };
                    let format = if highlight { Highlight } else { tt };

                    let size = s.len();
                    Some(FormatItem {
                        s,
                        len: size,
                        //t,
                        format,
                    })
                }
                None => None, //None => unreachable!()
            }
        }
    }
}

impl<'a> FormatIterator<'a> {
    fn new(line: &'a String, inx: usize, highlight: String, config: BufferConfig) -> Self {
        Self {
            line,
            inx,
            highlight,
            format: LineFormatType::Normal,
            config,
        }
    }
}

pub fn format_wrapped(
    line: &String,
    sx: usize,
    highlight: String,
    config: &BufferConfig,
) -> Vec<Vec<LineFormat>> {
    let mut it = FormatIterator::new(line, 0, highlight, config.clone());
    let mut ch_count = 0;
    let mut out = vec![];
    let mut format = LineFormatType::Normal;
    let mut acc = String::from("");
    let mut row_count = 0;
    let mut row: Vec<LineFormat> = Vec::new();
    let end = line.len();
    debug!("Format line: {:?}", (line, end));
    while ch_count < end {
        let o = it.next();
        match o {
            Some(i) => {
                debug!("match: {:?}", (ch_count, format, &i));
                ch_count += 1;

                // make a row, if we have reached the end of the wrapped line
                if row_count == sx {
                    if acc.len() > 0 {
                        row.push(LineFormat(format, acc.clone()));
                        acc.truncate(0);
                    }
                    out.push(row.clone());
                    row.truncate(0);
                    row_count = 0;
                }

                // if formatting has changed, then push that
                if format != i.format {
                    if acc.len() > 0 {
                        row.push(LineFormat(format, acc.clone()));
                        acc.truncate(0);
                    }
                    format = i.format;
                }
                acc.push_str(&i.s);
                row_count += 1;
            }
            None => {
                debug!("no match: {:?}", (ch_count));
                break;
            }
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

pub fn format_line(line: &String, highlight: String, config: &BufferConfig) -> Vec<LineFormat> {
    format_range(line, 0, line.len(), highlight, config)
}

pub fn format_range(
    line: &String,
    start: usize,
    end: usize,
    highlight: String,
    config: &BufferConfig,
) -> Vec<LineFormat> {
    let mut it = FormatIterator::new(line, 0, highlight, config.clone());
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
                if format != i.format {
                    if acc.len() > 0 {
                        out.push(LineFormat(format, acc.clone()));
                    }
                    acc.truncate(0);
                    format = i.format;
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
    #[test]
    fn test_format_range_1() {
        let config = BufferConfig::config_for(None);
        let line = String::from("asdf");
        let r = format_range(&line, 0, line.len(), "asdf".into(), &config);
        println!("1:{:?}", r);
        assert_eq!(vec![LineFormat(Highlight, "asdf".into())], r);
    }
    #[test]
    fn test_format_range_2() {
        let config = BufferConfig::config_for(None);
        let line = String::from("xasdfx");
        let r = format_range(&line, 1, line.len() - 1, "sd".into(), &config);
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
        let config = BufferConfig::config_for(None);
        let line = String::from("asdf");
        let r = format_range(&line, 0, line.len(), "a".into(), &config);
        println!("1:{:?}", r);
        assert_eq!(
            vec![
                LineFormat(Highlight, "a".into()),
                LineFormat(Normal, "sdf".into()),
            ],
            r
        );
        let r = format_range(&line, 0, line.len(), "f".into(), &config);
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
        let config = BufferConfig::config_for(None);
        let line = String::from("asdf");
        let r = format_wrapped(&line, 2, "sd".into(), &config);
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
    fn test_format_tab() {
        let config = BufferConfig::config_tabs();
        let line = String::from("\t\t\t\tooo");
        let r = format_wrapped(&line, 10, "sd".into(), &config);
        println!("1:{:?}", r);
    }

    #[test]
    fn test_format_chinese() {
        let config = BufferConfig::config_for(None);
        let line = String::from("mèng 梦/夢");
        let r = format_wrapped(&line, 2, "sd".into(), &config);
        println!("1:{:?}", r);
        // TODO: chinese is not handled well
        //assert_eq!(vec![
        //vec![LineFormat(Normal, "a".into()), LineFormat(Highlight, "s".into())],
        //vec![LineFormat(Highlight, "d".into()), LineFormat(Normal, "f".into())]
        //], r);
    }
}
