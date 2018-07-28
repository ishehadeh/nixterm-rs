use errors::*;
use failure::ResultExt;
use nix::sys::termios;
use std::cell::RefMut;
use std::collections::VecDeque;
use std::io;
use std::io::Write;
use std::io::{BufRead, Read};
use std::os::unix::io::{AsRawFd, RawFd};
use term;
use terminfo;
use termios::LocalFlags;
use termios::Termios;

pub struct RawTerm<'a, I, O>
where
    I: io::Read + AsRawFd + 'a,
    O: io::Write + AsRawFd + 'a,
{
    tty: &'a mut term::Term<I, O>,
    original: termios::Termios,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ArrowDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Control(char),
    Delete,
    Newline,
    Escape,
    Arrow(ArrowDirection),
    Invalid(u8),
}

pub struct Keys<'a, I, O>
where
    I: io::Read + AsRawFd + 'a,
    O: io::Write + AsRawFd + 'a,
{
    // Keys may need to be buffered if we have to back out of an escape code
    buffer: VecDeque<Key>,
    unread: VecDeque<u8>,
    tty: &'a term::Term<I, O>,
}

impl<'a, I, O> Keys<'a, I, O>
where
    I: io::Read + AsRawFd + 'a,
    O: io::Write + AsRawFd + 'a,
{
    fn getch(&mut self) -> Result<Option<u8>> {
        if let Some(v) = self.unread.pop_front() {
            return Ok(Some(v));
        }

        let mut c: [u8; 1] = [0; 1];
        let read = self.tty
            .stdin()
            .read(&mut c)
            .context(ErrorKind::GetCharFailed)?;
        if read == 0 {
            Ok(None)
        } else {
            Ok(Some(c[0]))
        }
    }

    fn getkey(&mut self) -> Result<Key> {
        let mut c = self.getch()?;
        while c.is_none() {
            c = self.getch()?;
        }
        let ch = c.unwrap();

        Ok(match ch {
            0...12 => Key::Control((ch + 64) as char),
            13 => Key::Newline,
            27 => match self.getch()? {
                Some(91) => match self.getch()? {
                    Some(65) => Key::Arrow(ArrowDirection::Up),
                    Some(66) => Key::Arrow(ArrowDirection::Down),
                    Some(67) => Key::Arrow(ArrowDirection::Left),
                    Some(68) => Key::Arrow(ArrowDirection::Right),
                    Some(v) => {
                        self.buffer.push_back(Key::Char(']'));
                        self.unread.push_back(v);
                        Key::Escape
                    }
                    None => {
                        self.buffer.push_back(Key::Escape);
                        Key::Char(']')
                    }
                },
                Some(v) => {
                    self.unread.push_back(v);
                    Key::Escape
                }
                None => Key::Escape,
            },
            127 => Key::Delete,
            32...126 => Key::Char(ch as char),
            _ => Key::Invalid(ch),
        })
    }
}

impl<'a, I, O> Iterator for Keys<'a, I, O>
where
    I: io::Read + AsRawFd + 'a,
    O: io::Write + AsRawFd + 'a,
{
    type Item = Result<Key>;

    fn next(&mut self) -> Option<Self::Item> {
        // if a key is in the buffer then return it
        match self.buffer.pop_front() {
            Some(v) => return Some(Ok(v)),
            None => (),
        };

        Some(self.getkey())
    }
}

pub fn raw<'a, I, O>(t: &'a mut term::Term<I, O>) -> Result<RawTerm<I, O>>
where
    I: io::Read + AsRawFd,
    O: io::Write + AsRawFd,
{
    Ok(RawTerm {
        original: init_raw_mode(t.as_raw_fd())?,
        tty: t,
    })
}

fn init_raw_mode(fd: RawFd) -> Result<termios::Termios> {
    let mut raw_termios = termios::tcgetattr(fd).unwrap();
    let original_termios = raw_termios.clone();

    termios::cfmakeraw(&mut raw_termios); // TODO: do this manually
    raw_termios.local_flags.remove(LocalFlags::ICANON);
    raw_termios.control_chars[termios::SpecialCharacterIndices::VTIME as usize] = 1;
    raw_termios.control_chars[termios::SpecialCharacterIndices::VMIN as usize] = 0;
    termios::tcsetattr(0, termios::SetArg::TCSAFLUSH, &raw_termios)
        .context(ErrorKind::InitRawModeFailed)?;

    Ok(original_termios)
}

fn restore_termios(fd: RawFd, original: &termios::Termios) -> Result<()> {
    termios::tcsetattr(fd, termios::SetArg::TCSAFLUSH, original)
        .context(ErrorKind::ExitRawModeFailed)?;
    Ok(())
}

impl<'a, I, O> Drop for RawTerm<'a, I, O>
where
    I: io::Read + AsRawFd,
    O: io::Write + AsRawFd,
{
    fn drop(&mut self) {
        restore_termios(self.tty.as_raw_fd(), &self.original).unwrap();
    }
}

