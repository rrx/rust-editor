#[derive(Debug, Clone)]
pub struct ViewSpec {
    pub w: u16, // width of view
    pub h: u16, // height of view
    pub origin_x: u16,
    pub origin_y: u16,
    pub header: u16, // header rows
    pub footer: u16, // footer rows
    pub status: u16, // status rows
    pub lm: u16, // left margin
    pub rm: u16, // right margin
    pub sx: u16, // horizontal size for body
    pub sy: u16, // vertical size for body
    pub x0: u16, // x origin for body
    pub y0: u16, // y origin for body
}

impl ViewSpec {
    pub fn new(w: u16, h: u16, origin_x: u16, origin_y: u16) -> Self {
        let header = 1;
        let footer = 1;
        let status = 1;
        let lm = 6;
        let rm = 1;
        let s = Self {
            w: w,
            h: h,
            origin_x: origin_x,
            origin_y: origin_y,
            header: header,
            footer: footer,
            status: status,
            lm: lm,
            rm: rm,
            sx: 0,
            sy: 0,
            x0: 0,
            y0: 0
        };
        s.init()
    }

    fn init(mut self) -> Self {
        self.calc();
        self
    }

    pub fn resize(&mut self, w: u16, h: u16, origin_x: u16, origin_y: u16) {
        self.w = w;
        self.h = h;
        self.origin_x = origin_x;
        self.origin_y = origin_y;
        self.calc();
    }

    pub fn calc(&mut self) {
        self.sx = self.w - self.lm - self.rm;
        self.sy = self.h - self.header - self.footer - self.status;
        self.x0 = self.origin_x + self.lm;
        self.y0 = self.origin_y + self.header;
    }

}



