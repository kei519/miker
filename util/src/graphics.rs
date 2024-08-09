//! Provides graphical supports.

use crate::{bitfield::BitField as _, font_data};

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

/// Represents true color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
    /// Red intensity.
    pub r: u8,
    /// Green intensity.
    pub g: u8,
    /// Blue intensity.
    pub b: u8,
}

impl From<u32> for Color {
    fn from(value: u32) -> Self {
        Self {
            r: value.get_bits(16..24) as _,
            g: value.get_bits(8..16) as _,
            b: value.get_bits(..8) as _,
        }
    }
}

impl From<Color> for u32 {
    fn from(value: Color) -> Self {
        (value.r as u32) << 16 | (value.g as u32) << 8 | (value.b as u32)
    }
}

impl From<u8> for Color {
    fn from(value: u8) -> Self {
        Self {
            r: value,
            g: value,
            b: value,
        }
    }
}

/// A trait for painting pixels with true color.
pub trait PixelWrite {
    /// Paints `pos` with `color` if `pos` is in the range.
    ///
    /// Returns if `pos` is in the range.
    ///
    /// # Arguments
    ///
    /// * pos - (x, y).
    /// * color - true color.
    fn write(&mut self, pos: (usize, usize), color: impl Into<Color>) -> bool;

    /// Returns writable area (width, height).
    fn range(&self) -> (usize, usize);
}

impl<T: PixelWrite> GrayscalePixelWrite for T {
    fn write(&mut self, pos: (usize, usize), color: u8) -> bool {
        let color: Color = color.into();
        PixelWrite::write(self, pos, color)
    }

    fn range(&self) -> (usize, usize) {
        PixelWrite::range(self)
    }
}

/// A trait for printing string with true color.
pub trait Print: PixelWrite {
    /// Prints `string` at `pos` (x, y).
    fn print_str(&mut self, s: &str, pos: (usize, usize), color: impl Into<Color>) {
        let color = color.into();
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
                col += Print::print_char(self, c, (pos.0 + col * 8, pos.1 + row * 16), color);
            }
            row += 1;
        }
    }

    /// Prints `c` at `pos` (x, y) and returns the width used by `c`, in 8 px.
    fn print_char(&mut self, c: char, pos: (usize, usize), color: impl Into<Color>) -> usize {
        let color: Color = color.into();

        if c.is_ascii() {
            for (y, &byte) in font_data::get_font(c as _).iter().enumerate() {
                for (x, color) in (0..9)
                    .map(|i| {
                        if byte >> (7 - i) & 1 != 0 {
                            color
                        } else {
                            0u32.into()
                        }
                    })
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
            Print::print_char(self, '?', pos, color)
                + Print::print_char(self, '?', (pos.0 + 8, pos.1), color)
        }
    }
}

impl<T: PixelWrite> Print for T {}