impl<'a, I, O> RawTerm<'a, I, O>
where
    I: io::Read + AsRawFd,
    O: io::Write + AsRawFd,
{
    pub fn move_cursor(&self, x: usize, y: usize) -> Result<()> {
        self.tty.move_cursor(x, y)
    }

    pub fn shift_cursor(&self, x: isize, y: isize) -> Result<()> {
        self.tty.shift_cursor(x, y)
    }

    pub fn set_column(&self, x: isize) -> Result<()> {
        self.tty.set_column(x)
    }

    pub fn newline(&self) -> Result<()> {
        self.tty.set_column(0)?;
        self.write("\n")?;
        Ok(())
    }

    pub fn keys<'b>(&'b self) -> Keys<'b, I, O> {
        Keys {
            buffer: VecDeque::with_capacity(4),
            unread: VecDeque::with_capacity(4),
            tty: self.tty,
        }
    }

    pub fn write<T: AsRef<str>>(&self, s: T) -> Result<()> {
        self.tty.stdout().write(s.as_ref().as_bytes());
        self.tty.stdout().flush();
        Ok(())
    }

    pub fn cursor(&mut self) -> Result<term::Cursor> {
        self.tty
            .stdout()
            .write(
                self.tty
                    .info
                    .string(terminfo::ReqMousePos)
                    .unwrap_or("\x1b[6n")
                    .as_bytes(),
            )
            .context(ErrorKind::InvalidCursorPosition)?;
        self.tty
            .stdout()
            .flush()
            .context(ErrorKind::InvalidCursorPosition)?;
        let mut pos = Vec::with_capacity(10);
        self.tty
            .stdin()
            .read_until(b'R', &mut pos)
            .context(ErrorKind::InvalidCursorPosition)?;

        if pos.len() < 4 || pos[0] != b'\x1b' || pos[1] != b'[' {
            return Err(ErrorKind::InvalidCursorPosition.into());
        }

        let y = pos.iter()
            .skip(2)
            .take_while(|&&c| c != b';')
            .try_fold((0, 1), |(x, pow), c| -> Result<(usize, usize)> {
                match c {
                    48...57 => Ok((x + (c - b'0') as usize * pow, pow * 10)),
                    _ => Err(ErrorKind::InvalidNumber.into()),
                }
            })
            .context(ErrorKind::InvalidCursorPosition)?
            .0;
        let x = pos.iter()
            .skip_while(|&&c| c != b';')
            .skip(1)
            .take_while(|&&c| c != b'R')
            .try_fold((0, 1), |(x, pow), c| -> Result<(usize, usize)> {
                match c {
                    48...57 => Ok((x + (c - b'0') as usize * pow, pow * 10)),
                    _ => Err(ErrorKind::InvalidNumber.into()),
                }
            })
            .context(ErrorKind::InvalidCursorPosition)?
            .0;
        Ok(term::Cursor::new(x, y))
    }

    pub fn save_cursor(&self) -> Result<()> {
        self.tty.save_cursor()
    }

    pub fn restore_cursor(&self) -> Result<()> {
        self.tty.restore_cursor()
    }

    pub fn test_unicode(&mut self) -> Result<bool> {
        self.save_cursor().context(ErrorKind::FailedToGetTabWidth)?;
        self.move_cursor(0, 0)?;

        self.tty
            .stdout()
            .write("é".as_bytes())
            .context(ErrorKind::FailedToGetTabWidth)?;
        self.tty
            .stdout()
            .flush()
            .context(ErrorKind::FailedToGetTabWidth)?;

        let cursor_after = self.cursor().context(ErrorKind::FailedToGetTabWidth)?;

        self.restore_cursor()
            .context(ErrorKind::FailedToGetTabWidth)?;

        // if the cursor only move by one it probably means the character was detected and displayed correctly
        Ok(cursor_after.cols() == 1)
    }

    pub fn test_tab_width(&mut self) -> Result<usize> {
        self.save_cursor().context(ErrorKind::FailedToGetTabWidth)?;
        self.move_cursor(0, 0)?;

        self.tty
            .stdout()
            .write(b"\t")
            .context(ErrorKind::FailedToGetTabWidth)?;
        self.tty
            .stdout()
            .flush()
            .context(ErrorKind::FailedToGetTabWidth)?;

        let cursor_after = self.cursor().context(ErrorKind::FailedToGetTabWidth)?;

        self.restore_cursor()
            .context(ErrorKind::FailedToGetTabWidth)?;

        Ok(cursor_after.cols() - 1)
    }
}

impl<'a, I, O> AsRawFd for RawTerm<'a, I, O>
where
    I: io::Read + AsRawFd,
    O: io::Write + AsRawFd,
{
    fn as_raw_fd(&self) -> RawFd {
        self.tty.as_raw_fd()
    }
}
