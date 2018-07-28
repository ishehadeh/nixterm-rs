use failure;
use std::os::unix::io::RawFd;
use std::{fmt, result};
use terminfo;

pub type Result<T> = result::Result<T, Error>;
#[derive(Debug)]
pub struct Error {
    inner: failure::Context<ErrorKind>,
}

#[derive(Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "failed to put the terminal in raw mode")]
    InitRawModeFailed,

    #[fail(display = "failed to take terminal out of raw mode")]
    ExitRawModeFailed,

    #[fail(display = "failed to get the next character")]
    GetCharFailed,

    #[fail(display = "failed to read the next keystroke")]
    ReadKeyFailed,

    #[fail(display = "Invalid number")]
    InvalidNumber,

    #[fail(display = "Invalid color")]
    InvalidColor,

    #[fail(display = "Failed to write OS Command escape code")]
    OscFailed,

    #[fail(display = "Failed to create terminal")]
    TermInitFailed,

    #[fail(display = "Failed to write Control Sequence")]
    CsiFailed,

    #[fail(display = "Expect fg:/bg: inside [+] block")]
    InvalidColorLocation,

    #[fail(display = "Expect fg/bg inside [-] block")]
    InvalidResetSpecifier,

    #[fail(display = "Unknown color \"{}\"", _0)]
    UnknownColorName(String),

    #[fail(
        display = "Failed to get the cursor position. The terminal did not return a valid escape sequence."
    )]
    InvalidCursorPosition,

    #[fail(display = "Failed to find the with of a tab character in this terminal")]
    FailedToGetTabWidth,

    #[fail(display = "Failed to read a line from standard in")]
    ReadLineFailed,

    #[fail(display = "Failed to write to standard out")]
    FailedWriteToStdout,

    #[fail(display = "Failed to align line right")]
    FailedToAlignRight,

    #[fail(display = "Failed to align line center")]
    FailedToAlignCenter,

    #[fail(display = "A terminfo field is missing!")]
    MissingTermInfoField(terminfo::StringField),

    #[fail(display = "Failed to execute a terminfo string")]
    FailedToRunTerminfo(terminfo::StringField),
}

impl Error {
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
