use errors::*;
use failure::ResultExt;
use lang::Argument;
use std::io;

const NULL: &'static [u8] = &[b'(', b'n', b'u', b'l', b'l', b')'];
const NUM_CHARS: [u8; 16] = [
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd', b'e', b'f',
];

const UPPERCASE_NUM_CHARS: [u8; 16] = [
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'A', b'B', b'C', b'D', b'E', b'F',
];

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct PrintfArgs {
    pub left_align: bool,
    pub show_sign: bool,
    pub pad_sign: bool,
    pub alt: bool,
    pub width: Option<usize>,
    pub prec: Option<usize>,
    pub character: char,
}

impl PrintfArgs {
    pub fn parse(src: &[u8]) -> Result<PrintfArgs> {
        let mut spec = PrintfArgs::default();

        if src.len() < 1 {
            return Err(ErrorKind::BadPrintfSpecifier.into());
        }

        match src[0] {
            // flags are prefixed with a `:`
            b':' => spec.parse_flags(&src[1..])?,
            b'0'..=b'9' | b'.' => spec.parse_width(src)?,
            _ => spec.parse_specifier(src)?,
        }

        Ok(spec)
    }

    fn pad<W: io::Write>(&self, w: &mut W, buf: &[u8]) -> Result<()> {
        if let Some(width) = self.width {
            if buf.len() < width && !self.left_align {
                for _ in buf.len()..width {
                    w.write(&[b' ']).context(ErrorKind::FailedToWriteArgument)?;
                }
            }
        }

        w.write(buf).context(ErrorKind::FailedToWriteArgument)?;

        if let Some(width) = self.width {
            if buf.len() < width && self.left_align {
                for _ in buf.len()..width {
                    w.write(&[b' ']).context(ErrorKind::FailedToWriteArgument)?;
                }
            }
        }

        Ok(())
    }
    pub fn write_number<W: io::Write>(&self, w: &mut W, num: i64) -> Result<()> {
        let (radix, uppercase) = match self.character {
            'x' => (16, false),
            'X' => (16, true),
            'o' => (8, false),
            'd' => (10, false),
            's' => return Err(ErrorKind::UnexpectedArgumentType("string", "integer").into()),
            'c' => return Err(ErrorKind::UnexpectedArgumentType("char", "integer").into()),
            _ => return Err(ErrorKind::UnexpectedArgumentType("", "integer").into()),
        };
        let mut num_buf = [0u8; 22];

        let mut wnum = num;
        let mut num_buf_len = 0;
        if wnum < 0 {
            wnum = -wnum
        }

        if num < 0 {
            num_buf[0] = b'-';
            num_buf_len += 1;
        } else if self.pad_sign {
            num_buf[0] = b' ';
            num_buf_len += 1;
        } else if self.show_sign {
            num_buf[0] = b'+';
            num_buf_len += 1;
        }

        if self.alt {
            if radix == 8 {
                num_buf[num_buf_len] = b'0';
                num_buf_len += 1;
            } else if uppercase && radix == 16 {
                num_buf[num_buf_len] = b'0';
                num_buf[num_buf_len + 1] = b'X';
                num_buf_len += 2;
            } else if radix == 16 {
                num_buf[num_buf_len] = b'0';
                num_buf[num_buf_len + 1] = b'x';
                num_buf_len += 2;
            }
        }

        let prefix_len = num_buf_len;

        while wnum > 0 {
            let c = wnum % radix;
            wnum /= radix;
            if uppercase {
                num_buf[num_buf_len] = UPPERCASE_NUM_CHARS[c as usize]
            } else {
                num_buf[num_buf_len] = NUM_CHARS[c as usize]
            }
            num_buf_len += 1;
        }

        num_buf[prefix_len..num_buf_len].reverse();

        if let Some(prec) = self.prec {
            if num_buf_len - prefix_len > prec {
                num_buf_len = prec + prefix_len;
            }
        }
        self.pad(w, &num_buf[..num_buf_len])
    }

