//! Provides types representing whole screen.

use core::slice;

use crate::graphics::Color;

#[cfg(not(feature = "alloc"))]
pub use self::_no_alloc::*;

#[cfg(feature = "alloc")]
pub use self::_alloc::*;

/// Represents an information of frame buffer.
#[derive(Debug, Clone)]
pub struct FrameBufferInfo {
    /// Represents the frame buffer pixel format.
    pub format: PixelFormat,
    /// Represents horizontal resolution of the frame buffer.
    pub horizontal_resolution: usize,
    /// Represents vertical resolution of the frame buffer.
    pub vertical_resolution: usize,
    /// Number of pixelf per scanline.
    pub pixels_per_scanline: usize,
    /// Address of the frame buffer.
    pub frame_buffer: usize,
}

/// Represents pixel format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// Represents that a pixel uses 32 bit whose first 8 bits, second and third represent red,
    /// green and blue intensity respectively. 8 bits left are reserved.
    Rgb,
    /// Represents that a pixel uses 32 bit whose first 8 bits, second and third represent blue,
    /// green and red intensity respectively. 8 bits left are reserved.
    Bgr,
    /// Custom pixel format.
    Bitmask,
    /// The graphics mode does not support drawing directly to the frame buffer.
    Bitonly,
}

#[cfg(not(feature = "alloc"))]
mod _no_alloc {
    use super::*;
    use crate::graphics::GrayscalePixelWrite;

    /// Represents screen painting with grayscale color.
    ///
    /// Pixelf formats [PixelFormat::Bitmask] and [PixelFormat::Bitonly] are not supported.
    #[derive(Debug)]
    pub struct GrayscaleScreen {
        info: FrameBufferInfo,
    }

    impl GrayscaleScreen {
        /// Constructs [Screen] with [FrameBufferInfo] but if `info.format` is either
        /// [PixelFromat::Bitmask] or [PixelFormat::Bitonly], causes `panic`.
        pub fn new(info: FrameBufferInfo) -> Self {
            if matches!(info.format, PixelFormat::Bitmask | PixelFormat::Bitonly) {
                panic!("PixelFormat bitmask and bitonly are not supported");
            }
            Self { info }
        }

        /// Clear the whole screen.
        pub fn clear(&mut self) {
            let buf = unsafe {
                slice::from_raw_parts_mut(
                    self.info.frame_buffer as *mut u32,
                    self.info.pixels_per_scanline * self.info.vertical_resolution,
                )
            };
            for pixel in buf {
                *pixel = 0;
            }
        }
    }

    impl GrayscalePixelWrite for GrayscaleScreen {
        fn write(&mut self, pos: (usize, usize), color: u8) -> bool {
            if !(0..self.info.pixels_per_scanline).contains(&pos.0)
                || !(0..self.info.vertical_resolution).contains(&pos.1)
            {
                return false;
            }

            // Convert grayscale intensity to RGB color.
            let color = Color::from(color).into();

            let addr = self.info.frame_buffer + (self.info.pixels_per_scanline * pos.1 + pos.0) * 4;
            unsafe { *(addr as *mut u32) = color };
            true
        }

        fn range(&self) -> (usize, usize) {
            (
                self.info.horizontal_resolution,
                self.info.vertical_resolution,
            )
        }
    }
}

#[cfg(feature = "alloc")]
mod _alloc {
    use core::mem;

    use alloc::boxed::Box;

    use super::*;
    use crate::{bitfield::BitField, graphics::PixelWrite};

    /// Represents screen painting with grayscale color.
    ///
    /// Pixelf formats [`PixelFormat::Bitmask`] and [`PixelFormat::Bitonly`] are not supported.
    #[derive(Debug)]
    pub enum Screen {
        /// Grayscale screen.
        Grayscale(ScreenBase),
        /// RGB true color screen.
        Rgb(ScreenBase),
        /// BGR true color screen.
        Bgr(ScreenBase),
    }

