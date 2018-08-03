#[macro_use]
extern crate failure;
extern crate nix;

#[macro_use]
pub mod ansi;
mod errors;
pub mod events;
pub mod term;
pub mod terminfo;
mod util;
pub mod xterm;

pub use self::errors::*;
pub use term::Term;