    pub fn write_string<W: io::Write>(&self, w: &mut W, s: &str) -> Result<()> {
        match self.character {
            'x' | 'X' | 'o' | 'd' => {
                return Err(ErrorKind::UnexpectedArgumentType("integer", "string").into())
            }
            'c' => return Err(ErrorKind::UnexpectedArgumentType("char", "string").into()),
            _ => (),
        };

        let mut slen = s.len();

        if let Some(prec) = self.prec {
            if slen > prec {
                slen = prec
            }
        }

        self.pad(w, s[..slen].as_bytes())
    }

    pub fn write_char<W: io::Write>(&self, w: &mut W, c: u8) -> Result<()> {
        match self.character {
            'x' | 'X' | 'o' | 'd' => {
                return Err(ErrorKind::UnexpectedArgumentType("integer", "char").into())
            }
            's' => return Err(ErrorKind::UnexpectedArgumentType("integer", "string").into()),
            _ => (),
        };

        self.pad(w, &[c])
    }

    pub fn print<T: Into<Argument>, W: io::Write>(&self, w: &mut W, arg: Option<T>) -> Result<()> {
        match arg.map(|x| x.into()) {
            Some(Argument::Integer(x)) => self.write_number(w, x)?,
            Some(Argument::String(s)) => self.write_string(w, &s)?,
            Some(Argument::Char(c)) => self.write_char(w, c)?,
            None => {
                w.write(NULL).context(ErrorKind::FailedToWriteArgument)?;
            }
        };

        Ok(())
    }

    fn parse_specifier(&mut self, src: &[u8]) -> Result<()> {
        match src.iter().nth(0) {
            Some(b'x') => self.character = 'x',
            Some(b'o') => self.character = 'o',
            Some(b'X') => self.character = 'X',
            Some(b'd') => self.character = 'd',
            Some(b's') => self.character = 's',
            Some(b'c') => self.character = 'c',
            _ => return Err(ErrorKind::BadPrintfSpecifier.into()),
        };
        Ok(())
    }

    fn parse_flags(&mut self, src: &[u8]) -> Result<()> {
        let flags = src.iter()
            .take_while(|&&c| c == b'+' || c == b'-' || c == b'#' || c == b' ')
            .fold(0, |x, flag| {
                match flag {
                    b'+' => self.show_sign = true,
                    b'-' => self.left_align = true,
                    b'#' => self.alt = true,
                    b' ' => self.pad_sign = true,
                    _ => unreachable!(),
                }
                x + 1
            });

        self.parse_width(&src[flags..])
    }

    fn parse_width(&mut self, src: &[u8]) -> Result<()> {
        let width_width = src.iter().take_while(|&&c| c >= b'0' && c <= b'9').count();

        if width_width > 0 {
            self.width =
                Some(parse_usize(&src[..width_width]).context(ErrorKind::BadPrecisionSpecified)?);
        }

        if src.len() > width_width && src[width_width] == b'.' {
            let prec_width = src.iter()
                .skip(width_width + 1)
                .take_while(|&&c| c >= b'0' && c <= b'9')
                .count();

            if prec_width > 0 {
                self.prec = Some(
                    parse_usize(&src[width_width + 1..width_width + 1 + prec_width])
                        .context(ErrorKind::BadPrecisionSpecified)?,
                );

                self.parse_specifier(&src[prec_width + 1 + width_width..])
            } else {
                Err(ErrorKind::BadPrecisionSpecified.into())
            }
        } else {
            self.parse_specifier(&src[width_width..])
        }
    }
}

fn parse_usize(s: &[u8]) -> Result<usize> {
    s.iter()
        .try_fold((0_usize, 1_usize), |(num, pow), &c| {
            if c >= b'0' && c <= b'9' {
                Ok((num + (((c - b'0') as usize) * pow), pow * 10))
            } else {
                Err(ErrorKind::InvalidDigit(c).into())
            }
        })
        .map(|(num, _)| num)
}
