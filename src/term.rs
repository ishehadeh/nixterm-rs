use ansi;
use errors::*;
use events::Keys;
use failure::Fail;
use failure::ResultExt;
use nix::sys::termios;
use std::cell::RefCell;
use std::io;
use std::io::{BufRead, BufReader, Read};
use std::ops::DerefMut;
use std::os::unix::io::{AsRawFd, RawFd};
use std::sync::{Mutex, MutexGuard};
use terminfo;
use util;

macro_rules! terminfo_setter {
    (@imp $name:ident($field:ident) -> $enum:ident::$flag:ident) => {
        #[inline]
        pub fn $name(mut self, v: bool) -> Self {
            use $crate::nix::sys::termios::$enum;

            if v {
                self.termios.$field |= $enum::$flag;
            } else {
                self.termios.$field ^= $enum::$flag;
            }
            self
        }
    };

    (char $name:ident -> $char:ident) => {
        terminfo_setter!(char $name -> $char<char>);
    };

    (char $name:ident -> $char:ident<$typ:ty>) => {
        #[inline]
        pub fn $name(mut self, v: $typ) -> Self {
            use $crate::nix::sys::termios;
            use $crate::nix::libc;

            self.termios.control_chars[termios::SpecialCharacterIndices::$char as usize] = v as libc::cc_t;
            self
        }
    };

    (local $name:ident -> $flag:ident) => {
        terminfo_setter!(@imp $name(local_flags) -> LocalFlags::$flag);
    };

    (ctrl $name:ident -> $flag:ident) => {
        terminfo_setter!(@imp $name(control_flags) -> ControlFlags::$flag);
    };

    (out $name:ident -> $flag:ident) => {
        terminfo_setter!(@imp $name(output_flags) -> OutputFlags::$flag);
    };

    (inp $name:ident -> $flag:ident) => {
        terminfo_setter!(@imp $name(input_flags) -> InputFlags::$flag);
    };
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Cursor(usize, usize);

pub enum Align {
    Left,
    Right,
    Center,
}

#[derive(Clone)]
pub struct Settings {
    termios: termios::Termios,
}

pub struct Term<I, O>
where
    I: io::Read + AsRawFd,
    O: io::Write + AsRawFd,
{
    pub info: terminfo::TermInfoBuf,
    stdin_fd: RawFd,
    stdin: Mutex<BufReader<I>>,
    stdout: Mutex<O>,
    err: RefCell<Option<Error>>,
}

pub struct TermWriter<'a, O>
where
    O: io::Write + AsRawFd + 'a,
{
    info: &'a terminfo::TermInfoBuf,
    err: Option<Error>,
    written: usize,
    stdout: MutexGuard<'a, O>,

    bold: bool,
    blink: bool,
    underline: bool,
    italics: bool,
    standout: bool,
    invert: bool,
    invisible: bool,
    dim: bool,

    foreground: Option<ansi::Color>,
    background: Option<ansi::Color>,
}

impl Settings {
    /// Convenience method to configure the terminal to be in "raw" mode
    ///
    /// This method is roughly equivalent to the termios C API's `cfmakeraw()` function. It configures the terminal
    /// to stop responding to signals, perform no post, or input processing, and turns off echoing and canonical mode.
    pub fn raw(mut self) -> Self {
        use nix::sys::termios::ControlFlags;
        use nix::sys::termios::InputFlags;
        use nix::sys::termios::LocalFlags;
        use nix::sys::termios::OutputFlags;

        self.termios.input_flags &= !(InputFlags::IGNBRK
            | InputFlags::BRKINT
            | InputFlags::PARMRK
            | InputFlags::ISTRIP
            | InputFlags::INLCR
            | InputFlags::IGNCR
            | InputFlags::ICRNL
            | InputFlags::IXON);
        self.termios.output_flags &= !OutputFlags::OPOST;
        self.termios.local_flags &= !(LocalFlags::ECHO
            | LocalFlags::ECHONL
            | LocalFlags::ICANON
            | LocalFlags::ISIG
            | LocalFlags::IEXTEN);
        self.termios.control_flags &= !(ControlFlags::CSIZE | ControlFlags::PARENB);
        self.termios.control_flags |= ControlFlags::CS8;
        return self;
    }

    /// Set the character size, `x` must be in the range 5-8 otherwise this method will panic
    pub fn char_size(mut self, x: u8) -> Self {
        if x < 5 || x > 8 {
            panic!("terminal character size must be in the range 5-8");
        }
        self.termios
            .control_flags
            .remove(termios::ControlFlags::CSIZE);

        self.termios.control_flags.insert(match x {
            5 => termios::ControlFlags::CS5,
            6 => termios::ControlFlags::CS6,
            7 => termios::ControlFlags::CS7,
            8 => termios::ControlFlags::CS8,
            _ => unreachable!(),
        });
        self
    }

    /// Turn off canonical mode.
    /// In non-canonical mode the terminal does perform line editing, the input buffer is 4096 characters long,
    /// and timeouts may be set for input.
    terminfo_setter!(local canonical -> ICANON);

    /// Set the minimum number of characters for the `Term::read` function to return.
    ///
    /// __*non-canonical mode only__
    terminfo_setter!(char characters -> VMIN<u8>);

    /// Set the maximum time the `Term::read` function will wait before returning (time in deciseconds)
    ///
    ///
    /// __*non-canonical mode only__
    terminfo_setter!(char timeout -> VTIME<u8>);

    /// Set the character that will signal the stdout buffer should be flushed.
    terminfo_setter!(char flush -> VEOF);

    /// Set an extra end-of-line character
    terminfo_setter!(char eol -> VEOL);

    /// Set the erase char (typically it will be something like `DEL` by default)
    terminfo_setter!(char erase -> VERASE);

    /// Set the interrupt char, when this character is read the running process will be sent `SIGINT`.
    terminfo_setter!(char interrupt -> VINTR);

    /// Set the suspend char, when this character is read the running process will be sent `SIGTSTP`.
    terminfo_setter!(char suspend -> VSUSP);

    /// Set the kill char, when this character is read the running process will be sent `SIGKILL`.
    terminfo_setter!(char kill -> VKILL);

    /// Set the quit char, when this character is read the running process will be sent `SIGQUIT`.
    terminfo_setter!(char quit -> VQUIT);

    /// stop showing output until the `start_output` char is reached.
    terminfo_setter!(char start_output -> VSTART);

    /// Set the character the will restart  output, ofter the `stop_output` char.
    terminfo_setter!(char stop_output -> VSTOP);

    terminfo_setter!(local echo -> ECHO);
    terminfo_setter!(local echo_newline -> ECHONL);
    terminfo_setter!(local signals -> ISIG);
    terminfo_setter!(local flush_on_signal -> NOFLSH);
    terminfo_setter!(local input_processing -> IEXTEN);

    terminfo_setter!(ctrl parity -> PARENB);
    terminfo_setter!(ctrl odd_parity -> PARODD);
    terminfo_setter!(ctrl hangup -> HUPCL);
    terminfo_setter!(ctrl ignore_modem_ctrl_lines -> CLOCAL);

    terminfo_setter!(out post_processing -> OPOST);
    terminfo_setter!(out make_output_carriage_return_newline -> OCRNL);

    terminfo_setter!(inp ignore_break -> IGNBRK);
    terminfo_setter!(inp interrupt_on_break -> BRKINT);
    terminfo_setter!(inp ignore_frame_and_parity_errors -> IGNPAR);
    terminfo_setter!(inp check_input_parity -> INPCK);
    terminfo_setter!(inp strip_bit8 -> ISTRIP);
    terminfo_setter!(inp make_input_carriage_return_newline -> ICRNL);
    terminfo_setter!(inp make_input_newline_carriage_return -> INLCR);
    terminfo_setter!(inp mark_bad_input -> IGNPAR);
    terminfo_setter!(inp ignore_input_carriage_return -> IGNCR);
    terminfo_setter!(inp xon_xoff -> IXON);
    terminfo_setter!(inp utf8 -> IUTF8);
}

impl Term<io::Stdin, io::Stdout> {
    pub fn new() -> Result<Term<io::Stdin, io::Stdout>> {
        Ok(Term::from_streams(
            terminfo::from_env().context(ErrorKind::FailedToCreateTermInstance)?,
            io::stdin(),
            io::stdout(),
        ))
    }
}

/// Map a `seta[b/f]` color to a `set[b/f]` color.
#[inline]
fn seta_to_set_pallet(x: u8) -> u8 {
    match x {
        1 => 4, // red and blue switch places
        4 => 1,
        3 => 6, // magenta and yellow switch places
        6 => 3,
        _ => x,
    }
}

/// Convert r, g and b values into a 3-bit pallet based color
///
/// Expected Color Pallet:
/// 0. black
/// 1. red
/// 2. green
/// 3. yellow
/// 4. blue
/// 5. magenta
/// 6. cyan
/// 7. grey
fn index_from_rgb3(r: u8, g: u8, b: u8) -> u8 {
    let ir = r as isize;
    let ig = g as isize;
    let ib = b as isize;

    if ir > 200 && ig > 200 && ib > 200 {
        7
    } else if ir > (ig + ib) {
        1
    } else if ig > (ir + ib) {
        2
    } else if ib > (ig + ir) {
        4
    } else if (ir - ig).abs() < ib {
        3
    } else if (ib - ig).abs() < ir {
        6
    } else if (ib - ir).abs() < ig {
        5
    } else {
        0
    }
}

/// Same as index_from_rgb3 but with any extra bit to tell if the color should be "bright"
fn index_from_rgb4(r: u8, g: u8, b: u8) -> u8 {
    let ir = r as isize;
    let ig = g as isize;
    let ib = b as isize;

    if ir > 200 && ig > 200 && ib > 200 {
        15
    } else if ir > 150 && ig > 150 && ib > 150 {
        8
    } else if ir > (ig + ib) {
        if ir / 2 > (ig + ib) {
            9
        } else {
            1
        }
    } else if ig > (ir + ib) {
        if ig > (ir + ib) {
            10
        } else {
            2
        }
    } else if ib > (ig + ir) {
        if ib > (ig + ir) {
            12
        } else {
            4
        }
    } else if (ir - ig).abs() < ib {
        if (ir - ig).abs() < ib / 2 {
            11
        } else {
            3
        }
    } else if (ib - ig).abs() < ir {
        if (ib - ig).abs() < ir / 2 {
            14
        } else {
            6
        }
    } else if (ib - ir).abs() < ig {
        if (ib - ir).abs() < ig / 2 {
            13
        } else {
            5
        }
    } else {
        0
    }
}

impl<'a, O> TermWriter<'a, O>
where
    O: io::Write + AsRawFd + 'a,
{
    fn exec<'b>(&'b self, field: terminfo::StringField) -> Result<terminfo::lang::Executor<'a>> {
        match self.info.exec(field) {
            Some(v) => Ok(v),
            None => Err(ErrorKind::MissingTermInfoField(field).into()),
        }
    }

    fn write_info_str(mut self, field: terminfo::StringField, fallback: &[u8]) -> Self {
        if self.err().is_some() {
            return self;
        }

        match self.exec(field) {
            Ok(mut v) => {
                self.written += v
                    .write(self.stdout.deref_mut())
                    .context(ErrorKind::FailedToRunTerminfo(field))
                    .unwrap_or_else(|e| {
                        self.err = Some(e.into());
                        0
                    });
                self
            }
            Err(_) => self.write_bytes(fallback),
        }
    }

    /// Try to map the color into its closest equivalent supported by this terminal.
    fn scrunch_color(&self, color: ansi::Color) -> ansi::Color {
        match self.info.number(terminfo::MaxColors).unwrap_or(2) {
            8..=15 => match color {
                ansi::Color::Index(x @ 0..=7) => x,
                ansi::Color::Index(x @ 8..=15) => (x - 8),
                ansi::Color::Index(16) => 0,
                ansi::Color::Index(x @ 17..=232) => {
                    index_from_rgb3((x % 6) * 51, ((x / 6) % 6) * 51, (x / 36) * 51)
                }
                ansi::Color::Index(x) => {
                    if x > 233 + (255 - 233) / 2 {
                        7
                    } else {
                        0
                    }
                }
                ansi::Color::Rgb(r, g, b) => index_from_rgb3(r, g, b),
            }.into(),
            16..=87 => match color {
                ansi::Color::Index(x @ 0..=15) => x,
                ansi::Color::Index(16) => 0,
                ansi::Color::Index(x @ 17..=232) => {
                    index_from_rgb4((x % 6) * 51, ((x / 6) % 6) * 51, (x / 36) * 51)
                }
                ansi::Color::Index(x) => {
                    let y = 233 + (255 - 233);
                    if x > (y / 3) * 2 {
                        15
                    } else if x > y / 3 {
                        0
                    } else {
                        7
                    }
                }
                ansi::Color::Rgb(r, g, b) => index_from_rgb4(r, g, b),
            }.into(),
            88..=255 => match color.into() {
                ansi::Color::Index(x @ 0..=15) => x,
                ansi::Color::Index(x) => (x as f64 * 0.3451171875) as u8,
                ansi::Color::Rgb(r, g, b) => (r * 4 + g) * 4 + b + 16,
            }.into(),
            256 => match color.into() {
                ansi::Color::Index(x) => x,
                ansi::Color::Rgb(r, g, b) => (r * 16 + g) * 16 + b + 16,
            }.into(),
            _ => unimplemented!(),
        }
    }

    fn write_u8(mut self, x: u8) -> Self {
        if self.err().is_some() {
            return self;
        }

        match util::write_u8_ansi(self.stdout.deref_mut(), x) {
            Ok(v) => self.written += v,
            Err(e) => self.err = Some(e.context(ErrorKind::WriteFailed).into()),
        };

        self
    }

    fn wipe_formatting(&mut self) {
        self.standout = false;
        self.underline = false;
        self.invert = false;
        self.blink = false;
        self.dim = false;
        self.bold = false;
        self.invisible = false;
        self.background = None;
        self.foreground = None;
    }

    fn set_sgr(&mut self) {
        match self
            .exec(terminfo::SetAttributes)
            .unwrap()
            .arg(self.standout)
            .arg(self.underline)
            .arg(self.invert)
            .arg(self.blink)
            .arg(self.dim)
            .arg(self.bold)
            .arg(self.invisible)
            .write(self.stdout.deref_mut())
        {
            Ok(v) => self.written += v,
            Err(e) => {
                self.err = Some(
                    e.context(ErrorKind::FailedToRunTerminfo(terminfo::SetAAttributes))
                        .into(),
                );
            }
        }
    }

    pub fn write_bytes(mut self, buf: &[u8]) -> Self {
        if self.err().is_some() {
            return self;
        }

        self.set_sgr();
        if let Err(e) = self.write_fg_bg() {
            self.err = Some(
                e.context(ErrorKind::FailedToRunTerminfo(terminfo::SetAAttributes))
                    .into(),
            );
            return self;
        }

        match self.stdout.write(buf) {
            Ok(v) => self.written += v,
            Err(e) => self.err = Some(e.context(ErrorKind::WriteFailed).into()),
        };

        self.wipe_formatting();
        self
    }

    pub fn print<T: AsRef<str>>(self, s: T) -> Self {
        self.write_bytes(s.as_ref().as_bytes())
    }

    pub fn println<T: AsRef<str>>(self, s: T) -> Self {
        self.print(s).print("\n")
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub fn blink(mut self) -> Self {
        self.blink = true;
        self
    }

    pub fn italics(mut self) -> Self {
        self.italics = true;
        self
    }

    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    pub fn invisible(mut self) -> Self {
        self.invisible = true;
        self
    }

    pub fn standout(mut self) -> Self {
        self.standout = true;
        self
    }

    pub fn dim(mut self) -> Self {
        self.dim = true;
        self
    }

    pub fn clear(self) -> Self {
        self.write_info_str(terminfo::ExitAttributeMode, ansi::ALL_OFF)
    }

    pub fn done(mut self) -> Result<usize> {
        self.stdout.flush().context(ErrorKind::WriteFailed)?;
        match self.err {
            Some(v) => Err(v),
            None => Ok(self.written),
        }
    }

    pub fn err(&self) -> &Option<Error> {
        &self.err
    }

    pub fn written(&self) -> usize {
        self.written
    }

    /// Set the terminal's foreground color.
    ///
    /// `T` a `ansi::Color` enum, a number (`u8`) or a string.
    /// strings may name a color, or provide custom r, g, and b values.
    /// Any of the following are valid:
    /// - "black"
    /// - "red"
    /// - "green"
    /// - "yellow"
    /// - "blue"
    /// - "magenta"
    /// - "cyan"
    /// - "grey"
    /// - "white" (same as "bright grey")
    /// - "bright*" (`*` may be any of the other named colors)
    /// - "#rrggbb"
    /// - "#rgb"
    /// - "rgb(r, g, b)"
    ///
    /// Numbers are also valid, the number must fit inside a `u8` (so it should be in the range 0-255).
    ///
    /// # Compatibility
    /// Everything layed out above will work as far as this library, but the chances of it actually being supported across
    /// any meaningful number of terminals is closed to 0.
    ///
    /// ## 24-bit Colors
    /// Full RGB (that is, 24-bit color) terminals are pretty rare. When full 24-bit colors are not supported,
    /// the color will get as close as it can with whatever the terminal provides.
    ///
    /// ## 8-bit Colors
    /// 256 color terminals are fairly common, using depending on indices beyond 15 is still risky though.
    ///
    /// ## 4/3-bit Colors
    /// Basically all terminals will support 3-bits, many will support 4 (that 4th bit gives the option of a "bright" variant).
    pub fn foreground<T: Into<ansi::Color>>(mut self, color: T) -> Self {
        if self.err().is_some() {
            return self;
        }

        self.foreground = Some(self.scrunch_color(color.into()));
        self
    }

    pub fn background<T: Into<ansi::Color>>(mut self, color: T) -> Self {
        if self.err().is_some() {
            return self;
        }

        self.background = Some(self.scrunch_color(color.into()));
        self
    }

    fn write_fg_bg(&mut self) -> Result<()> {
        if self.err().is_some() {
            return Ok(());
        }
        let bg = self.background.clone();
        let fg = self.foreground.clone();

        self.write_color(
            bg,
            b"48;2;5",
            terminfo::SetABackground,
            terminfo::SetBackground,
        )?;
        self.write_color(
            fg,
            b"38;2;5",
            terminfo::SetAForeground,
            terminfo::SetForeground,
        )
    }

    fn write_color(
        &mut self,
        color: Option<ansi::Color>,
        rgb_prefix: &[u8],
        seta: terminfo::StringField,
        set: terminfo::StringField,
    ) -> Result<()> {
        match color {
            Some(ansi::Color::Index(x)) => {
                self.written += match self.exec(seta) {
                    Ok(e) => e
                        .arg(x as usize)
                        .write(self.stdout.deref_mut())
                        .context(ErrorKind::FailedToRunTerminfo(set))
                        .map_err(|e| e.into()),
                    Err(_) => self.exec(set).map(|exe| {
                        exe.arg(seta_to_set_pallet(x) as usize)
                            .write(self.stdout.deref_mut())
                            .unwrap_or_else(|e| {
                                self.err =
                                    Some(e.context(ErrorKind::FailedToRunTerminfo(set)).into());
                                0
                            })
                    }),
                }.context(ErrorKind::WriteFailed)?
            }
            Some(ansi::Color::Rgb(r, g, b)) => {
                use std::io::Write;

                self.written += self.write(rgb_prefix).context(ErrorKind::WriteFailed)?
                    + util::write_u8_ansi(self, r).context(ErrorKind::WriteFailed)?
                    + self.write(b";").context(ErrorKind::WriteFailed)?
                    + util::write_u8_ansi(self, g).context(ErrorKind::WriteFailed)?
                    + self.write(b";").context(ErrorKind::WriteFailed)?
                    + util::write_u8_ansi(self, b).context(ErrorKind::WriteFailed)?
                    + self.write(b"m").context(ErrorKind::WriteFailed)?;
            }
            None => (),
        }

        Ok(())
    }

    pub fn shift_cursor(mut self, x: isize, y: isize) -> Self {
        if self.err.is_some() {
            return self;
        }

        if (x > 0) {
            self.exec(terminfo::ParmRightCursor)
                .map(|ctx| ctx.arg(x).write(&mut self))
                .map_err(|e| self.err = Some(e));
        } else if (x < 0) {
            self.exec(terminfo::ParmLeftCursor)
                .map(|ctx| ctx.arg(-x).write(&mut self))
                .map_err(|e| self.err = Some(e));
        }

        if self.err.is_some() {
            return self;
        }

        if (y > 0) {
            self.exec(terminfo::ParmUpCursor)
                .map(|ctx| ctx.arg(y).write(&mut self))
                .map_err(|e| self.err = Some(e));
        } else if (y < 0) {
            self.exec(terminfo::ParmDownCursor)
                .map(|ctx| ctx.arg(-y).write(&mut self))
                .map_err(|e| self.err = Some(e));
        }

        return self;
    }

    pub fn default_background(mut self) -> Self {
        self.write_bytes(ansi::RESET_BACKGROUND)
    }

    pub fn default_foreground(mut self) -> Self {
        self.write_bytes(ansi::RESET_FOREGROUND)
    }
}

