use errors::*;
use failure::ResultExt;
use std::io::Write;
use std::str::{Chars, FromStr};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Color {
    Index(u8),
    Rgb(u8, u8, u8),
}

#[repr(u8)]
pub enum ControlSequence {
    CursorUp = 65,
    CursorDown = 66,
    CursorRight = 67,
    CursorLeft = 69,
    CursorNextLine = 70,
    CursorPrevLine = 71,
    CursorToColumn = 72,
    CursorMoveTo = 73,
    EraseDisplay = 74,
    EraseLine = 75,
    ScollUp = 76,
    ScrollDown = 77,
    CursorMoveToAlt = 102,
    AddGraphicRendition = 109,
    AuxControl = 105,
    ReportCursor = 110,
    CursorSaveState = 115,
    CursorRestoreState = 117,
}

pub enum GraphicRendition {
    Reset = 0,
    Bold = 1,
    Faint = 2,
    Italic = 3,
    Underline = 4,
    Blink = 5,
    FastBlink = 6,
    Invert = 7,
    Conceal = 8,
    Strike = 9,
    Fraktur = 20,

    ResetImpact = 22,
    ResetStyle = 23,
    ResetUnderline = 24,
    ResetBlink = 25,
    ResetInvert = 27,
    ResetConceal = 28,
    ResetStrike = 29,

    ResetFrame = 54,
    ResetOverline = 55,
    ResetIdeogram = 65,

    ResetFont = 10,
    ResetBackground = 49,
    ResetForeground = 39,
}

impl From<u8> for Color {
    fn from(v: u8) -> Color {
        Color::Index(v)
    }
}

impl From<(u8, u8, u8)> for Color {
    fn from(v: (u8, u8, u8)) -> Color {
        Color::Rgb(v.0, v.1, v.2)
    }
}

impl Color {
    fn from_hex<'a>(iter: Chars<'a>) -> Result<Color> {
        let s = iter.as_str();
        let char_count = s.len();
        match char_count {
            3 => Ok(Color::Rgb(
                s[0..1].parse::<u8>().context(ErrorKind::InvalidNumber)?,
                s[1..2].parse::<u8>().context(ErrorKind::InvalidNumber)?,
                s[2..3].parse::<u8>().context(ErrorKind::InvalidNumber)?,
            )),
            6 => Ok(Color::Rgb(
                s[0..2].parse::<u8>().context(ErrorKind::InvalidNumber)?,
                s[2..4].parse::<u8>().context(ErrorKind::InvalidNumber)?,
                s[4..6].parse::<u8>().context(ErrorKind::InvalidNumber)?,
            )),
            _ => Err(ErrorKind::InvalidNumber.into()),
        }
    }

    fn from_name(s: &str) -> Result<Color> {
        Ok(match s {
            "black" => Color::Index(0),
            "red" => Color::Index(1),
            "green" => Color::Index(2),
            "yellow" => Color::Index(3),
            "blue" => Color::Index(4),
            "magenta" => Color::Index(5),
            "cyan" => Color::Index(6),
            "grey" => Color::Index(7),
            "darkgrey" => Color::Index(8),
            "brightred" => Color::Index(9),
            "brightgreen" => Color::Index(10),
            "brightyellow" => Color::Index(11),
            "brightblue" => Color::Index(12),
            "brightmagenta" => Color::Index(13),
            "brightcyan" => Color::Index(14),
            "white" => Color::Index(15),
            _ => return Err(ErrorKind::UnknownColorName(s.to_owned()).into()),
        })
    }

    fn from_rgb<'a>(srciter: Chars<'a>) -> Result<Color> {
        let s = srciter.clone().as_str();
        let iter = srciter.enumerate();

        let mut rgb: [&str; 3] = ["", "", ""];
        let mut is_floating = false;

        let mut index = 1;
        for x in 0..3 {
            for (i, c) in iter.clone().skip(index) {
                match c {
                    ' ' | '\t' | '\n' => index += 1,
                    '0'...'9' => (),
                    '.' => is_floating = true,
                    'e' | 'E' => is_floating = true,
                    ',' | ')' => {
                        rgb[x] = &s[index..i];
                        index = i + 1;
                        break;
                    }
                    _ => (),
                }
            }
        }
        if !is_floating {
            Ok(Color::Rgb(
                rgb[0].parse::<u8>().context(ErrorKind::InvalidNumber)?,
                rgb[1].parse::<u8>().context(ErrorKind::InvalidNumber)?,
                rgb[2].parse::<u8>().context(ErrorKind::InvalidNumber)?,
            ))
        } else {
            Ok(Color::Rgb(
                (rgb[0]
                    .parse::<f64>()
                    .context(ErrorKind::InvalidNumber)?
                    .fract() * 255.0) as u8,
                (rgb[1]
                    .parse::<f64>()
                    .context(ErrorKind::InvalidNumber)?
                    .fract() * 255.0) as u8,
                (rgb[2]
                    .parse::<f64>()
                    .context(ErrorKind::InvalidNumber)?
                    .fract() * 255.0) as u8,
            ))
        }
    }
}

