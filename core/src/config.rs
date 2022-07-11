// From: https://github.com/mathphreak/mfte/blob/master/src/config.rs
use editorconfig::get_config;
use log::*;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum IndentStyle {
    Tab,
    Space,
}

#[derive(Debug, Clone)]
pub enum IndentSize {
    Size(u8),
    Tab,
}

#[derive(Debug, Clone)]
pub enum EndOfLine {
    Lf,
    CrLf,
    Cr,
}

#[derive(Debug, Clone)]
pub enum Charset {
    Latin1,
    UTF8,
    UTF16BE,
    UTF16LE,
}

#[derive(Debug, Clone)]
pub struct BufferConfig {
    pub indent_style: IndentStyle,
    pub indent_size: IndentSize,
    pub tab_width: u8,
    pub end_of_line: EndOfLine,
    pub charset: Charset,
    pub trim_trailing_whitespace: bool,
    pub insert_final_newline: bool,
}

#[derive(Debug, Clone)]
pub struct ViewPos {
    pub w: usize,  // width of the block
    pub h: usize,  // height of the block
    pub x0: usize, // x-coordinate of the top corner
    pub y0: usize, // y-coordinate of the top corner
}

impl ViewPos {
    pub fn new() -> Self {
        Self {
            w: 0,
            h: 0,
            x0: 0,
            y0: 0,
        }
    }
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self::config_for(None)
    }
}

fn normalize_path(maybe_path: Option<&str>) -> Option<PathBuf> {
    match maybe_path {
        Some(p) => {
            let path = Path::new(p);
            match path.canonicalize() {
                Ok(c_path) => Some(PathBuf::from(c_path)),
                Err(err) => {
                    error!("Error on path: {:?}", err);
                    None
                }
            }
        }
        None => None,
    }
}

impl BufferConfig {
    pub fn config_tabs() -> Self {
        Self {
            // sensible editor defaults
            indent_style: IndentStyle::Tab,
            indent_size: IndentSize::Tab,
            tab_width: 8,
            end_of_line: EndOfLine::Lf,
            charset: Charset::UTF8,
            trim_trailing_whitespace: true,
            insert_final_newline: true,
        }
    }

    pub fn config_spaces(spaces: u8) -> Self {
        Self {
            // sensible editor defaults
            indent_style: IndentStyle::Space,
            indent_size: IndentSize::Size(spaces),
            tab_width: spaces,
            end_of_line: EndOfLine::Lf,
            charset: Charset::UTF8,
            trim_trailing_whitespace: true,
            insert_final_newline: true,
        }
    }

    pub fn config_for(path: Option<&str>) -> Self {
        let mut result = Self {
            // sensible editor defaults
            indent_style: IndentStyle::Space,
            indent_size: IndentSize::Size(4),
            tab_width: 4,
            end_of_line: EndOfLine::Lf,
            charset: Charset::UTF8,
            trim_trailing_whitespace: true,
            insert_final_newline: true,
        };
        if let Some(p) = normalize_path(path) {
            let result_conf = get_config(&p);
            info!("config: {:?}", (path, &result_conf));
            match result_conf {
                Err(err) => error!("Error: {:?}", err),
                Ok(conf) => {
                    if let Some(style) = conf.get("indent_style") {
                        if style == "tab" {
                            result.indent_style = IndentStyle::Tab;
                        } else if style == "space" {
                            result.indent_style = IndentStyle::Space;
                        }
                    }

                    if let Some(size) = conf.get("indent_size") {
                        if size == "tab" {
                            result.indent_size = IndentSize::Tab;
                        } else {
                            if let Ok(size) = size.parse() {
                                result.indent_size = IndentSize::Size(size);
                            }
                        }
                    }

                    if let Some(width) = conf.get("tab_width") {
                        if let Ok(width) = width.parse() {
                            result.tab_width = width;
                        }
                    }

                    if let Some(eol) = conf.get("end_of_line") {
                        if eol == "cr" {
                            result.end_of_line = EndOfLine::Cr;
                        } else if eol == "crlf" {
                            result.end_of_line = EndOfLine::CrLf;
                        } else if eol == "lf" {
                            result.end_of_line = EndOfLine::Lf;
                        }
                    }

                    if let Some(charset) = conf.get("charset") {
                        if charset == "latin1" {
                            result.charset = Charset::Latin1;
                        } else if charset == "utf-8" {
                            result.charset = Charset::UTF8;
                        } else if charset == "utf-16be" {
                            result.charset = Charset::UTF16BE;
                        } else if charset == "utf-16le" {
                            result.charset = Charset::UTF16LE;
                        }
                    }

                    if let Some(ttw) = conf.get("trim_trailing_whitespace") {
                        if ttw == "true" {
                            result.trim_trailing_whitespace = true;
                        } else if ttw == "false" {
                            result.trim_trailing_whitespace = false;
                        }
                    }

                    if let Some(ifn) = conf.get("insert_final_newline") {
                        if ifn == "true" {
                            result.insert_final_newline = true;
                        } else if ifn == "false" {
                            result.insert_final_newline = false;
                        }
                    }
                }
            }
        }
        info!("config: {:?}", (&result));
        result
    }

    pub fn indent(&self) -> String {
        match self.indent_style {
            IndentStyle::Tab => "\t".to_string(),
            IndentStyle::Space => {
                let spaces = match self.indent_size {
                    IndentSize::Tab => self.tab_width,
                    IndentSize::Size(n) => n,
                } as usize;
                " ".repeat(spaces)
            }
        }
    }

    pub fn line_sep(&self) -> &str {
        match self.end_of_line {
            EndOfLine::Lf => "\n",
            EndOfLine::CrLf => "\r\n",
            EndOfLine::Cr => "\r",
        }
    }
}
