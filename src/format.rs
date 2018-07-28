use ansi;
use failure::ResultExt;
use std::io;
use std::str;
use std::str::FromStr;
use ErrorKind;
use Result;

pub fn write_fmt<W: io::Write>(w: &mut W, s: &[u8]) -> Result<()> {
    let mut bold = false;
    let mut italics = false;
    let mut strike = false;
    let mut blink = false;

    let mut slice = s;
    while slice.len() > 0 {
        let printable_count = slice
            .iter()
            .take_while(|&&c| {
                c != b'_' && c != b'*' && c != b'\\' && c != b'%' && c != b'[' && c != b'~'
            })
            .count();
        w.write(&slice[..printable_count])
            .context(ErrorKind::FailedWriteToStdout)?;;
        if slice.len() == printable_count {
            break;
        }

        let mut read = printable_count;

        match slice[read] {
            b'_' => {
                bold = !bold;

                if bold {
                    ansi::sgr(w, ansi::GraphicRendition::Bold)?;
                } else {
                    ansi::sgr(w, ansi::GraphicRendition::ResetImpact)?;
                };
                read += 1;
            }
            b'*' => {
                italics = !italics;

                if italics {
                    ansi::sgr(w, ansi::GraphicRendition::Italic)?;
                } else {
                    ansi::sgr(w, ansi::GraphicRendition::ResetStyle)?;
                };
                read += 1;
            }
            b'~' => {
                strike = !strike;

                if strike {
                    ansi::sgr(w, ansi::GraphicRendition::Strike)?;
                } else {
                    ansi::sgr(w, ansi::GraphicRendition::ResetStrike)?;
                };
                read += 1;
            }
            b'%' => {
                blink = !blink;

                if blink {
                    ansi::sgr(w, ansi::GraphicRendition::Blink)?;
                } else {
                    ansi::sgr(w, ansi::GraphicRendition::ResetBlink)?;
                };
                read += 1;
            }
            b'\\' => {
                match slice.iter().nth(read + 1) {
                    Some(b'\\') => w.write(&[b'\\']),
                    Some(b'_') => w.write(&[b'_']),
                    Some(b'~') => w.write(&[b'~']),
                    Some(b'*') => w.write(&[b'*']),
                    Some(b'%') => w.write(&[b'%']),
                    Some(b'{') => w.write(&[b'{']),
                    Some(&x) => w.write(&[b'\\', x]),
                    None => w.write(b"\\"),
                }.context(ErrorKind::FailedWriteToStdout)?;
                read += 2;
            }
            b'[' => match slice.iter().nth(read + 1) {
                Some(b'+') => {
                    let split_index = slice[read + 2..].iter().take_while(|&&c| c != b':').count();
                    let end_index = slice[split_index + read + 2..]
                        .iter()
                        .take_while(|&&c| c != b']')
                        .count() + split_index;

                    let s =
                        unsafe { str::from_utf8_unchecked(&slice[read + 2..read + 2 + end_index]) };
                    match s[..split_index].trim() {
                        "fg" => {
                            ansi::set_foreground(w, ansi::Color::from_str(&s[split_index + 1..])?)?
                        }
                        "bg" => {
                            ansi::set_background(w, ansi::Color::from_str(&s[split_index + 1..])?)?
                        }
                        _ => return Err(ErrorKind::InvalidColorLocation.into()),
                    };
                    read += end_index + 3
                }
                Some(b'-') => {
                    let end_index = slice[read + 2..].iter().take_while(|&&c| c != b']').count();

                    let s =
                        unsafe { str::from_utf8_unchecked(&slice[read + 2..read + 2 + end_index]) };
                    match s.trim() {
                        "fg" => ansi::sgr(w, ansi::GraphicRendition::ResetForeground)?,
                        "bg" => ansi::sgr(w, ansi::GraphicRendition::ResetBackground)?,
                        "all" => ansi::sgr(w, ansi::GraphicRendition::Reset)?,
                        _ => return Err(ErrorKind::InvalidResetSpecifier.into()),
                    };
                    read += end_index + 3
                }
                Some(&x) => {
                    w.write(&[b'[', x]).context(ErrorKind::FailedWriteToStdout)?;
                }
                _ => {
                    w.write(b"[").context(ErrorKind::FailedWriteToStdout)?;
                }
            },
            _ => {
                w.write(&slice[read..read + 1])
                    .context(ErrorKind::FailedWriteToStdout)?;
                read += 1;
            }
        }
        slice = &slice[read..];
    }

    Ok(())
}

pub fn format<T: AsRef<str>>(s: T) -> Result<String> {
    let mut buffer = Vec::new();
    write_fmt(&mut buffer, s.as_ref().as_bytes())?;
    Ok(String::from_utf8(buffer).unwrap())
}

#[cfg(test)]
mod test {
    use format;
    use terminfo;
    #[test]
    fn colors() {
        assert_eq!(
            format("this is [+fg:red]red[-fg] so is [+fg:rgb(128, 0, 0)]this[-fg]").unwrap(),
            "this is \x1b[31mred\x1b[39m so is \x1b[38;2;128;0;0mthis\x1b[39m"
        );

        assert_eq!(
            format("this is [+fg : blue]~red~ -- I mean blue[-fg] but this is red [+fg:rgb(128, 52, 52)]this[-fg]").unwrap(),
            "this is \x1b[34m\x1b[9mred\x1b[29m -- I mean blue\x1b[39m but this is red \x1b[38;2;128;52;52mthis\x1b[39m"
        );

        assert_eq!(
            format("maybe... [+fg:6]cyan[-fg] or should I go with [+fg:15]white[-fg]").unwrap(),
            "maybe... \x1b[36mcyan\x1b[39m or should I go with \x1b[97mwhite\x1b[39m"
        );
    }

    #[test]
    fn glitter() {
        assert_eq!(
            format("_THIS IS VERY IMPORTANT").unwrap(),
            "\x1b[1mTHIS IS VERY IMPORTANT"
        );

        assert_eq!(
            format("_THIS IS VERY %IMPOR_TANT%").unwrap(),
            "\x1b[1mTHIS IS VERY \x1b[5mIMPOR\x1b[22mTANT\x1b[25m"
        );

        assert_eq!(
            format("*fancy* %blinky _text_%").unwrap(),
            "\x1b[3mfancy\x1b[23m \x1b[5mblinky \x1b[1mtext\x1b[22m\x1b[25m"
        );

        assert_eq!(
            format("_%*~HORRIBLE[-all] ok!").unwrap(),
            "\x1b[1m\x1b[5m\x1b[3m\x1b[9mHORRIBLE\x1b[0m ok!"
        );
    }
}
