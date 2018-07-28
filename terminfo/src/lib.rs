#![feature(test)]
//! TermInfo is a small library for quickly reading and modifying the terminfo database.
//!
//!

#[macro_use]
extern crate failure;
extern crate test;

mod errors;
mod fields;
// pub mod interpreter;
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

#[cfg(test)]
mod tests {
    use terminfo::*;
    use terminfobuf::*;
    use test;
    use test::Bencher;
    use BooleanField::*;
    use NumericField::*;
    use StringField::*;

    const RXVT_INFO: &'static [u8] = include_bytes!("../test-data/rxvt");
    const XTERM_INFO: &'static [u8] = include_bytes!("../test-data/xterm");
    const LINUX_16COLOR_INFO: &'static [u8] = include_bytes!("../test-data/linux-16color");

    #[bench]
    fn bench_parse_time(b: &mut Bencher) {
        b.iter(|| {
            let _rxvt = TermInfo::parse(RXVT_INFO).unwrap();
            let _xterm = TermInfo::parse(XTERM_INFO).unwrap();
            let _l16c = TermInfo::parse(LINUX_16COLOR_INFO).unwrap();
        })
    }

    #[bench]
    fn bench_parse_buffer_time(b: &mut Bencher) {
        b.iter(|| {
            let rxvt_buf: TermInfoBuf = TermInfo::parse(RXVT_INFO).unwrap().into();
            let xterm_buf: TermInfoBuf = TermInfo::parse(XTERM_INFO).unwrap().into();
            let l16c_buf: TermInfoBuf = TermInfo::parse(LINUX_16COLOR_INFO).unwrap().into();
            test::black_box(rxvt_buf);
            test::black_box(xterm_buf);
            test::black_box(l16c_buf);
        });
    }

    #[bench]
    fn bench_terminfo_lookup(b: &mut Bencher) {
        let rxvt = TermInfo::parse(RXVT_INFO).unwrap();
        let xterm = TermInfo::parse(XTERM_INFO).unwrap();
        let l16c = TermInfo::parse(LINUX_16COLOR_INFO).unwrap();

        b.iter(|| {
            test::black_box(rxvt.string(KeyF10));
            test::black_box(rxvt.string(KeyHome));
            test::black_box(rxvt.string(Bell));
            test::black_box(rxvt.string(KeyCancel));
            test::black_box(rxvt.number(Columns));
            test::black_box(rxvt.number(MaxColors));

            test::black_box(xterm.number(MaxColors));

            test::black_box(xterm.number(Columns));
            test::black_box(xterm.number(MaxColors));
            test::black_box(xterm.boolean(AutoLeftMargin));
            test::black_box(xterm.boolean(AutoRightMargin));
            test::black_box(xterm.boolean(MoveInsertMode));
            test::black_box(xterm.boolean(XonXoff));

            test::black_box(l16c.boolean(AutoLeftMargin));
            test::black_box(l16c.boolean(AutoRightMargin));
            test::black_box(l16c.boolean(MoveInsertMode));
            test::black_box(l16c.boolean(XonXoff));

            test::black_box(xterm.boolean(AutoLeftMargin));
            test::black_box(xterm.boolean(AutoRightMargin));
            test::black_box(xterm.boolean(MoveInsertMode));
            test::black_box(xterm.boolean(CanChange));
        });
    }

    #[bench]
    fn bench_terminfobuf_lookup(b: &mut Bencher) {
        let rxvt: TermInfoBuf = TermInfo::parse(RXVT_INFO).unwrap().into();
        let xterm: TermInfoBuf = TermInfo::parse(XTERM_INFO).unwrap().into();
        let l16c: TermInfoBuf = TermInfo::parse(LINUX_16COLOR_INFO).unwrap().into();

        b.iter(|| {
            test::black_box(rxvt.string(KeyF10));
            test::black_box(rxvt.string(KeyHome));
            test::black_box(rxvt.string(Bell));
            test::black_box(rxvt.string(KeyCancel));
            test::black_box(rxvt.number(Columns));
            test::black_box(rxvt.number(MaxColors));

            test::black_box(xterm.number(MaxColors));

            test::black_box(xterm.number(Columns));
            test::black_box(xterm.number(MaxColors));
            test::black_box(xterm.boolean(AutoLeftMargin));
            test::black_box(xterm.boolean(AutoRightMargin));
            test::black_box(xterm.boolean(MoveInsertMode));
            test::black_box(xterm.boolean(XonXoff));

            test::black_box(l16c.boolean(AutoLeftMargin));
            test::black_box(l16c.boolean(AutoRightMargin));
            test::black_box(l16c.boolean(MoveInsertMode));
            test::black_box(l16c.boolean(XonXoff));

            test::black_box(xterm.boolean(AutoLeftMargin));
            test::black_box(xterm.boolean(AutoRightMargin));
            test::black_box(xterm.boolean(MoveInsertMode));
            test::black_box(xterm.boolean(CanChange));
        });
    }
}
