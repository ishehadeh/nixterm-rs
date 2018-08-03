use errors::*;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::io;
use std::os::unix::io::AsRawFd;
use term;
use terminfo;

const FUNC_KEYS_KEY: [terminfo::StringField; 64] = [
    terminfo::StringField::KeyF0,
    terminfo::StringField::KeyF1,
    terminfo::StringField::KeyF2,
    terminfo::StringField::KeyF3,
    terminfo::StringField::KeyF4,
    terminfo::StringField::KeyF5,
    terminfo::StringField::KeyF6,
    terminfo::StringField::KeyF7,
    terminfo::StringField::KeyF8,
    terminfo::StringField::KeyF9,
    terminfo::StringField::KeyF10,
    terminfo::StringField::KeyF11,
    terminfo::StringField::KeyF12,
    terminfo::StringField::KeyF13,
    terminfo::StringField::KeyF14,
    terminfo::StringField::KeyF15,
    terminfo::StringField::KeyF16,
    terminfo::StringField::KeyF17,
    terminfo::StringField::KeyF18,
    terminfo::StringField::KeyF19,
    terminfo::StringField::KeyF20,
    terminfo::StringField::KeyF21,
    terminfo::StringField::KeyF22,
    terminfo::StringField::KeyF23,
    terminfo::StringField::KeyF24,
    terminfo::StringField::KeyF25,
    terminfo::StringField::KeyF26,
    terminfo::StringField::KeyF27,
    terminfo::StringField::KeyF28,
    terminfo::StringField::KeyF29,
    terminfo::StringField::KeyF30,
    terminfo::StringField::KeyF31,
    terminfo::StringField::KeyF32,
    terminfo::StringField::KeyF33,
    terminfo::StringField::KeyF34,
    terminfo::StringField::KeyF35,
    terminfo::StringField::KeyF36,
    terminfo::StringField::KeyF37,
    terminfo::StringField::KeyF38,
    terminfo::StringField::KeyF39,
    terminfo::StringField::KeyF40,
    terminfo::StringField::KeyF41,
    terminfo::StringField::KeyF42,
    terminfo::StringField::KeyF43,
    terminfo::StringField::KeyF44,
    terminfo::StringField::KeyF45,
    terminfo::StringField::KeyF46,
    terminfo::StringField::KeyF47,
    terminfo::StringField::KeyF48,
    terminfo::StringField::KeyF49,
    terminfo::StringField::KeyF50,
    terminfo::StringField::KeyF51,
    terminfo::StringField::KeyF52,
    terminfo::StringField::KeyF53,
    terminfo::StringField::KeyF54,
    terminfo::StringField::KeyF55,
    terminfo::StringField::KeyF56,
    terminfo::StringField::KeyF57,
    terminfo::StringField::KeyF58,
    terminfo::StringField::KeyF59,
    terminfo::StringField::KeyF60,
    terminfo::StringField::KeyF61,
    terminfo::StringField::KeyF62,
    terminfo::StringField::KeyF63,
];

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Key {
    /// the value of Fn may be between 0 - 63.
    Fn(usize),
    Char(char),
    Up,
    Down,
    Left,
    Tab,
    Right,
    Delete,
    Backspace,
    Escape,
    Enter,
    Begin,
    End,
    Clear,
    Exit,
    Backtab,
    KeypadA1,
    KeypadA3,
    KeypadB2,
    KeypadC1,
    KeypadC3,
    Control(char),
    Invalid(u8),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Event {
    ScrollUp(usize),
    ScrollDonw(usize),
    Key(Key),
}

pub struct Keys<'a, I, O>
where
    I: io::Read + AsRawFd + 'a,
    O: io::Write + AsRawFd + 'a,
{
    // Keys may need to be buffered if we have to back out of an escape code
    buffer: VecDeque<Key>,
    unread: VecDeque<u8>,
    map: HashMap<&'a str, Key>,
    tty: &'a term::Term<I, O>,
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

impl<'a, I, O> Keys<'a, I, O>
where
    I: io::Read + AsRawFd + 'a,
    O: io::Write + AsRawFd + 'a,
{
    pub fn new(t: &'a term::Term<I, O>) -> Keys<'a, I, O> {
        let mut keys = Keys {
            buffer: VecDeque::new(),
            unread: VecDeque::new(),
            tty: t,
            map: HashMap::new(),
        };
        keys.make_keymap();
        if let Some(v) = keys.tty.info.string(terminfo::KeypadXmit) {
            keys.tty.writer().write_bytes(v.as_bytes());
            keys.tty.flush();
        }
        keys
    }

    fn string_to_key(&mut self, key: Key, field: terminfo::StringField) {
        self.map
            .insert(self.tty.info.string(field).unwrap_or(""), key);
    }

    fn make_keymap(&mut self) {
        self.string_to_key(Key::Backspace, terminfo::KeyBackspace);
        self.string_to_key(Key::Backtab, terminfo::BackTab);
        self.string_to_key(Key::Begin, terminfo::KeyBeg);
        self.string_to_key(Key::End, terminfo::KeyEnd);
        self.string_to_key(Key::Clear, terminfo::KeyClear);
        self.string_to_key(Key::Exit, terminfo::KeyExit);
        self.string_to_key(Key::KeypadC1, terminfo::KeyC1);
        self.string_to_key(Key::KeypadC3, terminfo::KeyC3);
        self.string_to_key(Key::KeypadB2, terminfo::KeyB2);
        self.string_to_key(Key::KeypadA3, terminfo::KeyA3);
        self.string_to_key(Key::KeypadA1, terminfo::KeyA1);
        self.string_to_key(Key::Up, terminfo::KeyUp);
        self.string_to_key(Key::Down, terminfo::KeyDown);
        self.string_to_key(Key::Left, terminfo::KeyLeft);
        self.string_to_key(Key::Right, terminfo::KeyRight);
        self.string_to_key(Key::Up, terminfo::ScrollForward);
        self.string_to_key(Key::Down, terminfo::ScrollReverse);

        FUNC_KEYS_KEY.iter().enumerate().for_each(|(i, &x)| {
            self.map
                .insert(self.tty.info.string(x).unwrap_or(""), Key::Fn(i));
        });
    }

    fn getch(&mut self) -> Option<u8> {
        if let Some(v) = self.unread.pop_front() {
            return Some(v);
        }

        let mut c: [u8; 1] = [0; 1];
        let read = self.tty.read(&mut c);
        if read == 0 {
            None
        } else {
            Some(c[0])
        }
    }

    fn getkey_esc(&mut self) -> Result<Key> {
        let mut read = 1;
        let mut possible_keys = self.map.clone();
        let mut c: [u8; 1] = [0; 1];

        while possible_keys.len() > 0 {
            let mut read_len = self.tty.read(&mut c);

            while read_len < 1 {
                read_len = self.tty.read(&mut c);
            }

            possible_keys.retain(|k, _| k.bytes().nth(read) == Some(c[0]));

            read += 1;
            self.unread.push_back(c[0]);

            if let Some(k) = possible_keys
                .iter()
                .filter(|(k, _)| k.len() == read)
                .map(|(_, v)| v)
                .next()
            {
                self.unread.clear();
                return Ok(k.clone());
            }
        }

        Ok(Key::Escape)
    }

    fn getkey(&mut self) -> Result<Key> {
        self.tty.err()?;

        let mut c = self.getch();
        while c.is_none() {
            c = self.getch();
        }
        let ch = c.unwrap();

        Ok(match ch {
            0...8 | 10...12 | 14...26 | 28...31 => Key::Control((ch + 64) as char),
            9 => Key::Tab,
            13 => Key::Enter,
            27 => self.getkey_esc()?,
            127 => Key::Delete,
            32...126 => Key::Char(ch as char),
            _ => Key::Invalid(ch),
        })
    }
}

impl<'a, I, O> Drop for Keys<'a, I, O>
where
    I: io::Read + AsRawFd + 'a,
    O: io::Write + AsRawFd + 'a,
{
    fn drop(&mut self) {
        if let Some(v) = self.tty.info.string(terminfo::KeypadLocal) {
            self.tty.writer().write_bytes(v.as_bytes());
            self.tty.flush();
        }
    }
}
