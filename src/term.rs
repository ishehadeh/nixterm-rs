use ansi;
use errors::*;
use failure::ResultExt;
use format;
use nix::libc;
use nix::libc::{winsize, TIOCGWINSZ};
use nix::sys::termios;
use nix::sys::termios::{ControlFlags, LocalFlags};
use rawterm;
use std::cell::{RefCell, RefMut};
use std::io;
use std::io::{BufRead, Write};
use std::mem;
use std::os::unix::io::{AsRawFd, RawFd};
use std::{thread, time};
use terminfo;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Cursor(usize, usize);

pub enum Align {
    Left,
    Right,
    Center,
}

/// The user's terminal
pub struct Term<I, O>
where
    I: io::Read + AsRawFd,
    O: io::Write + AsRawFd,
{
    pub info: terminfo::TermInfoBuf,

    stdout: RefCell<O>,
    stdin: RefCell<io::BufReader<I>>,
    stdin_fd: RawFd,
    old_termios: Option<termios::Termios>,
    tabwidth: Option<usize>,
    unicode_supported: Option<bool>,
    align: Align,

    // to properly align text the whole line has to be available.
    // since `O` is write-only a buffer for the currently is kept
    // so if the user wants to append to it text can still be properly aligned
    line_buffer: String,

    // 99% of the time this will just be '\n' (0x0A), sometimes it may be '\r' (0x0D) or "\n\r"
    newline: String,
}

/// An iterator over a string split into lines to fit a terminal
///
/// Create `TermLines` with the Term::fit_terminal()  method
pub struct TermLines<'a> {
    columns: usize,
    slice: &'a str,
    len: usize,
}

impl Cursor {
    pub fn new(x: usize, y: usize) -> Cursor {
        Cursor(x, y)
    }

    pub fn x(&self) -> usize {
        self.0
    }

    pub fn y(&self) -> usize {
        self.1
    }

    pub fn cols(&self) -> usize {
        self.x()
    }

    pub fn rows(&self) -> usize {
        self.y()
    }
}

impl<'a> Iterator for TermLines<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            None
        } else if self.len > self.columns {
            let (line, extra) = split_near(self.slice, self.columns);
            self.slice = extra;
            self.len = length(self.slice);
            Some((self.columns - length(line), line))
        } else {
            let oldlen = self.len;
            self.len = 0;
            Some((self.columns - oldlen, self.slice))
        }
    }
}

impl Term<io::Stdin, io::Stdout> {
    pub fn new() -> Result<Term<io::Stdin, io::Stdout>> {
        Term::from_streams(io::stdin(), io::stdout())
    }
}

fn length(s: &str) -> usize {
    let mut len = s.len();
    let mut slice = s;
    while slice.len() > 0 {
        slice = &slice[slice.bytes().take_while(|&c| c != b'\x1b').count()..];
        if slice.len() > 1 {
            match slice.chars().nth(1) {
                Some('[') => {
                    let escape_length = slice
                        .chars()
                        .take_while(|&c| !((c <= 'Z' && c >= 'A') || (c <= 'z' && c >= 'a')))
                        .count();
                    slice = &slice[escape_length + 1..];
                    len -= escape_length + 1
                }
                Some(_) => slice = &slice[1..],
                None => (),
            }
        }
    }
    len
}

fn is_whitespace(c: char) -> bool {
    c == ' ' || c == '\t' || c == '\n' || c == '\r'
}

fn split_near(s: &str, x: usize) -> (&str, &str) {
    let closes_space = x - s.chars()
        .rev()
        .skip(s.len() - x)
        .take_while(|&c| !is_whitespace(c))
        .count();
    if closes_space < x / 2 {
        (&s[..x], &s[x..])
    } else {
        (&s[..closes_space - 1], &s[closes_space..])
    }
}

