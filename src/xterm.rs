///! Xterm OS Commands
///!
///! This module can also be used xterm-based terminal (rxvt, xterm-256, kitty, etc)
use ansi;
use failure::ResultExt;
use std::fmt::Write;
use ErrorKind;
use Result;

#[derive(Debug, Clone)]
pub enum XColor {
    Index(u8),
    Rgbi(f32, f32, f32),
    Rgb(u16, u16, u16),
    Raw(String),
}

pub fn set_icon_and_title<W: Write, T: AsRef<str>>(w: &mut W, s: T) -> Result<()> {
    Ok(write!(w, "\x1b[0;{}\x1b\\", s.as_ref()).context(ErrorKind::OscFailed)?)
}

pub fn set_icon<W: Write, T: AsRef<str>>(w: &mut W, s: T) -> Result<()> {
    Ok(write!(w, "\x1b[1;{}\x1b\\", s.as_ref()).context(ErrorKind::OscFailed)?)
}

pub fn set_title<W: Write, T: AsRef<str>>(w: &mut W, s: T) -> Result<()> {
    Ok(write!(w, "\x1b[2;{}\x1b\\", s.as_ref()).context(ErrorKind::OscFailed)?)
}

pub fn reset_title<W: Write>(w: &mut W) -> Result<()> {
    Ok(write!(w, "\x1b[2;\x1b\\").context(ErrorKind::OscFailed)?)
}

pub fn set_x_property<W: Write, T: AsRef<str>, U: AsRef<str>>(w: &mut W, k: T, v: U) -> Result<()> {
    Ok(write!(w, "\x1b[3;{}={}\x1b\\", k.as_ref(), v.as_ref()).context(ErrorKind::OscFailed)?)
}

pub fn remove_x_property<W: Write, T: AsRef<str>>(w: &mut W, k: T) -> Result<()> {
    Ok(write!(w, "\x1b[3;{}\x1b\\", k.as_ref()).context(ErrorKind::OscFailed)?)
}

pub fn query_x_property<W: Write, T: AsRef<str>>(w: &mut W, k: T) -> Result<()> {
    Ok(write!(w, "\x1b[3;?{}\x1b\\", k.as_ref()).context(ErrorKind::OscFailed)?)
}

pub fn map_color<W: Write>(w: &mut W, c: u8, new_color: XColor) -> Result<()> {
    Ok(match new_color {
        XColor::Index(x) => write!(w, "\x1b[4;{};{}\x1b\\", c, x),
        XColor::Rgbi(r, g, b) => write!(w, "\x1b[4;{};rgbi:{}/{}/{}\x1b\\", c, r, g, b),
        XColor::Rgb(r, g, b) => write!(w, "\x1b[4;{};rgb:{}/{}/{}\x1b\\", c, r, g, b),
        XColor::Raw(s) => write!(w, "\x1b[4;{};{}\x1b\\", c, s),
    }.context(ErrorKind::OscFailed)?)
}

pub fn query_color<W: Write>(w: &mut W, c: u8) -> Result<()> {
    Ok(write!(w, "\x1b[4;{};?\x1b\\", c).context(ErrorKind::OscFailed)?)
}

impl From<ansi::Color> for XColor {
    fn from(c: ansi::Color) -> XColor {
        match c {
            ansi::Color::Rgb(r, g, b) => XColor::Rgb(r as u16, g as u16, b as u16),
            ansi::Color::Index(c) => XColor::Index(c),
        }
    }
}

impl From<(u16, u16, u16)> for XColor {
    fn from(c: (u16, u16, u16)) -> XColor {
        XColor::Rgb(c.0, c.1, c.2)
    }
}

impl From<(f32, f32, f32)> for XColor {
    fn from(c: (f32, f32, f32)) -> XColor {
        XColor::Rgbi(c.0, c.1, c.2)
    }
}

impl<'a> From<&'a str> for XColor {
    fn from(s: &'a str) -> XColor {
        XColor::Raw(s.to_string())
    }
}

impl From<String> for XColor {
    fn from(s: String) -> XColor {
        XColor::Raw(s)
    }
}

///! Kitty extensions to the xterm protocol
///! [details](https://sw.kovidgoyal.net/kitty/protocol-extensions.html)
pub mod kitty {
    use ansi;
    use failure::ResultExt;
    use std::fmt::Write;
    use terminfo;
    use ErrorKind;
    use Result;

    pub enum Underline {
        None,
        Straight,
        Double,
        Curly,
        Dotted,
        Dashed,
    }

    pub fn set_underline<W: Write>(w: &mut W, u: Underline) -> Result<()> {
        Ok(match u {
            Underline::None => write!(w, "\x1b]4:0m"),
            Underline::Straight => write!(w, "\x1b]4:1m"),
            Underline::Double => write!(w, "\x1b]4:2m"),
            Underline::Curly => write!(w, "\x1b]4:3m"),
            Underline::Dotted => write!(w, "\x1b]4:4m"),
            Underline::Dashed => write!(w, "\x1b]4:5m"),
        }.context(ErrorKind::OscFailed)?)
    }

    pub fn set_underline_color<W: Write, T: Into<ansi::Color>>(w: &mut W, x: T) -> Result<()> {
        Ok(match x.into() {
            ansi::Color::Index(i) => write!(w, "\x1b]58;5;{}m", i),
            ansi::Color::Rgb(r, g, b) => write!(w, "\x1b]58;2;{};{};{}m", r, g, b),
        }.context(ErrorKind::OscFailed)?)
    }

    pub fn reset_underline_color<W: Write>(w: &mut W) -> Result<()> {
        Ok(write!(w, "\x1b]59m").context(ErrorKind::OscFailed)?)
    }
}