impl FromStr for Color {
    type Err = Error;

    fn from_str(s: &str) -> Result<Color> {
        let lower: String = s.to_lowercase()
            .chars()
            .skip_while(|&c| c == ' ' || c == '\t' || c == '\r' || c == '\n')
            .collect();
        let len = lower.len();

        let c0 = lower.chars().nth(0).unwrap();
        match c0 {
            '#' => Ok(Color::from_hex(lower.chars()).context(ErrorKind::InvalidColor)?),
            '0'...'9' => Ok(Color::Index(lower
                .parse::<u8>()
                .context(ErrorKind::InvalidColor)?)),
            'r' => {
                if len > 4 && lower.chars().nth(1).unwrap() == 'g'
                    && lower.chars().nth(2).unwrap() == 'b'
                {
                    let offset = lower[2..].chars().take_while(|&c| c != '(').count();
                    Ok(Color::from_rgb(lower[offset + 2..].chars())
                        .context(ErrorKind::InvalidColor)?)
                } else if len > 2 && lower.chars().nth(1).unwrap() == 'e'
                    && lower.chars().nth(2).unwrap() == 'd'
                {
                    Ok(Color::Index(1))
                } else {
                    Err(ErrorKind::UnknownColorName(s.to_owned()).into())
                }
            }
            _ => Color::from_name(&lower),
        }
    }
}

pub fn set_foreground<W: Write>(w: &mut W, c: Color) -> Result<()> {
    Ok(match c {
        Color::Index(x @ 0...7) => write!(w, "\x1b[{}m", x + 30),
        Color::Index(x @ 8...15) => write!(w, "\x1b[{}m", x + 82),
        Color::Index(x) => write!(w, "\x1b[38;5;{}m", x),
        Color::Rgb(r, g, b) => write!(w, "\x1b[38;2;{};{};{}m", r, g, b),
    }.context(ErrorKind::CsiFailed)?)
}

pub fn set_background<W: Write>(w: &mut W, c: Color) -> Result<()> {
    Ok(match c {
        Color::Index(x @ 0...7) => write!(w, "\x1b[{}m", x + 40),
        Color::Index(x @ 8...15) => write!(w, "\x1b[{}m", x + 92),
        Color::Index(x) => write!(w, "\x1b[48;5;{}m", x),
        Color::Rgb(r, g, b) => write!(w, "\x1b[98;2;{};{};{}m", r, g, b),
    }.context(ErrorKind::CsiFailed)?)
}

pub fn cursor_shift_vertical<W: Write>(w: &mut W, shift: isize) -> Result<()> {
    if shift < 0 {
        Ok(write!(w, "\x1b[{}B", -shift).context(ErrorKind::CsiFailed)?)
    } else if shift > 0 {
        Ok(write!(w, "\x1b[{}A", shift).context(ErrorKind::CsiFailed)?)
    } else {
        Ok(())
    }
}

pub fn cursor_shift_horizontal<W: Write>(w: &mut W, shift: isize) -> Result<()> {
    if shift < 0 {
        Ok(write!(w, "\x1b[{}D", -shift).context(ErrorKind::CsiFailed)?)
    } else if shift > 0 {
        Ok(write!(w, "\x1b[{}C", shift).context(ErrorKind::CsiFailed)?)
    } else {
        Ok(())
    }
}

pub fn cursor_move<W: Write>(w: &mut W, x: usize, y: usize) -> Result<()> {
    Ok(write!(w, "\x1b[{};{}m", x, y).context(ErrorKind::CsiFailed)?)
}

pub fn cursor_set_column<W: Write>(w: &mut W, x: usize) -> Result<()> {
    Ok(write!(w, "\x1b[{}G", x).context(ErrorKind::CsiFailed)?)
}

pub fn sgr<W: Write>(w: &mut W, gr: GraphicRendition) -> Result<()> {
    Ok(write!(w, "\x1b[{}m", gr as usize).context(ErrorKind::CsiFailed)?)
}
