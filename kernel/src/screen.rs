//! Provides tools for screen operations.

use core::iter;

use alloc::{collections::VecDeque, string::String};
use util::{
    graphics::GrayscalePrint as _,
    screen::{FrameBufferInfo, Screen},
    sync::OnceStatic,
};

use crate::sync::Mutex;

/// Frame buffer information initialized at the begining of kernel.
pub static FB_INFO: OnceStatic<FrameBufferInfo> = OnceStatic::new();

/// Console to display information.
pub static CONSOLE: OnceStatic<Mutex<Console>> = OnceStatic::new();

pub fn init() {
    let console = Console {
        screen: Screen::new(FB_INFO.as_ref().clone()),
        lines: VecDeque::new(),
        _col: 0,
        row: 0,
        col_num: FB_INFO.as_ref().horizontal_resolution / 8,
        row_num: FB_INFO.as_ref().vertical_resolution / 16,
    };
    CONSOLE.init(Mutex::new(console));
}

/// Provides the mean displaying information.
pub struct Console {
    screen: Screen,
    lines: VecDeque<String>,
    _col: usize,
    row: usize,
    col_num: usize,
    row_num: usize,
}

impl Console {
    pub fn draw_str(&mut self, s: impl Into<String>) {
        let s: String = s.into();
        for line in s.lines() {
            self.draw_line(line.chars());
        }
    }

    fn draw_line(&mut self, line: impl IntoIterator<Item = char>) {
        let line = line
            .into_iter()
            .chain(iter::repeat(' '))
            .take(self.col_num)
            .collect();
        if self.row == self.row_num - 1 {
            self.lines.pop_front();
            self.lines.push_back(line);
            for (row, line) in self.lines.iter().enumerate() {
                self.screen.print_str(line, (0, row * 16));
            }
        } else {
            self.screen.print_str(&line, (0, self.row * 16));
            self.lines.push_back(line);
            self.row += 1;
        }
    }
}
