use log::*;
use std::ops::AddAssign;
use crate::*;
use editor_core::{BufferConfig, RopeGraphemes, grapheme_width,
    prev_grapheme_boundary, nth_prev_grapheme_boundary,
    nth_next_grapheme_boundary
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RowUpdateType {
    Format(Vec<LineFormat>),
}

#[derive(Debug, Clone)]
pub struct RowUpdate {
    pub dirty: bool,
    pub formats: Vec<LineFormat>,
}
impl RowUpdate {
    pub fn from_formats(formats: Vec<LineFormat>) -> Self {
        Self {
            dirty: true,
            formats,
        }
    }
}

impl From<LineFormat> for RowUpdate {
    fn from(i: LineFormat) -> Self {
        Self {
            dirty: true,
            formats: vec![i],
        }
    }
}

impl Default for RowUpdate {
    fn default() -> Self {
        Self {
            dirty: true,
            formats: vec![],
        }
    }
}
impl PartialEq for RowUpdate {
    fn eq(&self, other: &Self) -> bool {
        self.formats == other.formats
    }
}

impl Eq for RowUpdate {}

impl AddAssign for RowUpdate {
    fn add_assign(&mut self, other: Self) {
        *self = other.clone()
    }
}

