/// A trait for painting pixels with grayscale.
pub trait GlayscalePixelWrite {
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