impl<'a, O> io::Write for TermWriter<'a, O>
where
    O: io::Write + AsRawFd,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Some(_) = self.err() {
            return Ok(0);
        }

        self.set_sgr();

        self.stdout.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stdout.flush()
    }
}

/// Term represents the user's terminal.
/// It has two channels, `I` (input), and `O` (output).
/// Each terminal is accompanied by a "terminfo" file, (represented by the `TermInfoBuf` struct).
impl<I, O> Term<I, O>
where
    I: io::Read + AsRawFd,
    O: io::Write + AsRawFd,
{
    pub fn from_streams(tib: terminfo::TermInfoBuf, stdin: I, stdout: O) -> Term<I, O> {
        Term {
            info: tib,
            stdin_fd: stdin.as_raw_fd(),
            stdin: Mutex::new(BufReader::new(stdin)),
            stdout: Mutex::new(stdout),
            err: RefCell::new(None),
        }
    }

    /// Write to the terminal's stdout, it returns the number of bytes written.
    /// `write` does not need a mutable reference to `self`, meaning it can be used while self is being borrowed,
    /// however `write` blocks if it's being called from another thread.
    ///
    /// # Examples
    /// ```
    /// use nixterm::term::Term;
    ///
    /// pub fn main() {
    ///     let term = Term::new().unwrap();
    ///     term.writer()
    ///         .foreground("rgb(64, 64, 128)")
    ///         .write(b" Look look look! I'm kinda blue now!\n")
    ///         .flush()
    ///         .err()
    ///         .unwrap();
    /// }
    /// ```
    pub fn writer<'a>(&'a self) -> TermWriter<'a, O> {
        TermWriter {
            info: &self.info,
            stdout: self.stdout.lock().unwrap(),
            written: 0,
            err: None,

            bold: false,
            dim: false,
            standout: false,
            italics: false,
            invert: false,
            blink: false,
            invisible: false,
            underline: false,

            foreground: None,
            background: None,
        }
    }

    pub fn print<T: AsRef<str>>(&self, s: T) -> Result<usize> {
        self.writer().print(s).done()
    }

    pub fn println<T: AsRef<str>>(&self, s: T) -> Result<usize> {
        self.writer().println(s).done()
    }

    /// Read from the terminal's standard input. Read into a fixed length buffer and return the number of characters read.
    /// Similar to `Term::write`, `read` does not need `Term` to be mutable, however only one thread may be reading at a time.
    ///
    /// # Examples
    /// ```
    /// use nixterm::term::Term;
    ///
    /// pub fn main() {
    ///     let term = Term::new().unwrap();
    ///     let mut buffer : [u8; 12] = [0; 12];
    ///     
    ///     // There's nothing to read! so read does nothing and returns 0.
    ///     assert_eq!(term.read(&mut buffer), 0);
    ///     assert_eq!(buffer, [0; 12]);
    /// }
    /// ```
    pub fn read(&self, buffer: &mut [u8]) -> usize {
        if self.err.borrow().is_none() {
            self.stdin
                .lock()
                .unwrap()
                .read(buffer)
                .context(ErrorKind::ReadFailed)
                .unwrap_or_else(|e| {
                    self.set_err(e);
                    0
                })
        } else {
            0
        }
    }

    pub fn readline(&self) -> Result<String> {
        let mut buf = String::new();
        self.stdin
            .lock()
            .unwrap()
            .read_line(&mut buf)
            .context(ErrorKind::ReadFailed)?;
        Ok(buf)
    }

    pub(crate) fn set_err<T: Into<Error>>(&self, e: T) {
        self.err.replace(Some(e.into()));
    }

    pub(crate) fn take_err(&self) -> Error {
        self.err.replace(None).unwrap()
    }

    pub(crate) fn has_err(&self) -> bool {
        self.err.borrow().is_some()
    }

    pub fn err(&self) -> Result<()> {
        if self.has_err() {
            Err(self.take_err())
        } else {
            Ok(())
        }
    }

    /// Execute a string field
    fn exec<'a>(&'a self, field: terminfo::StringField) -> Result<terminfo::lang::Executor<'a>> {
        match self.info.exec(field) {
            Some(v) => Ok(v),
            None => Err(ErrorKind::MissingTermInfoField(field).into()),
        }
    }

    /// Wrapper around exec, which immediately runs the string with no args and writes it to `O`.
    fn write_info_str(&self, field: terminfo::StringField) -> usize {
        match self.exec(field) {
            Ok(mut v) => v
                .write(self.stdout.lock().unwrap().deref_mut())
                .context(ErrorKind::FailedToRunTerminfo(field))
                .unwrap_or_else(|e| {
                    self.set_err(e);
                    0
                }),
            Err(e) => {
                self.err.replace(Some(
                    e.context(ErrorKind::FailedToRunTerminfo(field)).into(),
                ));
                0
            }
        }
    }

    pub fn settings(&self) -> Settings {
        Settings {
            termios: match termios::tcgetattr(self.as_raw_fd()) {
                Ok(v) => v,
                Err(e) => {
                    // This should be caught on the next `update`;
                    self.set_err(e.context(ErrorKind::FailedToSetTermios));
                    unsafe { termios::Termios::default_uninit() }
                }
            },
        }
    }

    pub fn update(&self, settings: Settings) -> Result<()> {
        self.err()?;

        termios::tcsetattr(
            self.as_raw_fd(),
            termios::SetArg::TCSAFLUSH,
            &settings.termios,
        ).context(ErrorKind::FailedToSetTermios)?;
        Ok(())
    }

    pub fn flush(&self) {
        match self.stdout.lock().unwrap().flush() {
            Ok(_) => (),
            Err(e) => self.set_err(e.context(ErrorKind::WriteFailed)),
        }
    }

    pub fn read_keys<'a>(&'a self) -> Keys<'a, I, O> {
        Keys::new(self)
    }

    pub fn clear_line_after_cursor(&self) {
        self.write_info_str(terminfo::ClrEol);
    }

    pub fn save_cursor(&self) {
        self.write_info_str(terminfo::SaveCursor);
    }

    pub fn restore_cursor(&self) {
        self.write_info_str(terminfo::RestoreCursor);
    }

    pub fn prompt<T: AsRef<str>>(&self, prompt: T) -> Result<String> {
        self.writer().print(prompt).done()?;
        self.readline()
    }

    #[inline]
    pub fn colors(&self) -> usize {
        // There has to be at least two colors... right???
        self.info.number(terminfo::MaxColors).unwrap_or(2) as usize
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

#[cfg(test)]
mod test {
    use std::io;
    use term::*;
    use terminfo;

    const TERMINFO: &'static [u8] = include_bytes!("../test-data/rxvt");

    struct FakeStdin {
        buffer: Vec<u8>,
    }

    struct FakeStdout {
        buffer: Vec<u8>,
    }

    impl FakeStdin {
        fn new() -> FakeStdin {
            FakeStdin { buffer: Vec::new() }
        }
    }

    impl<'a> AsRawFd for &'a mut FakeStdin {
        fn as_raw_fd(&self) -> RawFd {
            0
        }
    }

    impl FakeStdout {
        fn new() -> FakeStdout {
            FakeStdout { buffer: Vec::new() }
        }
    }

    impl<'a> AsRawFd for &'a mut FakeStdout {
        fn as_raw_fd(&self) -> RawFd {
            1
        }
    }

    impl<'a> io::Read for &'a mut FakeStdin {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let len = if buf.len() > self.buffer.len() {
                self.buffer.len()
            } else {
                buf.len()
            };

            self.buffer
                .drain(..len)
                .enumerate()
                .for_each(|(i, c)| buf[i] = c);
            Ok(len)
        }
    }

    impl<'a> io::Write for &'a mut FakeStdout {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buffer.extend(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn term() {
        let mut stdin = FakeStdin::new();
        let mut stdout = FakeStdout::new();
        {
            let term = Term::from_streams(
                terminfo::TermInfo::parse(TERMINFO).unwrap().into(),
                &mut stdin,
                &mut stdout,
            );
            term.writer().bold().print("Hello World?").done().unwrap();
        }
        assert_eq!(&stdout.buffer, b"\x1b[0;1mHello World?\x1b[m\x0F");
    }

    #[test]
    fn print() {
        use std::str::FromStr;

        let mut stdin = FakeStdin::new();
        let mut stdout = FakeStdout::new();
        {
            let term = Term::from_streams(
                terminfo::TermInfo::parse(TERMINFO).unwrap().into(),
                &mut stdin,
                &mut stdout,
            );
            term.writer()
                .bold()
                .print("Hello")
                .foreground(ansi::Color::from_str("red").unwrap())
                .print("World")
                .print("?")
                .done()
                .unwrap();
        }
        assert_eq!(
            String::from_utf8(stdout.buffer.clone()).unwrap(),
            "\x1b[0;1mHello\x1b[0m \x1b[31mWorld\x1b[0m?"
        );
        stdout.buffer.clear();

        {
            let term = Term::from_streams(
                terminfo::TermInfo::parse(TERMINFO).unwrap().into(),
                &mut stdin,
                &mut stdout,
            );
            term.writer()
                .bold()
                .print("Hello")
                .foreground(ansi::Color::from_str("red").unwrap())
                .bold()
                .print("World")
                .print("?")
                .done()
                .unwrap();
        }
        assert_eq!(
            String::from_utf8(stdout.buffer).unwrap(),
            "\x1b[1m\x1b[31mHi\x1b[39m\x1b[22m?"
        );
    }
}
