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
    fn print(&mut self, string: &str, pos: (usize, usize)) {
        let (width, height) = self.range();
        let col_num = width / 8;
        let row_num = height / 16;
        for (row, line) in string.lines().enumerate().take(row_num) {
            for (col, c) in line.chars().enumerate().take(col_num) {
                for (y, &bit) in font_data::get_font(c as _).iter().enumerate() {
                    for (x, color) in (0..8)
                        .map(|i| if bit >> (7 - i) & 1 != 0 { 0xff } else { 0x00 })
                        .enumerate()
                    {
                        let pos = (pos.0 + col * 8 + x, pos.1 + row * 16 + y);
                        self.write(pos, color);
                    }
                }
            }
        }
    }
}

impl<T: GrayscalePixelWrite> GrayscalePrint for T {}
