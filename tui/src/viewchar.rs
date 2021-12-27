use editor_core::{EndOfLine, grapheme_width, BufferConfig, IndentStyle, IndentSize};
use unicode_segmentation::{Graphemes, UnicodeSegmentation};
use LineFormatType::*;

pub struct LineFormatFactory<'a> {
    config: &'a BufferConfig,
    sx: usize
}

impl<'a> LineFormatFactory<'a> {
    pub fn new(config: &'a BufferConfig, sx: usize) -> Self {
        LineFormatFactory { config, sx }
    }

    pub fn create(&self, format: LineFormatType, s: String) -> LineFormat {
        LineFormat::new(format, s)
    }

    pub fn format(&self, s: String) -> Vec<LineFormat> {
        vec![]
    }

}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LineFormat {
    pub format: LineFormatType,
    pub s: String,
}

impl LineFormat {
    pub fn new(format: LineFormatType, s: String) -> Self {
        LineFormat { format, s }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum LineFormatType {
    Dim,
    Normal,
    Highlight,
    Bold,
}

#[derive(Debug)]
pub struct FormatItem {
    pub unicode_width: usize,
    pub s: String,
    pub format: LineFormatType,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ViewChar {
    Grapheme(String, u8),
    Tab(u8, u8),
    Control(String, u8),
    NL(u8),
}

impl ViewChar {
    pub fn char_len(&self) -> u8 {
        match self {
            Self::Tab(_, char_len) => *char_len,
            Self::NL(char_len) => *char_len,
            Self::Control(_, char_len) => *char_len,
            Self::Grapheme(_, char_len) => *char_len
        }
    }

    pub fn format(&self) -> String {
        match self {
            Self::Tab(size,_) => format_tab(*size), // right arrow
            Self::NL(_) => format_newline(), // paragraph symbol
            Self::Control(v, _) => format_control(&v),
            Self::Grapheme(s, _) => s.to_string(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ViewCharWithSize {
    viewchar: ViewChar,
    s: String,
    width: usize,
    char_len: usize,
    r: usize,
    lc: usize
}
impl ViewCharWithSize {
    pub fn new(viewchar: ViewChar, r: usize, lc: usize) -> Self {
        let s = viewchar.format();
        let width = grapheme_width(&s);
        let char_len = viewchar.char_len() as usize;
        Self { viewchar, s: s.to_string(), width, char_len: char_len, r, lc }
    }

    pub fn to_format_item(&self) -> FormatItem {
        let s = self.viewchar.format();
        let width = grapheme_width(&s);

        let format = match self.viewchar {
            ViewChar::Tab(_,_) => Dim,
            ViewChar::NL(_) => Dim,
            ViewChar::Control(_,_) => Dim,
            ViewChar::Grapheme(_,_) => Normal
        };

        FormatItem {
            s: s.to_string(),
            unicode_width: width,
            format,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ViewCharCollection {
    elements: Vec<ViewCharWithSize>,
    current_r: usize,
    current_lc: usize
}

impl Default for ViewCharCollection {
    fn default() -> Self {
        ViewCharCollection { elements: vec![], current_r: 0, current_lc: 0 }
    }
}

fn add_size(viewchar: ViewChar, r: usize, lc: usize) -> ViewCharWithSize {
    ViewCharWithSize::new(viewchar, r, lc)
}

impl ViewCharCollection {
    pub fn unicode_width(&self) -> usize {
        self.unicode_width_range(0,self.elements.len())
    }

    pub fn unicode_width_range(&self, start: usize, end: usize) -> usize {
        self.elements.as_slice()[start..end].iter().fold(0, |acc, e| acc + e.width)
    }

    pub fn char_length(&self) -> usize {
        self.char_length_range(0, self.elements.len())
    }

    pub fn lc_to_r(&self, lc: usize) -> usize {
        match self.elements.iter().find(|v| v.lc >= lc) {
            Some(v) => v.r,
            None => 0
        }
    }

    pub fn r_to_lc(&self, r: usize) -> usize {
        match self.elements.iter().find(|v| v.r >= r) {
            Some(v) => v.lc,
            None => 0
        }
    }

    pub fn char_length_range(&self, start: usize, end: usize) -> usize {
        let mut a = start;
        if a >= self.elements.len() {
            a = self.elements.len();
        }
        let mut b = end;
        if b > self.elements.len() {
            b = self.elements.len();
        }

        self.elements.as_slice()[a..b].iter().fold(0, |acc, e| acc + e.char_len)
    }

    pub fn elements_range(&self, start: usize, end: usize) -> Vec<ViewChar> {
        self.elements.as_slice()[start..end]
            .iter()
            .map(|v| v.viewchar.clone())
            .collect()
    }

    pub fn append(&mut self, v: &mut Vec<ViewChar>) {
        v.iter().for_each(|x| self.push(x.clone()));
    }

    pub fn push(&mut self, v: ViewChar) {
        let s = v.format();
        let char_len = v.char_len() as usize;
        self.elements.push(add_size(v, self.current_r, self.current_lc));
        self.current_r += grapheme_width(&s);
        self.current_lc += char_len;
    }

}

fn expand_tab(config: &BufferConfig) -> Vec<ViewChar> {
    use ViewChar::*;
    match config.indent_style {
        IndentStyle::Tab => {
            let mut v = Vec::new();
            v.push(Tab(config.tab_width, 1));
            v
        }
        IndentStyle::Space => {
            let spaces = match config.indent_size {
                IndentSize::Tab => config.tab_width,
                IndentSize::Size(n) => n,
            } as u8;
            let mut v = Vec::new();
            v.push(Tab(spaces, spaces));
            v
        }
    }
}

fn expand_newline(config: &BufferConfig) -> Vec<ViewChar> {
    use ViewChar::*;
    match config.end_of_line {
        EndOfLine::Lf => vec![NL(1)],
        EndOfLine::CrLf => vec![NL(2)],
        EndOfLine::Cr => vec![NL(1)],
    }
}

// covert a string into a viewcharcollection
pub fn string_to_elements(s: &String, config: &BufferConfig) -> ViewCharCollection {
    use ViewChar::*;
    s.graphemes(true).fold(ViewCharCollection::default(), |mut v, c| match c {
        
        "\t" => {
            v.append(&mut expand_tab(config));
            v
        }
        "\n" => {
            v.append(&mut expand_newline(config));
            v
        }
        _ => {
            let maybe_first = c.chars().next();
            if c.chars().count() == 1 && maybe_first.unwrap().is_ascii_control() {
                let first = maybe_first.unwrap();
                v.push(Control(first.to_string(), 1));
            } else {
                v.push(Grapheme(c.to_string(), 1));//c.len() as u8));
            }
            v
        }
    })
}

fn format_tab(tab_size: u8) -> String {
    let mut s = " ".repeat(tab_size as usize - 1);
    s.push_str("\u{2192}"); // right arrow
    s
}

fn format_newline() -> String {
    "\u{00B6}".to_string()
}

fn format_control(s: &str) -> String {
    s.chars().map(|ch| {
        format!("{}", ch.escape_unicode())
    }).collect::<Vec<String>>().join("")
}

pub fn grapheme_to_format_item(ch: &str, config: &BufferConfig, highlight: bool) -> Vec<FormatItem> {
    let elements = string_to_elements(&ch.to_string(), config);
    elements.elements.iter().map(|v| {
        let mut f = v.to_format_item();
        if highlight {
            f.format = Highlight;
        }
        f
    }).collect::<Vec<FormatItem>>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use LineFormatType::*;

    #[test]
    fn test_control_characters() {
        let config = BufferConfig::default();
        let mut line = String::from("");
        line.push(char::from(13));
        let e = string_to_elements(&line, &config);
        println!("2:{:?}", (line, e));
    }
}

