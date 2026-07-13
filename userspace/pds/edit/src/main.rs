#![no_std]
#![no_main]

use lerux_interface_types::{
    EditRequest, EditResponse, FsRequest, FsResponse, MAX_EDIT_LINES, MAX_EDIT_LINE_LEN,
    MAX_FS_PATH,
};
use lerux_ipc::{recv, send, send_unspecified_error, FsClient};
use lerux_logging::{debug, log};
use sel4_microkit::{protection_domain, Channel, Handler, Infallible, MessageInfo};

const FS_SERVER: FsClient = FsClient::new(Channel::new(1));
const SHELL: Channel = Channel::new(0);

/// Simple fixed-size editor buffer. Cursor is always valid.
struct Editor {
    path: [u8; MAX_FS_PATH],
    path_len: u8,
    lines: [[u8; MAX_EDIT_LINE_LEN]; MAX_EDIT_LINES],
    lens: [u8; MAX_EDIT_LINES],
    num_lines: u8,
    cur_row: u8,
    cur_col: u8,
    modified: bool,
}

impl Default for Editor {
    fn default() -> Self {
        let mut s = Self {
            path: [0; MAX_FS_PATH],
            path_len: 0,
            lines: [[0; MAX_EDIT_LINE_LEN]; MAX_EDIT_LINES],
            lens: [0; MAX_EDIT_LINES],
            num_lines: 1,
            cur_row: 0,
            cur_col: 0,
            modified: false,
        };
        // start with one empty line
        s.lens[0] = 0;
        s
    }
}

impl Editor {
    fn set_path(&mut self, p: &[u8]) {
        self.path_len = p.len().min(MAX_FS_PATH) as u8;
        self.path[..self.path_len as usize].copy_from_slice(&p[..self.path_len as usize]);
    }

    fn path_bytes(&self) -> &[u8] {
        &self.path[..self.path_len as usize]
    }

    fn clear(&mut self) {
        self.lines = [[0; MAX_EDIT_LINE_LEN]; MAX_EDIT_LINES];
        self.lens = [0; MAX_EDIT_LINES];
        self.num_lines = 1;
        self.cur_row = 0;
        self.cur_col = 0;
        self.modified = false;
    }

    /// Load from FS into buffer. On error, start empty. Very small files only.
    fn load_from_fs(&mut self) {
        self.clear();
        if self.path_len == 0 {
            return;
        }
        let handle = match fs_call(FsRequest::open(self.path_bytes())) {
            FsResponse::Handle { id } => id,
            _ => {
                // new file: keep empty buffer, mark modified so save creates it
                self.modified = true;
                return;
            }
        };

        let mut offset = 0u32;
        let mut total_lines = 0u8;
        let mut cur_line = 0usize;
        let mut col = 0usize;

        loop {
            match fs_call(FsRequest::Read {
                handle,
                offset,
                len: 128,
            }) {
                FsResponse::Data { data_len, data } if data_len > 0 => {
                    let n = data_len as usize;
                    for &b in &data[..n] {
                        if b == b'\n' || b == b'\r' {
                            // finish current line
                            if total_lines < MAX_EDIT_LINES as u8 {
                                self.lens[total_lines as usize] = col as u8;
                                total_lines += 1;
                            }
                            cur_line = total_lines as usize;
                            col = 0;
                            if b == b'\r' {
                                // ignore possible following \n on next iter
                            }
                        } else if cur_line < MAX_EDIT_LINES && col < MAX_EDIT_LINE_LEN {
                            self.lines[cur_line][col] = b;
                            col += 1;
                        }
                    }
                    offset += data_len as u32;
                }
                _ => break,
            }
        }
        // finalize last line
        if (total_lines as usize) < MAX_EDIT_LINES {
            self.lens[total_lines as usize] = col as u8;
            total_lines += 1;
        }
        self.num_lines = total_lines.max(1);
        self.cur_row = 0;
        self.cur_col = 0;
        self.modified = false;
    }

    fn insert_char(&mut self, ch: u8) {
        if (self.cur_row as usize) >= MAX_EDIT_LINES {
            return;
        }
        let row = self.cur_row as usize;
        let mut col = self.cur_col as usize;
        let mut len = self.lens[row] as usize;

        if col > len {
            col = len;
        }
        if len >= MAX_EDIT_LINE_LEN {
            return; // truncate at line limit
        }

        // shift right
        for i in (col..len).rev() {
            self.lines[row][i + 1] = self.lines[row][i];
        }
        self.lines[row][col] = ch;
        len += 1;
        self.lens[row] = len as u8;
        self.cur_col = (col + 1) as u8;
        self.modified = true;
    }

    fn backspace(&mut self) {
        let row = self.cur_row as usize;
        let col = self.cur_col as usize;
        if col == 0 {
            if row == 0 {
                return;
            }
            // join with prev line
            let prev = row - 1;
            let prev_len = self.lens[prev] as usize;
            let this_len = self.lens[row] as usize;
            if prev_len + this_len <= MAX_EDIT_LINE_LEN {
                for i in 0..this_len {
                    self.lines[prev][prev_len + i] = self.lines[row][i];
                }
                self.lens[prev] = (prev_len + this_len) as u8;
                // remove this row by shifting up
                for r in row..(self.num_lines as usize - 1) {
                    self.lines[r] = self.lines[r + 1];
                    self.lens[r] = self.lens[r + 1];
                }
                self.lens[(self.num_lines - 1) as usize] = 0;
                self.num_lines -= 1;
                self.cur_row = (row - 1) as u8;
                self.cur_col = prev_len as u8;
                self.modified = true;
            }
            return;
        }
        // delete char before cursor in line
        let len = self.lens[row] as usize;
        for i in (col - 1)..(len - 1) {
            self.lines[row][i] = self.lines[row][i + 1];
        }
        self.lines[row][len - 1] = 0;
        self.lens[row] = (len - 1) as u8;
        self.cur_col = (col - 1) as u8;
        self.modified = true;
    }