    impl Screen {
        /// Constructs [`Screen`] with [`FrameBufferInfo`] but if `info.format` is either
        /// [PixelFormat::Bitmask] or [PixelFormat::Bitonly], causes `panic`.
        pub fn new(info: FrameBufferInfo) -> Self {
            match info.format {
                PixelFormat::Bitmask | PixelFormat::Bitonly => {
                    panic!("PixelFormat bitmask and bitonly are not supported")
                }
                PixelFormat::Rgb => Self::Rgb(ScreenBase::new(info)),
                PixelFormat::Bgr => Self::Bgr(ScreenBase::new(info)),
            }
        }

        /// Clear the whole screen.
        pub fn clear(&mut self) {
            self.base_mut().clear();
        }

        /// Returns \(`horizontal`, `vertical`\) resolution.
        pub fn resolution(&self) -> (usize, usize) {
            self.base().resolution()
        }

        /// Returns the shared reference to the [`ScreenBase`] of `self`.
        fn base(&self) -> &ScreenBase {
            match self {
                Self::Rgb(base) | Self::Bgr(base) | Self::Grayscale(base) => base,
            }
        }

        /// Returns the exclusive reference to the [`ScreenBase`] of `self`.
        fn base_mut(&mut self) -> &mut ScreenBase {
            match self {
                Self::Rgb(base) | Self::Bgr(base) | Self::Grayscale(base) => base,
            }
        }
    }

    impl PixelWrite for Screen {
        fn write(&mut self, pos: (usize, usize), color: impl Into<Color>) -> bool {
            let range = PixelWrite::range(self);
            if pos.0 >= range.0 || pos.1 >= range.1 {
                return false;
            }

            let offset = self.base().pixels_per_scanline * pos.1 + pos.0;
            let color: Color = color.into();
            match self {
                Self::Rgb(base) => {
                    let pixel = unsafe { base.fb.get_unchecked_mut(offset) };
                    pixel.set_bits(..8, color.r as _);
                    pixel.set_bits(8..16, color.g as _);
                    pixel.set_bits(16..24, color.b as _);
                }
                Self::Bgr(base) => base.fb[offset] = color.into(),
                Self::Grayscale(base) => {
                    let pixel = unsafe { base.fb.get_unchecked_mut(offset) };
                    let color = (color.r as u32 + color.g as u32 + color.b as u32) / 3;
                    let color = color << 16 | color << 8 | color;
                    *pixel = color;
                }
            }
            true
        }

        fn range(&self) -> (usize, usize) {
            let base = self.base();
            let vert_res = base.fb.len() / base.pixels_per_scanline;
            (base.pixels_per_scanline, vert_res)
        }
    }

    /// Holding the frame buffer and its size of a screen..
    #[derive(Debug)]
    pub struct ScreenBase {
        fb: Box<[u32]>,
        horizontal_resolution: usize,
        pixels_per_scanline: usize,
    }

    impl ScreenBase {
        /// Constructs [Screen] with [FrameBufferInfo] but if `info.format` is either
        fn new(info: FrameBufferInfo) -> Self {
            let horizontal_resolution = info.horizontal_resolution;
            let pixels_per_scanline = info.pixels_per_scanline;
            let fb_slice = unsafe {
                slice::from_raw_parts_mut(
                    info.frame_buffer as *mut u32,
                    info.pixels_per_scanline * info.vertical_resolution,
                )
            };
            let raw_fb = fb_slice as *mut _;
            let fb = unsafe { Box::from_raw(raw_fb) };
            Self {
                fb,
                horizontal_resolution,
                pixels_per_scanline,
            }
        }

        /// Clear the whole screen.
        fn clear(&mut self) {
            for pixel in self.fb.iter_mut() {
                *pixel = 0;
            }
        }

        /// Returns \(`horizontal`, `vertical`\) resolution.
        fn resolution(&self) -> (usize, usize) {
            let vert_res = self.fb.len() / self.pixels_per_scanline;
            (self.horizontal_resolution, vert_res)
        }
    }

    impl Drop for ScreenBase {
        fn drop(&mut self) {
            // Since frame buffer must not be dropped, leak it.
            let fb = mem::replace(&mut self.fb, Box::new([]));
            Box::leak(fb);
        }
    }
}