impl<I, O> Term<I, O>
where
    I: io::Read + AsRawFd,
    O: io::Write + AsRawFd,
{
    pub fn from_streams(stdin: I, stdout: O) -> Result<Term<I, O>> {
        Ok(Term {
            stdin_fd: stdin.as_raw_fd(),
            old_termios: None,
            stdin: RefCell::new(io::BufReader::new(stdin)),
            stdout: RefCell::new(stdout),
            unicode_supported: None,
            info: terminfo::from_env().context(ErrorKind::TermInitFailed)?,
            tabwidth: None,
            align: Align::Left,
            line_buffer: String::new(),
            newline: String::from("\n"),
        })
    }

    fn try_exec<'a>(
        &'a self,
        field: terminfo::StringField,
    ) -> Result<terminfo::lang::Executor<'a>> {
        match self.info.exec(field) {
            Some(v) => Ok(v),
            None => Err(ErrorKind::MissingTermInfoField(field).into()),
        }
    }

    pub fn tab_width(&mut self) -> Result<usize> {
        if let Some(tw) = self.tabwidth {
            Ok(tw)
        } else {
            let tw = self.raw()?.test_tab_width()?;
            self.tabwidth = Some(tw);
            Ok(tw)
        }
    }

    pub fn unicode_supported(&mut self) -> Result<bool> {
        if let Some(us) = self.unicode_supported {
            Ok(us)
        } else {
            let supported = self.raw()?.test_unicode()?;
            self.unicode_supported = Some(supported);
            Ok(supported)
        }
    }

    pub fn read_line(&mut self) -> Result<String> {
        let mut s = String::new();
        self.stdin()
            .read_line(&mut s)
            .context(ErrorKind::ReadLineFailed)?;
        Ok(s)
    }

    pub fn query<T: AsRef<str>>(&mut self, s: T) -> Result<String> {
        self.print(s);
        self.stdout.borrow_mut().flush();
        self.line_buffer.clear();
        self.read_line()
    }

    pub fn size(&self) -> (usize, usize) {
        unsafe {
            let mut w: winsize = mem::uninitialized();
            libc::ioctl(
                self.stdout.borrow().as_raw_fd(),
                TIOCGWINSZ,
                &mut w as *mut winsize,
            );
            (w.ws_row as usize, w.ws_col as usize)
        }
    }

    pub fn has_underlines(&self) -> bool {
        self.info.ext_boolean("Su")
    }

    /// Slice a string into sections that fit onto a single terminal line.
    ///
    /// returns a iterator over tuples containing the sliced string and the remaining cells on the line if that string were to be printed.
    pub fn fit_terminal<'a>(&self, s: &'a str) -> TermLines<'a> {
        let (_, columns) = self.size();
        TermLines {
            columns: columns,
            len: length(s),
            slice: s,
        }
    }

    fn right_align_line(&mut self, s: &str) -> Result<()> {
        for (margin, line) in self.fit_terminal(s) {
            if line.is_empty() {
                self.stdout
                    .borrow_mut()
                    .write(self.newline.as_bytes())
                    .context(ErrorKind::FailedToAlignRight)?;
                continue;
            }

            self.shift_cursor(margin as isize, 0)?;

            self.stdout
                .borrow_mut()
                .write(line.as_bytes())
                .context(ErrorKind::FailedToAlignRight)?;
        }

        Ok(())
    }

    fn center_line(&mut self, s: &str) -> Result<()> {
        for (margin, line) in self.fit_terminal(s) {
            if line.is_empty() {
                self.stdout
                    .borrow_mut()
                    .write(self.newline.as_bytes())
                    .context(ErrorKind::FailedToAlignCenter)?;
                continue;
            }

            self.shift_cursor(margin as isize / 2, 0)?;

            self.stdout
                .borrow_mut()
                .write(line.as_bytes())
                .context(ErrorKind::FailedToAlignCenter)?;
        }

        Ok(())
    }

    pub fn align_center(&mut self) {
        self.align = Align::Center;
    }

    pub fn align_left(&mut self) {
        self.align = Align::Left;
    }

    pub fn align_right(&mut self) {
        self.align = Align::Right;
    }

    pub fn writeln<T: AsRef<str>>(&mut self, s: T) -> Result<()> {
        // writeln is optimized to avoid copying to the line buffer
        // since a newline is guaranteed
        let mut string = format(s.as_ref())?;
        if !self.line_buffer.is_empty() {
            string.insert_str(0, &self.line_buffer);

            // clear the entire line, line buffer and reset the cursor
            // we're going to just redraw this entire line
            print!("\x1b[G\x1b[K");
            self.line_buffer.clear();
        }

        match self.align {
            Align::Left => {
                self.stdout.borrow_mut().write(string.as_bytes());
                self.stdout.borrow_mut().write(b"\n");
            }
            Align::Right => {
                for ln in string.lines() {
                    if ln.len() > 0 {
                        self.right_align_line(ln);
                    }
                    self.stdout.borrow_mut().write(b"\n");
                }
            }
            Align::Center => {
                for ln in string.lines() {
                    if ln.len() > 0 {
                        self.center_line(ln);
                    }
                    self.stdout.borrow_mut().write(b"\n");
                }
            }
        };
        Ok(())
    }

    pub fn bold(&self, bold: bool) -> Result<()> {
        self.try_exec(terminfo::SetAttributes)?
            .argi(0, bold)
            .write(&mut *self.stdout())
            .context(ErrorKind::FailedToRunTerminfo(terminfo::SetAttributes))?;
        Ok(())
    }

    pub fn move_cursor(&self, x: usize, y: usize) -> Result<()> {
        self.info
            .exec(terminfo::CursorAddress)
            .unwrap_or(terminfo::lang::Executor::new(b"%i%p1%p2\x1b%d;%dH"))
            .arg(y)
            .arg(x)
            .write(&mut *self.stdout())
            .unwrap();
        Ok(())
    }

    pub fn save_cursor(&self) -> Result<()> {
        self.try_exec(terminfo::SaveCursor)?
            .write(&mut *self.stdout())
            .context(ErrorKind::FailedToRunTerminfo(terminfo::SaveCursor))?;
        Ok(())
    }

    pub fn restore_cursor(&self) -> Result<()> {
        self.try_exec(terminfo::RestoreCursor)?
            .write(&mut *self.stdout())
            .context(ErrorKind::FailedToRunTerminfo(terminfo::RestoreCursor))?;
        Ok(())
    }

    pub fn shift_cursor(&self, x: isize, y: isize) -> Result<()> {
        if y > 0 {
            self.try_exec(terminfo::ParmDownCursor)?
                .arg(y)
                .write(&mut *self.stdout())
                .context(ErrorKind::FailedToRunTerminfo(terminfo::ParmDownCursor))?;
        } else if y < 0 {
            self.try_exec(terminfo::ParmUpCursor)?
                .arg(-y)
                .write(&mut *self.stdout())
                .context(ErrorKind::FailedToRunTerminfo(terminfo::ParmUpCursor))?;
        }
        if x > 0 {
            self.try_exec(terminfo::ParmRightCursor)?
                .arg(x)
                .write(&mut *self.stdout())
                .context(ErrorKind::FailedToRunTerminfo(terminfo::ParmRightCursor))?;
        } else if x < 0 {
            self.try_exec(terminfo::ParmLeftCursor)?
                .arg(-x)
                .write(&mut *self.stdout())
                .context(ErrorKind::FailedToRunTerminfo(terminfo::ParmLeftCursor))?;
        }
        Ok(())
    }

    pub fn set_column(&self, x: isize) -> Result<()> {
        self.try_exec(terminfo::ColumnAddress)?
            .arg(x)
            .write(&mut *self.stdout())
            .context(ErrorKind::FailedToRunTerminfo(terminfo::ParmLeftCursor))?;
        Ok(())
    }

    pub fn raw<'a>(&'a mut self) -> Result<rawterm::RawTerm<'a, I, O>> {
        rawterm::raw(self)
    }

    pub fn stdout(&self) -> RefMut<O> {
        self.stdout.borrow_mut()
    }

    pub fn stdin(&self) -> RefMut<io::BufReader<I>> {
        self.stdin.borrow_mut()
    }

    pub fn println<S: AsRef<str>>(&mut self, s: S) -> Result<usize> {
        let mut buffer = Vec::new();
        format::write_fmt(&mut buffer, s.as_ref().as_bytes())?;
        let mut string = String::from_utf8(buffer).unwrap();
        if !self.line_buffer.is_empty() {
            string.insert_str(0, &self.line_buffer);

            // clear the entire line, line buffer and reset the cursor
            // we're going to just redraw this entire line
            self.stdout.borrow_mut().write(b"\x1b[G\x1b[K");
            self.line_buffer.clear();
        }
        let lines = string.split("\n");

        match self.align {
            Align::Left => {
                self.stdout.borrow_mut().write(string.as_bytes());
                self.stdout.borrow_mut().write(self.newline.as_bytes());
            }
            Align::Right => {
                for ln in lines {
                    self.right_align_line(ln)?;
                    self.stdout.borrow_mut().write(self.newline.as_bytes());
                }
            }
            Align::Center => {
                for ln in lines {
                    self.center_line(ln)?;
                    self.stdout.borrow_mut().write(self.newline.as_bytes());
                }
            }
        };
        Ok(string.len() + 1)
    }

    pub fn print<S: AsRef<str>>(&mut self, s: S) -> Result<usize> {
        // Write is almost identical to writeln, except it stores the last line,
        // if it wasn't empty.
        let mut buffer = Vec::new();
        format::write_fmt(&mut buffer, s.as_ref().as_bytes())?;
        let mut string = String::from_utf8(buffer).unwrap();
        if !self.line_buffer.is_empty() {
            string.insert_str(0, &self.line_buffer);

            // clear the entire line, line buffer and reset the cursor
            // we're going to just redraw this entire line
            self.stdout.borrow_mut().write(b"\x1b[G\x1b[K");
            self.line_buffer.clear();
        }
        let lines = string.split("\n");

        let last = lines.clone().last().unwrap();
        if last.len() > 0 {
            self.line_buffer = last.to_owned();
        }

        match self.align {
            Align::Left => {
                self.stdout.borrow_mut().write(string.as_bytes());
            }
            Align::Right => {
                for ln in lines {
                    if ln.len() > 0 {
                        self.right_align_line(ln);
                    } else {
                        self.stdout.borrow_mut().write(self.newline.as_bytes());
                    }
                }
            }
            Align::Center => {
                for ln in lines {
                    if ln.len() > 0 {
                        self.center_line(ln);
                    } else {
                        self.stdout.borrow_mut().write(self.newline.as_bytes());
                    }
                }
            }
        };
        self.flush();
        Ok(string.len())
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.stdout().flush()
    }
}

impl<I, O> AsRawFd for Term<I, O>
where
    I: io::Read + AsRawFd,
    O: io::Write + AsRawFd,
{
    fn as_raw_fd(&self) -> RawFd {
        self.stdin_fd
    }
}
