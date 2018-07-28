#[macro_use]
extern crate failure;
extern crate nix;

#[macro_use]
pub mod ansi;
mod errors;
pub mod format;
pub mod rawterm;
pub mod term;
pub mod xterm;
pub use self::errors::*;
pub use format::format;
use nix::sys::termios;
pub use term::Term;
pub mod terminfo;

#[macro_export]
macro_rules! tformat {
    ($s:expr, $($args:expr),*) => {
        $crate::format(format!($s, $($args),*)).unwrap_or($s.to_owned())
    };

    ($s:expr) => {
        $crate::format($s).unwrap_or($s.to_owned())
    };
}

#[macro_export]
macro_rules! tprintln {
    ($s:expr, $($args:expr),*) => {
        println!("{}", tformat!($s, $($args),*))
    };

    ($s:expr) => {
        println!("{}", tformat!($s))
    };
}

#[macro_export]
macro_rules! tprint {
    ($s:expr, $($args:expr),*) => {
        print!("{}", tformat!($s, $($args),*))
    };

    ($s:expr) => {
        print!("{}", tformat!($s))
    };
}

#[macro_export]
macro_rules! teprintln {
    ($s:expr, $($args:expr),*) => {
        eprintln!("{}", tformat!($s, $($args),*))
    };

    ($s:expr) => {
        eprintln!("{}", tformat!($s))
    };
}

#[macro_export]
macro_rules! teprint {
    ($s:expr, $($args:expr),*) => {
        eprint!("{}", tformat!($s, $($args),*))
    };

    ($s:expr) => {
        eprint!("{}", tformat!($s))
    };
}

#[macro_export]
macro_rules! twrite {
    ($o:expr, $s:expr, $($args:expr),*) => {
        write!($o, "{}", tformat!($s, $($args),*))
    };

    ($o:expr, $s:expr) => {
        write!($o, "{}", tformat!($s))
    };
}
