use failure;
use std::{fmt, result};

pub type Result<T> = result::Result<T, Error>;

/// TermInfo's failure type.
///
/// This is a thin wrapper around a `failure::Context`.
/// To get what actually happened call `Error::kind` to get an `ErrorKind`,
/// or `<Error as failure::Fail>::cause` to get the underlying error, if there is one (in the case of this library, there usually is).
#[derive(Debug)]
pub struct Error {
    inner: failure::Context<ErrorKind>,
}

/// Outlines the various points where TermInfo routines may fail.
///
/// ErrorKind will almost always be wrapped in an `Error`, and
/// generally it will be won't make much sense without that error's cause.
#[derive(Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "could not find a valid path to the terminfo file.")]
    FailedToFindTermInfo,

    #[fail(display = "failed to parse the terminfo file.")]
    FailedToParseFile,

    #[fail(display = "this file is not a terminfo file (bad magic number)")]
    InvalidMagicNumber,

    #[fail(display = "this file is not a terminfo file (too short)")]
    IncompleteTermInfo,

    #[fail(display = "the string table has exceeded its maximum capacity for a 16bit file")]
    MaxStrTabSizeReached,

    #[fail(display = "failed to read from index {}, there are only {} elements!", _0, _1)]
    OutOfRange(usize, usize),

    #[fail(display = "the file is too short to fit a terminfo header")]
    IncompleteTermInfoHeader,

    #[fail(display = "failed to read a string from a string table")]
    FailedToReadStringFromTable,

    #[fail(
        display = "The file is too short to fit any terminfo extended data, but too long to be only a standard terminfo file."
    )]
    IncompleteExtendedTermInfo,

    #[fail(
        display = "The file is too short to fit any terminfo extended header, but too long to be only a standard terminfo file."
    )]
    IncompleteExtendedHeader,

    #[fail(
        display = "maximum capability count exceeded, there can only be a maximum of 65535 capabilities in each array for 16bit files"
    )]
    MaximumCapabilityCountExceeded,

    #[fail(display = "invalid printf format specifier")]
    BadPrintfSpecifier,

    #[fail(display = "invalid precision number in printf specifier")]
    BadPrecisionSpecified,

    #[fail(display = "invalid digit in number")]
    InvalidDigit(u8),

    #[fail(display = "failed to write a string argument")]
    FailedToWriteArgument,

    #[fail(display = "invalid argument identifier")]
    InvalidArgumentIdentifier,

    #[fail(display = "unexpected argument type, expected a {}, got a(n) {}", _0, _1)]
    UnexpectedArgumentType(&'static str, &'static str),

    #[fail(display = "failed to write string literal")]
    FailedToWriteStringLiteral,

    #[fail(display = "unexpected EOF")]
    UnexpectedEof,
}

impl Error {
    /// Get this error's specific kind.
    pub fn kind(&self) -> &ErrorKind {
        self.inner.get_context()
    }
}

impl failure::Fail for Error {
    fn cause(&self) -> Option<&failure::Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&failure::Backtrace> {
        self.inner.backtrace()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error {
            inner: failure::Context::new(kind),
        }
    }
}

impl From<failure::Context<ErrorKind>> for Error {
    fn from(inner: failure::Context<ErrorKind>) -> Error {
        Error { inner: inner }
    }
}
