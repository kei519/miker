//! Provides graphical supports.

use crate::font_data;

/// A trait for painting pixels with grayscale.
pub trait GrayscalePixelWrite {
    /// Paints `pos` with `color` if `pos` is in the range.
    ///
    /// Returns if `pos` is in the range.
    ///
    /// # Arguments
    ///
    /// * pos - (x, y).
    /// * color - grayscale intensity.
    fn write(&mut self, pos: (usize, usize), color: u8) -> bool;

    /// Returns writable area (width, height).
    fn range(&self) -> (usize, usize);
}

/// A trait for printing string with grayscale color.
pub trait GrayscalePrint: GrayscalePixelWrite {
    /// Prints `string` at `pos` (x, y).
    fn print_str(&mut self, s: &str, pos: (usize, usize)) {
        let (width, height) = self.range();
        let col_num = width / 8;
        let row_num = (height - pos.1) / 16;
        let mut row = 0;
        let mut col = 0;
        for line in s.lines() {
            if row >= row_num {
                break;
            }
            for c in line.chars() {
                if col >= col_num {
                    break;
                }
                col += self.print_char(c, (pos.0 + col * 8, pos.1 + row * 16));
            }
            row += 1;
        }
    }

    /// Prints `c` at `pos` (x, y) and returns the width used by `c`, in 8 px.
    fn print_char(&mut self, c: char, pos: (usize, usize)) -> usize {
        if c.is_ascii() {
            for (y, &byte) in font_data::get_font(c as _).iter().enumerate() {
                for (x, color) in (0..9)
                    .map(|i| if byte >> (7 - i) & 1 != 0 { 0xff } else { 0x00 })
                    .enumerate()
                {
                    let pos = (pos.0 + x, pos.1 + y);
                    if !self.write(pos, color) {
                        return 0;
                    };
                }
            }
            1
        } else {
            self.print_char('?', pos) + self.print_char('?', (pos.0 + 8, pos.1))
        }
    }
}

impl<T: GrayscalePixelWrite> GrayscalePrint for T {}
