use super::*;
use log::*;

pub struct Editor {
    header: RenderBlock,
    cmd_block: BufferBlock,
    pub layout: WindowLayout,
    registers: Registers,
    variables: Variables,
    highlight: String,
    w: usize,
    h: usize,
    x0: usize,
    y0: usize,
    pub terminal: Terminal,
}

impl Default for Editor {
    fn default() -> Self {
        let layout = WindowLayout::default();
        Self {
            header: RenderBlock::default(),
            cmd_block: BufferBlock::new(FileBuffer::from_string(&"".to_string())),
            layout: layout,
            registers: Registers::default(),
            variables: Variables::default(),
            highlight: String::new(),
            w: 10,
            h: 10,
            x0: 0,
            y0: 0,
            terminal: Terminal::default(),
        }
    }
}

impl Editor {
    pub fn clear(&mut self) -> &mut Self {
        self.header.clear();
        self.cmd_block.clear();
        self.layout.clear();
        self
    }

    pub fn update(&mut self) -> &mut Self {
        let b = self.layout.get();
        let text = b.main.get_text();
        let path = b.main.get_path();
        let cursor = &b.main.cursor;
        let s = format!(
            "Rust-Editor-{} {} {} Line:{}/{}{:width$}",
            clap::crate_version!(),
            path,
            cursor.simple_format(),
            cursor.line_inx + 1,
            text.len_lines(),
            width = b.main.w
        );

        self.header.update_rows(vec![RowUpdate::from(LineFormat(
            LineFormatType::Highlight,
            s,
        ))]);
        self.layout.get_mut().update();
        self
    }

    pub fn update_cmd_normal(&mut self) -> &mut Self {
        self.cmd_block.update();
        // render command line
        let line = self.cmd_block.get_text();
        self.cmd_block
            .left
            .update_rows(vec![RowUpdate::from(LineFormat(
                LineFormatType::Normal,
                ">> ".to_string(),
            ))]);
        self.cmd_block
            .block
            .update_rows(vec![RowUpdate::from(LineFormat(
                LineFormatType::Normal,
                format!("{:width$}", line, width = self.cmd_block.block.w),
            ))]);
        self
    }

    pub fn generate_commands(&mut self) -> Vec<DrawCommand> {
        let mut out = self.layout.get_mut().generate_commands();
        out.append(&mut self.header.generate_commands());
        out.append(&mut self.cmd_block.generate_commands());
        out
    }

    pub fn add_window(&mut self, fb: LockedFileBuffer) {
        let mut bufwin = BufferWindow::from(fb);
        bufwin.resize(self.w, self.h - 2, self.x0, self.y0 + 1);
        bufwin.main.set_focus(true);
        self.layout.add(bufwin);
    }

    pub fn resize(&mut self, w: usize, h: usize, x0: usize, y0: usize) {
        info!("Resize: {}/{}", w, h);
        self.w = w;
        self.h = h;
        self.x0 = x0;
        self.y0 = y0;
        self.header.resize(w, 1, x0, y0);
        self.layout.resize(w, h - 2, x0, y0 + 1);
        self.cmd_block.resize(w, 1, x0, y0 + h - 1, 3);
    }

    pub fn get_command_line(&self) -> String {
        self.cmd_block.buf.read().text.line(0).to_string()
    }

    pub fn command_cancel(&mut self) -> &mut Self {
        self.command_reset()
    }

    pub fn command_update(&mut self) -> &mut Self {
        let line = self.get_command_line();
        if line.len() > 1 {
            let (first, last) = line.split_at(1);
            match first {
                "/" | "?" => {
                    self.highlight = last.to_string();
                    self.layout
                        .get_mut()
                        .main
                        .clear()
                        .block
                        .set_highlight(last.to_string());
                }
                _ => (),
            }
        } else {
            self.highlight.truncate(0);
        }
        self.update_cmd_normal();
        self
    }

    pub fn search_update(&mut self, s: String) -> &mut Self {
        self.layout
            .get_mut()
            .main
            .block
            .set_highlight(s.to_string());
        self
    }

