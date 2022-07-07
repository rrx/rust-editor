use super::*;
use log::*;

#[derive(Debug)]
pub enum DrawCommand {
    Clear(usize, usize),
    Line(u16, usize, String),
    Row(u16, u16, String),
    Status(u16, String),
    Cursor(u16, u16),
    Format(usize, usize, usize, Vec<LineFormat>),
    SavePosition,
    RestorePosition,
}

#[derive(Debug, Clone)]
pub struct RenderCursor {
    pub cx: usize,
    pub cy: usize,
    dirty: bool,
}
impl Default for RenderCursor {
    fn default() -> Self {
        Self {
            cx: 0,
            cy: 0,
            dirty: true,
        }
    }
}
impl RenderCursor {
    pub fn update(&mut self, cx: usize, cy: usize) {
        if self.cx != cx {
            self.cx = cx;
            self.dirty = true;
        }
        if self.cy != cy {
            self.cy = cy;
            self.dirty = true;
        }
    }
    pub fn generate_commands(&mut self) -> Vec<DrawCommand> {
        if self.dirty {
            vec![DrawCommand::Cursor(self.cx as u16, self.cy as u16)]
        } else {
            vec![]
        }
    }

    pub fn clear(&mut self) -> &mut Self {
        self.dirty = true;
        self
    }
}

// render block should be able to handle all text orientations
// this is the abstraction that handles all 8 text orientations
// From an api perspective it's Right-Left, Top-Bottom, but that
// will be possible to change in the future
#[derive(Debug, Clone)]
pub struct RenderBlock {
    pub w: usize,  // width of the block
    pub h: usize,  // height of the block
    pub x0: usize, // x-coordinate of the top corner
    pub y0: usize, // y-coordinate of the top corner
    rows: Vec<RowUpdate>,
    pub highlight: String,
}

impl Default for RenderBlock {
    fn default() -> Self {
        Self {
            w: 0,
            h: 0,
            x0: 0,
            y0: 0,
            rows: vec![],
            highlight: "".to_string(),
        }
    }
}

impl RenderBlock {
    pub fn set_highlight(&mut self, h: String) -> &mut Self {
        self.highlight = h;
        self
    }

    pub fn clear(&mut self) -> &mut Self {
        self.rows.truncate(0);
        self
    }

    pub fn resize(&mut self, w: usize, h: usize, x0: usize, y0: usize) -> &mut Self {
        self.w = w;
        self.h = h;
        self.x0 = x0;
        self.y0 = y0;
        // reset everything on resize
        self.rows.truncate(0);
        self.rows.resize_with(self.h, RowUpdate::default);
        self
    }

    pub fn update_rows(&mut self, rows: Vec<RowUpdate>) -> &mut Self {
        debug!("update_rows {:?}", (rows.len(), self.rows.len()));
        self.rows.resize_with(rows.len(), RowUpdate::default);
        self.rows
            .iter_mut()
            .zip(rows.iter())
            .enumerate()
            .for_each(|(_i, (left, right))| {
                if left != right {
                    left.dirty = true;
                    left.formats = right.formats.clone();
                }
            });
        self
    }

    pub fn generate_commands(&mut self) -> Vec<DrawCommand> {
        let y0 = self.y0;
        let x0 = self.x0;
        let w = self.w;
        let mut cs: Vec<DrawCommand> = self
            .rows
            .iter_mut()
            .enumerate()
            .filter_map(|(inx, r)| {
                if r.dirty {
                    r.dirty = false;
                    return Some(DrawCommand::Format(x0, y0 + inx, w, r.formats.clone()));
                }
                None
            })
            .collect();
        if cs.len() > 0 {
            cs.insert(0, DrawCommand::SavePosition);
            cs.push(DrawCommand::RestorePosition);
        }
        cs
    }
}
