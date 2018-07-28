mod errors;
mod fields;
pub mod lang;
mod terminfo;
mod terminfobuf;

mod util;

pub use self::errors::*;
pub use self::fields::*;
pub use self::terminfo::*;
pub use self::terminfobuf::*;

pub use self::BooleanField::*;
pub use self::NumericField::*;
pub use self::StringField::*;

use failure::ResultExt;
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

/// Enumerate any know terminfo databases on the system.
pub fn databases() -> Vec<PathBuf> {
    let mut dbs = Vec::new();

    if let Ok(terminfo) = env::var("TERMINFO") {
        dbs.push(PathBuf::from(terminfo))
    }

    if let Some(home) = env::home_dir() {
        dbs.push(home.join(".terminfo"))
    }

    if let Ok(dirs) = env::var("TERMINFO_DIRS") {
        dbs.extend(
            dirs.split(":")
                .filter(|p| !p.is_empty())
                .map(|p| PathBuf::from(p)),
        );
    }

    dbs.push(PathBuf::from("/usr/share/terminfo"));
    dbs
}

/// Get a path to the terminfo file base on the `$TERM` environment variable.
///
/// This function emulates the `curses` method for finding the compiled terminfo file.
/// This method is explained in detail in `terminfo.5`.
pub fn path() -> Option<PathBuf> {
    let terminal_name = match env::var("TERM") {
        Ok(v) => {
            if v.is_empty() {
                return None;
            } else {
                v
            }
        }
        Err(_) => return None,
    };

    let suffix = PathBuf::from(&terminal_name[..1]).join(terminal_name);
    databases()
        .iter()
        .find(|p| p.join(&suffix).exists())
        .map(|p| p.join(suffix))
}

pub fn from_env() -> Result<TermInfoBuf> {
    let path = match path() {
        Some(v) => v,
        None => return Err(ErrorKind::FailedToFindTermInfo.into()),
    };

    let mut file = File::open(path).context(ErrorKind::FailedToParseFile)?;
    let mut data = Vec::new();

    file.read_to_end(&mut data)
        .context(ErrorKind::FailedToParseFile)?;

    Ok(TermInfo::parse(&data)
        .context(ErrorKind::FailedToParseFile)?
        .into())
}
