//! Provides tools for screen operations.

use core::iter;

use alloc::{collections::VecDeque, format, string::String};
use util::{
    asmfunc,
    bitfield::BitField,
    graphics::{Color, Print as _},
    screen::{FrameBufferInfo, Screen},
    sync::OnceStatic,
};

use crate::sync::Mutex;

/// Frame buffer information initialized at the begining of kernel.
pub static FB_INFO: OnceStatic<FrameBufferInfo> = OnceStatic::new();

pub static STRINGS: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

pub fn drawing_task() {
    let mut console = Console {
        screen: Screen::new(FB_INFO.as_ref().clone()),
        lines: VecDeque::new(),
        _col: 0,
        row: 0,
        col_num: FB_INFO.as_ref().horizontal_resolution / 8,
        row_num: FB_INFO.as_ref().vertical_resolution / 16,
    };
    let mut color = 0u32;
    loop {
        let mut strings = STRINGS.lock();
        while let Some(s) = strings.pop_front() {
            let r = 13;
            let g = 19;
            let b = 29;
            color += r << 16 | g << 8 | b;
            color &= 0xffffff;
            console.draw_str(s, color);
        }
        drop(strings);
        asmfunc::hlt();
    }
}

struct Console {
    screen: Screen,
    lines: VecDeque<String>,
    _col: usize,
    row: usize,
    col_num: usize,
    row_num: usize,
}

impl Console {
    fn draw_str(&mut self, s: impl Into<String>, color: impl Into<Color>) {
        let color = color.into();
        let s: String = s.into();
        for line in s.lines() {
            self.draw_line(line.chars(), color);
        }
    }

    fn draw_line(&mut self, line: impl IntoIterator<Item = char>, color: impl Into<Color>) {
        let color = color.into();
        let line = line
            .into_iter()
            .chain(iter::repeat(' '))
            .take(self.col_num)
            .collect();
        if self.row == self.row_num - 1 {
            self.lines.pop_front();
            self.lines.push_back(line);
            for (row, line) in self.lines.iter().enumerate() {
                self.screen.print_str(line, (0, row * 16), color);
            }
        } else {
            self.screen.print_str(&line, (0, self.row * 16), color);
            self.lines.push_back(line);
            self.row += 1;
        }
    }
}
