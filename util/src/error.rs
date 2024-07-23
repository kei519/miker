//! Provides tools for error logging.

use alloc::boxed::Box;
use core::fmt::{Debug, Display};

/// MIKer result type.
pub type Result<T> = core::result::Result<T, Error>;

/// Returns directyly [`Err()`] of [`Error`].
#[macro_export]
macro_rules! error {
    ($err:expr) => {
        return Err($crate::error::Error {
            ty: ::alloc::boxed::Box::new($err),
            file: ::core::file!(),
            line: ::core::line!(),
        });
    };
    () => {
        error!($crate::error::PhantomError);
    };
}

/// Not pecified error type.
#[derive(Debug)]
pub struct PhantomError;

impl Display for PhantomError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(self, f)
    }
}

/// Error info containing [`ErrorType`], file name and line number where error occurs.
pub struct Error {
    /// Error type that implements [`ErrorType`].
    pub ty: Box<dyn ErrorType + 'static>,
    /// File name where the error occurs.
    pub file: &'static str,
    /// Line number where the error occurs.
    pub line: u32,
}

impl Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} at {}: {}", self.file, self.line, self.ty)
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} at {}: {:?}", self.file, self.line, self.ty)
    }
}

/// Requirements for [`Error::ty`].
pub trait ErrorType: Debug + Display {}

impl<T: Debug + Display> ErrorType for T {}
