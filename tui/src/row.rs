use log::*;
use std::ops::AddAssign;
use crate::*;
use editor_core::{BufferConfig, RopeGraphemes, grapheme_width,
    prev_grapheme_boundary, nth_prev_grapheme_boundary,
    nth_next_grapheme_boundary
};

#[derive(Debug, Clone)]
pub struct RowItem {
    pub cursor: Cursor,
}

impl RowItem {
    //pub fn to_string(&self) -> String {
        //let acc = String::from("");
        //let s: String = format_line(&self.cursor.line, "".into(), &self.config)
            //.iter()
            //.map(|f| f.s.clone())
            //.fold(acc, |mut acc, x| {
                //acc.push_str(&x);
                //acc
            //});
        //s
    //}

    pub fn to_line_format(&self, config: &BufferConfig, sx: usize, highlight: String) -> Vec<LineFormat> {
        debug!("to_line_format: {}: {:?}", self.cursor.simple_format(), sx);
        // get the current row of the wrapped line
        match format_wrapped(&self.cursor.line, sx, highlight, config).get(self.cursor.wrap0)
        {
            Some(row) => row.clone(),
            None => vec![],
        }
    }
}
impl PartialEq for RowItem {
    fn eq(&self, other: &Self) -> bool {
        self.cursor.line == other.cursor.line
    }
}
impl Eq for RowItem {}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RowUpdateType {
    Empty,
    Row(RowItem),
    Format(Vec<LineFormat>),
}

#[derive(Debug, Clone)]
pub struct RowUpdate {
    pub dirty: bool,
    pub item: RowUpdateType,
}
impl RowUpdate {
    pub fn to_line_format(&self, config: &BufferConfig, sx: usize, highlight: String) -> Vec<LineFormat> {
        use RowUpdateType::*;
        match &self.item {
            Row(x) => x.to_line_format(config, sx, highlight),
            Format(x) => x.clone(),
            Empty => vec![],
        }
    }
}

impl From<LineFormat> for RowUpdate {
    fn from(i: LineFormat) -> Self {
        Self {
            dirty: true,
            item: RowUpdateType::Format(vec![i]),
        }
    }
}
impl From<RowItem> for RowUpdate {
    fn from(i: RowItem) -> Self {
        Self {
            dirty: true,
            item: RowUpdateType::Row(i),
        }
    }
}
impl From<RowUpdateType> for RowUpdate {
    fn from(i: RowUpdateType) -> Self {
        Self {
            dirty: true,
            item: i,
        }
    }
}
impl Default for RowUpdate {
    fn default() -> Self {
        Self {
            dirty: true,
            item: RowUpdateType::Empty,
        }
    }
}
impl PartialEq for RowUpdate {
    fn eq(&self, other: &Self) -> bool {
        self.item == other.item
    }
}
impl Eq for RowUpdate {}
impl AddAssign for RowUpdate {
    fn add_assign(&mut self, other: Self) {
        *self = other.clone()
    }
}

