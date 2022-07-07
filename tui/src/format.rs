use super::*;
use editor_core::{grapheme_width, BufferConfig};
use log::*;
use unicode_segmentation::{Graphemes, UnicodeSegmentation};

pub struct FormatIterator<'a> {
    line: &'a String,
    grapheme_iter: Graphemes<'a>,
    inx: usize,
    //format: LineFormatType,
    highlight: String,
    config: &'a BufferConfig,
}

impl<'a> Iterator for FormatIterator<'a> {
    type Item = FormatItem;
    fn next(&mut self) -> Option<Self::Item> {
        if self.inx >= self.line.chars().count() {
            None
        } else {
            // search bounds for hightlight
            let search_start =
                std::cmp::max(0, self.inx as i32 - self.highlight.len() as i32 + 1) as usize;
            let search_end =
                std::cmp::min(self.line.len(), self.inx + self.highlight.len()) as usize;

            let mut highlight = false;
            if self.highlight.len() > 0
                && search_end > search_start
                && search_end - search_start >= self.highlight.len()
            {
                debug!("search: {:?}", (search_start, search_end, self.line));
                let range = self
                    .line
                    .graphemes(true)
                    .skip(search_start)
                    .take(search_end - search_start)
                    .collect::<Vec<&str>>()
                    .join("");
                if range.matches(&self.highlight).next().is_some() {
                    highlight = true;
                }
            }

            match self.grapheme_iter.next() {
                Some(ch) => {
                    self.inx += 1;
                    let mut items = grapheme_to_format_item(ch, &self.config, highlight);
                    items.pop()
                }
                None => None,
            }
        }
    }
}

impl<'a> FormatIterator<'a> {
    fn new(line: &'a String, inx: usize, highlight: String, config: &'a BufferConfig) -> Self {
        Self {
            line,
            grapheme_iter: line.graphemes(true),
            inx,
            highlight,
            //format: LineFormatType::Normal,
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
    let mut it = FormatIterator::new(line, 0, highlight, config);
    let mut out = vec![];
    let mut format = LineFormatType::Normal;
    let mut acc = String::from("");
    let mut render_count = 0;
    let mut row: Vec<LineFormat> = Vec::new();
    loop {
        let o = it.next();
        match o {
            Some(i) => {
                // make a row, if we have reached the end of the wrapped line
                if render_count + i.unicode_width > sx {
                    // make a new row
                    if acc.len() > 0 {
                        row.push(LineFormat::new(format, acc.clone()));
                        acc.truncate(0);
                    }
                    out.push(row.clone());
                    row.truncate(0);
                    render_count = 0;
                }

                // make a row if formatting has changed
                if format != i.format {
                    if acc.len() > 0 {
                        row.push(LineFormat::new(format, acc.clone()));
                        acc.truncate(0);
                    }
                    format = i.format;
                }

                // add to this row
                acc.push_str(&i.s);
                render_count += i.unicode_width;
            }
            None => {
                break;
            }
        }
    }

    // handle remainder
    if acc.len() > 0 {
        row.push(LineFormat::new(format, acc.clone()));
    }
    if row.len() > 0 {
        out.push(row);
    }

    out
}

pub fn format_line(line: &String, highlight: String, config: &BufferConfig) -> Vec<LineFormat> {
    format_range(line, 0, grapheme_width(line), highlight, config)
}

pub fn format_range(
    line: &String,
    start: usize,
    end: usize,
    highlight: String,
    config: &BufferConfig,
) -> Vec<LineFormat> {
    let mut it = FormatIterator::new(line, 0, highlight, config);
    let mut rx = 0;
    let mut out = vec![];
    let mut format = LineFormatType::Normal;
    let mut acc = String::from("");

    while rx < start {
        match it.next() {
            Some(i) => {
                //info!("skip: {:?}", (rx, start));
                rx += i.unicode_width;
            }
            None => break,
        }
    }

    while rx < end {
        match it.next() {
            Some(i) => {
                //info!("match: {:?}", (rx, start));
                rx += i.unicode_width;
                if format != i.format {
                    if acc.len() > 0 {
                        out.push(LineFormat::new(format, acc.clone()));
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
        out.push(LineFormat::new(format, acc.clone()));
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
        assert_eq!(vec![LineFormat::new(Highlight, "asdf".into())], r);
    }
    #[test]
    fn test_format_range_2() {
        let config = BufferConfig::config_for(None);
        let line = String::from("xasdfx");
        let r = format_range(&line, 1, line.len() - 1, "sd".into(), &config);
        println!("1:{:?}", r);
        assert_eq!(
            vec![
                LineFormat::new(Normal, "a".into()),
                LineFormat::new(Highlight, "sd".into()),
                LineFormat::new(Normal, "f".into()),
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
                LineFormat::new(Highlight, "a".into()),
                LineFormat::new(Normal, "sdf".into()),
            ],
            r
        );
        let r = format_range(&line, 0, line.len(), "f".into(), &config);
        println!("1:{:?}", r);
        assert_eq!(
            vec![
                LineFormat::new(Normal, "asd".into()),
                LineFormat::new(Highlight, "f".into()),
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
                    LineFormat::new(Normal, "a".into()),
                    LineFormat::new(Highlight, "s".into())
                ],
                vec![
                    LineFormat::new(Highlight, "d".into()),
                    LineFormat::new(Normal, "f".into())
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

        let r = format_wrapped(&line, 2, "".into(), &config);
        println!("2:{:?}", r);
    }

    #[test]
    fn test_format_control() {
        let config = BufferConfig::config_tabs();
        let mut line = String::from("");
        line.push(char::from_u32(13).unwrap());
        line.push(char::from_u32(13).unwrap());
        let r = format_wrapped(&line, 9, "".into(), &config);
        // this should split into 2 rows
        assert_eq!(r.len(), 2);
        println!("1:{:?}", r);
    }

    #[test]
    fn test_format_chinese() {
        let config = BufferConfig::config_spaces(2);
        let line = String::from("\tmèng 梦/夢\t梦夢\t");
        let r = format_wrapped(&line, 2, "sd".into(), &config);
        for row in r.iter() {
            for format in row.iter() {
                println!("1:{:?}", format);
                assert!(grapheme_width(&format.s) <= 2)
            }
        }
    }
}
