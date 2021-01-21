#[derive(Debug)]
pub struct ViewPort {
    pub char_start: usize,
    pub char_current: usize,
    pub char_end: usize,
}
impl Default for ViewPort {
    fn default() -> Self {
        Self {
            char_start: 0,
            char_end: 0,
            char_current: 0
        }
    }
}


