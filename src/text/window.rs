use log::*;
use super::*;
use crossbeam::channel;

// this might be dynamic at some point to handle tiling windows,
// or multiple window layouts as needed
// For now it's a hardwired layout
pub struct EditorWindow {
    header: RenderBlock,
    status: RenderBlock,
    command: RenderBlock,
    left: RenderBlock,
    main: RenderBlock,
    w: usize,
    h: usize,
    rx: channel::Receiver<EditorWindowUpdate>,
    tx: channel::Sender<EditorWindowUpdate>,
    rx_app: channel::Receiver<Msg>,
    tx_app: channel::Sender<Msg>
}

pub enum EditorWindowUpdate {
    Header(Vec<RowItem>),
    Status(Vec<RowItem>),
    Command(Vec<RowItem>),
    Left(Vec<RowItem>),
    Main(Vec<RowItem>)
}

impl EditorWindow {
    pub fn new(w: usize, h: usize) -> Self {
        let (tx, rx) = channel::unbounded();
        let (tx_app, rx_app) = channel::unbounded();
        Self {
            w, h,
            header: RenderBlock::default(),
            status: RenderBlock::default(),
            command: RenderBlock::default(),
            left: RenderBlock::default(),
            main: RenderBlock::default(),
            tx,
            rx,
            tx_app,
            rx_app
        }.update_size(w, h)
    }

    fn update_size(mut self, w: usize, h: usize) -> Self {
        self.w = w;
        self.h = h;
        // header on the top row
        self.header.update_view(w, 1, 0, 0);
        self.status.update_view(w, 1, 0, h-2);
        self.command.update_view(w, 1, 0, h-1);
        self.left.update_view(6, h-3, 0, 1);
        self.main.update_view(w - 6, h-3, 6, 1);
        self
    }

    pub fn get_channel(&self) -> channel::Sender<EditorWindowUpdate> {
        self.tx.clone()
    }

    pub fn get_app_channel(&self) -> channel::Sender<Msg> {
        self.tx_app.clone()
    }


    fn refresh(&mut self, out: &mut std::io::Stdout) {
        //render_commands(out, self.header.generate_commands());
        render_commands(out, self.main.generate_commands());
    }

    pub fn events(&mut self) {
        use EditorWindowUpdate::*;
        let mut out = std::io::stdout();
        render_reset(&mut out);

        // initial refresh
        self.refresh(&mut out);

        loop {
            channel::select! {
                recv(self.rx_app) -> r => {
                    match r {
                        Ok(msg) => {
                            match msg {
                                Msg::Quit => break,
                                _ => ()
                            }
                        }
                        Err(e) => {
                            error!("{:?}", e);
                        }
                    }
                }

                recv(self.rx) -> r => {
                    match r {
                        Ok(msg) => {
                            match msg {
                                //Header(v) => self.header.update_rows(v),
                                Main(v) => {
                                    self.main.update_rows(v);
                                    self.refresh(&mut out);
                                }
                                _ => ()

                            }
                        }
                        Err(e) => {
                            error!("{:?}", e);
                        }
                    }
                }
            }
        }
    }

}
