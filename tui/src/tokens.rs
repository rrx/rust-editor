use ropey::Rope;

pub struct TokenIterator<'a> {
    text: &'a Rope,
}

impl<'a> Iterator for TokenIterator<'a> {
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
    fn new(line: &'a String, inx: usize, highlight: String, config: BufferConfig) -> Self {
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

