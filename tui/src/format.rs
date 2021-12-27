use super::*;
use log::*;
use crate::*;
use editor_core::{EndOfLine, BufferConfig, IndentStyle, IndentSize, grapheme_width};
use crate::LineFormatType::*;
use unicode_width::UnicodeWidthStr;
use std::str::Chars;
use unicode_segmentation::{Graphemes, UnicodeSegmentation};

pub struct FormatIterator<'a> {
    line: &'a String,
    grapheme_iter: Graphemes<'a>,
    inx: usize,
    format: LineFormatType,
    highlight: String,
    config: &'a BufferConfig,
}

impl<'a> Iterator for FormatIterator<'a> {
    type Item = FormatItem;
    fn next(&mut self) -> Option<Self::Item> {
        if self.inx >= self.line.chars().count() {
            None
        } else {
            use LineFormatType::*;
            // search bounds for hightlight
            let search_start =
                std::cmp::max(0, self.inx as i32 - self.highlight.len() as i32 + 1) as usize;
            let search_end =
                std::cmp::min(self.line.len(), self.inx + self.highlight.len()) as usize;

            let mut highlight = false;
            if self.highlight.len() > 0 && search_end > search_start && search_end - search_start >= self.highlight.len() {
                debug!("search: {:?}", (search_start, search_end, self.line));
                let range = self.line.graphemes(true).skip(search_start).take(search_end - search_start).collect::<Vec<&str>>().join("");
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
    let mut it = FormatIterator::new(line, 0, highlight, config);
    let mut ch_count = 0;
    let mut out = vec![];
    let mut format = LineFormatType::Normal;
    let mut acc = String::from("");
    let mut row_count = 0;
    let mut row: Vec<LineFormat> = Vec::new();
    let end = grapheme_width(line);
    debug!("Format line: {:?}", (line, end));
    while ch_count < end {
        let o = it.next();
        match o {
            Some(i) => {
                debug!("match: {:?}", (ch_count, format, &i));
                //println!("{:?}", (ch_count, format, &i));
                ch_count += 1;

                // make a row, if we have reached the end of the wrapped line
                if row_count == sx {
                    if acc.len() > 0 {
                        row.push(LineFormat::new(format, acc.clone()));
                        acc.truncate(0);
                    }
                    out.push(row.clone());
                    row.truncate(0);
                    row_count = 0;
                }

                // if formatting has changed, then push that
                if format != i.format {
                    if acc.len() > 0 {
                        row.push(LineFormat::new(format, acc.clone()));
                        acc.truncate(0);
                    }
                    format = i.format;
                }
                acc.push_str(&i.s);
                row_count += 1;
            }
            None => {
                debug!("no match: {:?}", (ch_count));
                //println!("nomatch");
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
        let r = format_wrapped(&line, 10, "".into(), &config);
        println!("1:{:?}", r);
    }

    #[test]
    fn test_format_chinese() {
        let config = BufferConfig::default();
        let line = String::from("\tmèng 梦/夢\t");
        let r = format_wrapped(&line, 2, "sd".into(), &config);
        println!("1:{:?}", r);
        // TODO: chinese is not handled well
        //assert_eq!(vec![
        //vec![LineFormat(Normal, "a".into()), LineFormat(Highlight, "s".into())],
        //vec![LineFormat(Highlight, "d".into()), LineFormat(Normal, "f".into())]
        //], r);
    }

}
