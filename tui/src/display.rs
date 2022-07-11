use super::*;
use editor_core::ViewPos;
use log::*;

#[derive(Debug, Clone)]
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
    commands: Vec<DrawCommand>,
}
impl Default for RenderCursor {
    fn default() -> Self {
        Self {
            cx: 0,
            cy: 0,
            commands: vec![DrawCommand::Cursor(0, 0)],
        }
    }
}
impl RenderCursor {
    pub fn update(&mut self, cx: usize, cy: usize) {
        if self.cx != cx {
            self.cx = cx;
            self.commands
                .push(DrawCommand::Cursor(self.cx as u16, self.cy as u16));
        }
        if self.cy != cy {
            self.cy = cy;
            self.commands
                .push(DrawCommand::Cursor(self.cx as u16, self.cy as u16));
        }
    }

    pub fn generate_commands(&mut self) -> Vec<DrawCommand> {
        self.commands.drain(..).collect()
    }

    pub fn clear(&mut self) -> &mut Self {
        self.commands.truncate(0);
        self.commands
            .push(DrawCommand::Cursor(self.cx as u16, self.cy as u16));
        self
    }
}

// render block should be able to handle all text orientations
// this is the abstraction that handles all 8 text orientations
// From an api perspective it's Right-Left, Top-Bottom, but that
// will be possible to change in the future
#[derive(Debug, Clone)]
pub struct RenderBlock {
    pub view: ViewPos,
    rows: Vec<RowUpdate>,
    pub highlight: String,
    commands: Vec<DrawCommand>,
}

impl RenderBlock {
    pub fn new(view: ViewPos) -> Self {
        Self {
            view,
            rows: vec![],
            highlight: "".into(),
            commands: vec![],
        }
    }

    pub fn set_highlight(&mut self, h: String) -> &mut Self {
        self.highlight = h;
        self
    }

    pub fn clear(&mut self) -> &mut Self {
        self.rows.truncate(0);
        self
    }

    pub fn resize(&mut self, view: ViewPos) -> &mut Self {
        // reset everything on resize
        self.rows.truncate(0);
        self.rows.resize_with(view.h, RowUpdate::default);
        self.view = view;
        self
    }

    pub fn update_rows(&mut self, rows: Vec<RowUpdate>) -> &mut Self {
        debug!("update_rows {:?}", (rows.len(), self.rows.len()));
        self.rows.resize_with(rows.len(), RowUpdate::default);
        let mut commands = self
            .rows
            .iter_mut()
            .zip(rows.iter())
            .enumerate()
            .filter_map(|(inx, (left, right))| {
                if left != right {
                    left.formats = right.formats.clone();
                    Some(DrawCommand::Format(
                        self.view.x0,
                        self.view.y0 + inx,
                        self.view.w,
                        left.formats.clone(),
                    ))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        self.commands.append(&mut commands);
        self
    }

    pub fn generate_commands(&mut self) -> Vec<DrawCommand> {
        if self.commands.len() > 0 {
            self.commands.insert(0, DrawCommand::SavePosition);
            self.commands.push(DrawCommand::RestorePosition);
        }
        self.commands.drain(..).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn display() {
        // update status
        let view = ViewPos {
            w: 10,
            h: 2,
            x0: 0,
            y0: 0,
        };
        let mut block = RenderBlock::new(view);
        let s = format!("test: {}", 1);
        block.update_rows(vec![RowUpdate::from(LineFormat::new(
            LineFormatType::Highlight,
            s,
        ))]);

        let commands = block.generate_commands();
        println!("{:?}", commands);
    }
}
