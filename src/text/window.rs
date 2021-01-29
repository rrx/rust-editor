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
    pub main: RenderBlock,
    cursor: RenderCursor,
    pub buffers: BufferList,
    w: usize,
    h: usize,
    rx: channel::Receiver<EditorWindowUpdate>,
    tx: channel::Sender<EditorWindowUpdate>,
    rx_app: channel::Receiver<Command>,
    tx_app: channel::Sender<Command>
}

#[derive(Debug, Clone)]
pub enum EditorWindowUpdate {
    Header(Vec<RowUpdate>),
    Status(Vec<RowUpdate>),
    Command(Vec<RowUpdate>),
    Left(Vec<RowUpdate>),
    Main(Vec<RowUpdate>),
    Cursor(usize, usize)
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
            cursor: RenderCursor::default(),
            buffers: BufferList::default(),
            tx,
            rx,
            tx_app,
            rx_app
        }._update_size(w, h)
    }
    fn _update_size(mut self, w: usize, h: usize) -> Self {
        self.update_size(w, h);
        self
    }

    fn update_size(&mut self, w: usize, h: usize) {
        self.w = w;
        self.h = h;
        // header on the top row
        self.header.update_view(w, 1, 0, 0);
        self.status.update_view(w, 1, 0, h-2);
        self.command.update_view(w, 1, 0, h-1);
        self.left.update_view(6, h-3, 0, 1);
        self.main.update_view(w - 6, h-3, 6, 1);

        self.buffers.resize(self.main.w, self.main.h, self.main.x0, self.main.y0);
    }

    pub fn get_channel(&self) -> channel::Sender<EditorWindowUpdate> {
        self.tx.clone()
    }

    pub fn get_app_channel(&self) -> channel::Sender<Command> {
        self.tx_app.clone()
    }


    fn refresh(&mut self, out: &mut std::io::Stdout) {
        let mut commands = self.header.generate_commands();
        commands.append(&mut self.status.generate_commands());
        commands.append(&mut self.command.generate_commands());
        commands.append(&mut self.left.generate_commands());
        commands.append(&mut self.main.generate_commands());
        commands.append(&mut self.cursor.generate_commands());
        render_commands(out, commands);
    }

    fn buffer_update(&mut self) {
        let b = self.buffers.get();
        self.header.update_rows(b.header_updates());
        self.left.update_rows(b.left_updates());
        self.status.update_rows(b.status_updates());
        self.main.update_rows(b.get_updates().clone());
        self.command.update_rows(b.command_updates());
        self.cursor.update(b.cx + b.x0, b.cy + b.y0);
    }

    pub fn events(&mut self, save_tx: channel::Sender<Command>) {
        let mut out = std::io::stdout();
        render_reset(&mut out);

        // initial update of the window
        // get buffers
        let mut b = self.buffers.get_mut();
        b.update_view();
        //b.send_updates(&self.tx);
        self.buffer_update();
        self.refresh(&mut out);

        loop {
            channel::select! {
                recv(self.rx_app) -> r => {
                    use Command::*;
                    match r {
                        Ok(c) => {
                            match c {
                                Quit => break,
                                Save => {
                                    info!("Save");
                                    // get the buffer and send it off to the save thread
                                    let b = self.buffers.get();
                                    save_tx.send(Command::SaveBuffer(b.path.clone(), b.text.clone())).unwrap();
                                }
                                Resize(x, y) => {
                                    info!("Resize: {:?}", (x, y));
                                    // update the size of the window
                                    self.update_size(x as usize, y as usize);

                                    // generate diff
                                    self.buffer_update();
                                }
                                _ => {
                                    info!("Command: {:?}", c);
                                    self.buffers.command(&c);
                                    //let b = self.buffers.get();
                                    self.buffer_update();
                                    //b.send_updates(&self.tx);
                                }
                            }
                            self.refresh(&mut out);
                        }
                        Err(e) => {
                            error!("{:?}", e);
                        }
                    }
                }

                //recv(self.rx) -> r => {
                    //use EditorWindowUpdate::*;
                    //match r {
                        //Ok(msg) => {
                            //match msg {
                                //Header(v) => self.header.update_rows(v),
                                //Status(v) => self.status.update_rows(v),
                                //Command(v) => self.command.update_rows(v),
                                //Left(v) => self.left.update_rows(v),
                                //Main(v) => {
                                    //self.main.update_rows(v);
                                    //self.refresh(&mut out);
                                //}
                                //Cursor(x, y) => {
                                    //self.cursor.update(x, y);
                                    //info!("Cursor:{:?}", (x, y));
                                    //self.cursor.update(x, y);
                                    //self.refresh(&mut out);
                                //}
                            //}
                        //}
                        //Err(e) => {
                            //error!("{:?}", e);
                        //}
                    //}
                //}
            }
        }
    }

}
