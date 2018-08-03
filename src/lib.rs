#[macro_use]
extern crate failure;
extern crate nix;

#[macro_use]
pub mod ansi;
mod errors;
pub mod term;
mod util;
pub mod xterm;
pub use self::errors::*;
pub use term::Term;
pub mod events;
pub mod terminfo;
