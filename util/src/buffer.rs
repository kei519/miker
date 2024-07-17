//! Provides format string under no [alloc] environments.

use core::fmt::{Error, Write};

/// Provides [Write] implemnted buffer with a given limited buffer.
pub struct StrBuf<'buf> {
    buf: &'buf mut [u8],
    pos: usize,
}

impl<'buf> StrBuf<'buf> {
    /// Construct [StrBuf] with `buf`.
    pub fn new(buf: &'buf mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// Converts bytes string in the inner buffer into `&`[`str`].
    pub fn to_str(&self) -> &str {
        // Safety: Since `buf[..pos]` bytes string is stored through `Write::write_str()`,
        //      `buf[..pos]` is a valid UTF-8 string. However returned value is valid only when
        //      `StrBuf` is taking care of inner bytes string. This means that this method should
        //      NOT return reference to `&str` bound `buf lifetime.
        unsafe { core::str::from_utf8_unchecked(&self.buf[..self.pos]) }
    }
}

impl<'buf> Write for StrBuf<'buf> {
    /// When trying to write string whose length runs over the inner buffer, writes string within
    /// the limit and returns an [Error].
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let (len, ret) = if self.buf.len() - self.pos >= s.len() {
            (s.len(), Ok(()))
        } else {
            (self.buf.len() - self.pos, Err(Error))
        };

        for b in s.bytes().take(len) {
            self.buf[self.pos] = b;
            self.pos += 1;
        }
        ret
    }
}