    pub fn command_exec(&mut self) -> &mut Self {
        let line = self.get_command_line();
        info!("EXEC: {}", line);

        if line.len() > 1 {
            let (first, last) = line.split_at(1);
            match first {
                "/" => {
                    self.search_update(last.to_string());
                    self.layout
                        .get_mut()
                        .main
                        .search(last, false)
                        .search_next(0)
                        .update();
                    self.layout
                        .get_mut()
                        .main
                        .clear()
                        .block
                        .set_highlight(last.to_string());
                    self.command_reset();
                }
                "?" => {
                    self.search_update(last.to_string());
                    self.layout
                        .get_mut()
                        .main
                        .search(last, true)
                        .search_next(0)
                        .update();
                    self.layout
                        .get_mut()
                        .main
                        .clear()
                        .block
                        .set_highlight(last.to_string());
                    self.command_reset();
                }
                ":" => {
                    self.command_reset();
                    //self.cmd_block.generate_commands();
                    match command_parse(last) {
                        Ok(commands) => {
                            info!("command parse: {:?}", commands);
                            commands.iter().for_each(|c| {
                                self.command(&c);
                            });
                            self.update();
                        }
                        Err(err) => {
                            error!("command parse: {:?}", err);
                            self.command_output(&String::from("ERROR"));
                            //self.cmd_block.replace_text("ERROR");
                            //self.command_reset();
                            self.update();
                        }
                    }
                }
                _ => (),
            }
        }
        self
    }

    pub fn command_output(&mut self, s: &String) -> &mut Self {
        self.cmd_block
            .left
            .update_rows(vec![RowUpdate::from(LineFormat(
                LineFormatType::Normal,
                "OUT".to_string(),
            ))]);
        self.cmd_block
            .block
            .update_rows(vec![RowUpdate::from(LineFormat(
                LineFormatType::Bold,
                format!("{:width$}", s, width = self.cmd_block.block.w),
            ))]);
        self
    }

    pub fn command_reset(&mut self) -> &mut Self {
        self.cmd_block.reset_buffer().update();
        self.cmd_block.set_focus(false);
        self.layout.get_mut().main.set_focus(true);
        self.update_cmd_normal();
        self
    }