    fn newline(&mut self) {
        if (self.num_lines as usize) >= MAX_EDIT_LINES {
            return;
        }
        let row = self.cur_row as usize;
        let col = self.cur_col as usize;
        let len = self.lens[row] as usize;

        // split line at col
        let mut new_len = 0usize;
        for i in col..len {
            self.lines[row + 1][new_len] = self.lines[row][i];
            new_len += 1;
        }
        for i in col..len {
            self.lines[row][i] = 0;
        }
        self.lens[row] = col as u8;
        self.lens[row + 1] = new_len as u8;

        // shift lower lines down
        for r in ((row + 2)..(self.num_lines as usize)).rev() {
            self.lines[r] = self.lines[r - 1];
            self.lens[r] = self.lens[r - 1];
        }
        self.num_lines += 1;
        self.cur_row += 1;
        self.cur_col = 0;
        self.modified = true;
    }

    fn move_left(&mut self) {
        if self.cur_col > 0 {
            self.cur_col -= 1;
        } else if self.cur_row > 0 {
            self.cur_row -= 1;
            self.cur_col = self.lens[self.cur_row as usize];
        }
    }

    fn move_right(&mut self) {
        let len = self.lens[self.cur_row as usize];
        if (self.cur_col as usize) < (len as usize) {
            self.cur_col += 1;
        } else if (self.cur_row as usize + 1) < (self.num_lines as usize) {
            self.cur_row += 1;
            self.cur_col = 0;
        }
    }

    fn move_up(&mut self) {
        if self.cur_row > 0 {
            self.cur_row -= 1;
            let len = self.lens[self.cur_row as usize];
            if self.cur_col > len {
                self.cur_col = len;
            }
        }
    }

    fn move_down(&mut self) {
        if (self.cur_row as usize + 1) < (self.num_lines as usize) {
            self.cur_row += 1;
            let len = self.lens[self.cur_row as usize];
            if self.cur_col > len {
                self.cur_col = len;
            }
        }
    }

    fn save(&mut self) -> bool {
        if self.path_len == 0 {
            return false;
        }
        // build flat content with \n
        let mut buf = [0u8; 512]; // small files only
        let mut pos = 0usize;
        for i in 0..(self.num_lines as usize) {
            let l = self.lens[i] as usize;
            if pos + l + 1 > buf.len() {
                break;
            }
            buf[pos..pos + l].copy_from_slice(&self.lines[i][..l]);
            pos += l;
            if i + 1 < self.num_lines as usize {
                buf[pos] = b'\n';
                pos += 1;
            }
        }
        let data = &buf[..pos];

        let handle = match fs_call(FsRequest::create(self.path_bytes())) {
            FsResponse::Handle { id } => id,
            _ => return false,
        };
        match fs_call(FsRequest::write(handle, 0, data)) {
            FsResponse::Ok => {
                self.modified = false;
                true
            }
            _ => false,
        }
    }

    fn view(&self) -> EditResponse {
        let mut view_path = [0u8; MAX_FS_PATH];
        let n = self.path_len as usize;
        view_path[..n].copy_from_slice(&self.path[..n]);
        EditResponse::View {
            path_len: self.path_len,
            path: view_path,
            line_count: self.num_lines,
            line_lens: self.lens,
            lines: self.lines,
            cursor_row: self.cur_row,
            cursor_col: self.cur_col,
            modified: self.modified,
        }
    }
}

fn fs_call(req: FsRequest) -> FsResponse {
    FS_SERVER.call(req)
}

#[protection_domain]
fn init() -> HandlerImpl {
    debug::init().unwrap();
    log::info!("lerux-edit: ready");
    HandlerImpl {
        ed: Editor::default(),
    }
}

struct HandlerImpl {
    ed: Editor,
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        if channel != SHELL {
            return Ok(send_unspecified_error());
        }

        Ok(match recv::<EditRequest>(msg_info) {
            Ok(req) => {
                let resp = match req {
                    EditRequest::Open { path_len, path } => {
                        let p = &path[..path_len as usize];
                        self.ed.set_path(p);
                        self.ed.load_from_fs();
                        // return view immediately
                        self.ed.view()
                    }
                    EditRequest::InsertChar(ch) => {
                        self.ed.insert_char(ch);
                        self.ed.view()
                    }
                    EditRequest::Backspace => {
                        self.ed.backspace();
                        self.ed.view()
                    }
                    EditRequest::Newline => {
                        self.ed.newline();
                        self.ed.view()
                    }
                    EditRequest::MoveLeft => {
                        self.ed.move_left();
                        self.ed.view()
                    }
                    EditRequest::MoveRight => {
                        self.ed.move_right();
                        self.ed.view()
                    }
                    EditRequest::MoveUp => {
                        self.ed.move_up();
                        self.ed.view()
                    }
                    EditRequest::MoveDown => {
                        self.ed.move_down();
                        self.ed.view()
                    }
                    EditRequest::Save => {
                        if self.ed.save() {
                            EditResponse::Ok
                        } else {
                            EditResponse::Error
                        }
                    }
                    EditRequest::GetView => self.ed.view(),
                    EditRequest::Quit => EditResponse::Ok,
                };
                send(resp)
            }
            Err(_) => send_unspecified_error(),
        })
    }
}
