//! Provides tools for screen operations.

use core::{fmt::Write, mem};

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
    let col_num = FB_INFO.as_ref().horizontal_resolution / 8;
    let row_num = FB_INFO.as_ref().vertical_resolution / 16;
    let console = Console {
        screen: Screen::new(FB_INFO.as_ref().clone()),
        lines: VecDeque::new(),
        line: String::with_capacity(col_num),
        col: 0,
        row: 0,
        col_num,
        row_num,
    };
    CONSOLE.init(Mutex::new(console));
}

/// Console
pub struct Console {
    screen: Screen,
    lines: VecDeque<String>,
    line: String,
    col: usize,
    row: usize,
    col_num: usize,
    row_num: usize,
}

impl Console {
    fn line_break(&mut self) {
        for col in self.col..self.col_num {
            self.line.push(' ');
            self.screen.print_char(' ', (col * 8, self.row * 16));
        }
        let line = mem::replace(&mut self.line, String::with_capacity(self.col_num));
        self.lines.push_back(line);

        self.col = 0;
        if self.row < self.row_num - 1 {
            self.row += 1;
        } else {
            self.lines.pop_front();
            for (row, line) in self.lines.iter().enumerate() {
                self.screen.print_str(line, (0, row * 16));
            }
        }
    }
}

impl Write for Console {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            if c == '\n' {
                self.line_break();
            } else {
                if self.col >= self.col_num {
                    self.line_break();
                }
                self.line.push(c);
                self.col += self.screen.print_char(c, (self.col * 8, self.row * 16));
            }
        }
        Ok(())
    }
}