    pub fn command(&mut self, c: &Command) -> &mut Self {
        use Command::*;
        match c {
            BufferNext => {
                self.layout.next().get_mut().clear().update();
                self.layout
                    .get_mut()
                    .main
                    .block
                    .set_highlight(self.highlight.clone());
                let path = self.layout.buffers.get().main.get_path();
                info!("Next: {}", path);
            }
            BufferPrev => {
                self.layout.prev().get_mut().clear().update();
                self.layout
                    .get_mut()
                    .main
                    .block
                    .set_highlight(self.highlight.clone());
                let path = self.layout.buffers.get().main.get_path();
                info!("Prev: {}", path);
            }
            Insert(x) => {
                self.layout.get_mut().main.insert_char(*x).update();
            }
            Join => {
                self.layout.get_mut().main.join_line().update();
            }
            Delete(reps, m) => {
                self.layout.get_mut().main.delete_motion(m, *reps).update();
            }
            Yank(reg, m) => {
                self.registers
                    .update(reg, &self.layout.get_mut().main.motion_slice(m));
                self.update();
            }
            Paste(reps, reg, m) => {
                let s = self.registers.get(reg);
                self.layout
                    .get_mut()
                    .main
                    .paste_motion(m, &s, *reps)
                    .update();
            }
            RemoveChar(dx) => {
                self.layout.get_mut().main.remove_range(*dx).update();
            }
            Motion(reps, m) => {
                self.layout.get_mut().main.motion(m, *reps).update();
            }
            CliEdit(cmds) => {
                self.cmd_block.set_focus(true);
                self.layout.get_mut().main.set_focus(false);
                for c in cmds {
                    self.cmd_block.command(&c);
                }
                self.command_update().update();
            }
            CliExec => {
                self.command_update().command_exec().update();
            }
            CliCancel => {
                self.command_cancel().update();
            }
            //SearchInc(s) => {
            //self.highlight = s.clone();
            //self.layout.get_mut().main.clear().block.set_highlight(s.clone());
            //}
            //Search(s) => {
            //self.highlight = s.clone();
            //self.layout.get_mut().main.search(s.as_str()).search_next(0).update();
            //self.layout.get_mut().main.clear().block.set_highlight(s.clone());
            //}
            ScrollPage(ratio) => {
                let bw = self.layout.get();
                let xdy = bw.main.w as f32 / *ratio as f32;
                self.layout
                    .get_mut()
                    .main
                    .scroll(xdy as i32)
                    .update_from_start();
            }
            Scroll(dy) => {
                self.layout
                    .get_mut()
                    .main
                    .scroll(*dy as i32)
                    .update_from_start();
            }
            Line(line_number) => {
                let line_inx = line_number - 1;
                self.layout
                    .get_mut()
                    .main
                    .cursor_move_line(line_inx)
                    .update();
            }
            LineNav(dx) => {
                self.layout.get_mut().main.cursor_move_lc(*dx).update();
            }
            Resize(x, y) => {
                self.resize(*x as usize, *y as usize, self.x0, self.y0);
            }
            Mouse(x, y) => {
                let bw = self.layout.get_mut();
                match bw.main.cursor_from_xy(*x as usize, *y as usize) {
                    Some(c) => {
                        bw.main.cursor_move(c); //.update();
                    }
                    _ => (),
                }
            }

            VarGet(s) => {
                let v = self.variables.get(&Variable(s.clone()));
                self.command_output(&format!("get {} = {}", s, v)).update();
            }
            VarSet(a, b) => {
                let k = Variable(a.clone());
                let v = self.variables.update(&k, b);
                self.command_output(&format!("set {} = {}", a, b)).update();
            }

            Quit => {
                info!("Quit");
                self.terminal.cleanup();
                //signal_hook::low_level::raise(signal_hook::consts::signal::SIGHUP).unwrap();
            }
            Refresh => {
                info!("Refresh");
                self.terminal.enter_raw_mode();
                let (sx, sy) = crossterm::terminal::size().unwrap();
                self.resize(sx as usize, sy as usize, 0, 0);
                self.clear().update();
                self.cmd_block.set_focus(false);
                self.layout.get_mut().main.set_focus(true);
            }

            Resume => {
                info!("Resume");
                self.terminal.enter_raw_mode();
                let (sx, sy) = crossterm::terminal::size().unwrap();
                self.resize(sx as usize, sy as usize, 0, 0);
                self.clear().update();
            }

            Stop => {
                info!("Stop");
                self.terminal.leave_raw_mode();
                //use std::{io::stdout, time::Duration};
                //use nix::sys::signal;
                //use libc;

                //std::thread::sleep(std::time::Duration::from_millis(1000));
                //Duration
                //self.terminal.toggle();
                //self.toggle_terminal();
                //let mut out = std::io::stdout();
                //if self.in_terminal {
                //execute!(out, terminal::LeaveAlternateScreen).unwrap();
                //println!("{}", char::from_u32(0x001a).unwrap());
                signal_hook::low_level::raise(signal_hook::consts::signal::SIGSTOP).unwrap();
                //signal_hook::low_level::raise(signal_hook::consts::signal::SIGTSTP).unwrap();
                //signal_hook::low_level::raise(signal_hook::consts::signal::SIGSTOP).unwrap();
                //low_level::emulate_default_handler(SIGSTOP).unwrap();
                //} else {
                //execute!(out, terminal::EnterAlternateScreen).unwrap();
                //self.clear().update();
                //}
                //self.in_terminal = !self.in_terminal;
                //terminal_cleanup();
                //signal_hook::low_level::raise(signal_hook::consts::signal::SIGTSTP).unwrap();
                //self.command(&Command::Resume);
                //signal_hook::low_level::raise(signal_hook::consts::signal::SIGTSTP).unwrap();
                //println!("{}", char::from_u32(0x001a).unwrap());
                //low_level::emulate_default_handler(signal_hook::consts::signal::SIGTSTP).unwrap();
                //low_level::raise(signal_hook::consts::signal::SIGTSTP).unwrap();
            }
            _ => {
                error!("Not implemented: {:?}", c);
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_1() {
        let mut e = Editor::default();
        let fb1 = FileBuffer::from_string(&"".to_string());
        let fb2 = FileBuffer::from_string(&"".to_string());
        e.add_window(fb1.clone());
        e.add_window(fb2.clone());
        e.add_window(fb2.clone());
        e.resize(100, 20, 0, 0);

        use Command::*;
        let cs = vec![
            Insert('x'),
            BufferNext,
            Insert('y'),
            BufferNext,
            Insert('z'),
        ];
        cs.iter().for_each(|c| {
            e.command(c);
        });
        info!("A: {:?}", &fb1);
        info!("B: {:?}", &fb2);
        info!("C: {:?}", &mut e.layout.get_mut().generate_commands());
    }
}
