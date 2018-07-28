use errors::*;
use failure::Fail;
use std::mem::size_of;
use std::mem::transmute;
use std::ptr;
use std::slice;
use std::str;

const WORD_SIZE: usize = size_of::<usize>();

#[derive(Debug, Clone)]
pub struct StringTable {
    pub(crate) table: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct StrTable<'a> {
    pub(crate) table: &'a [u8],
}

/// Check if there is at least one NULL byte in `num`
#[inline(always)]
pub fn has_null_byte(num: usize) -> bool {
    const HI_BITS: usize = (isize::min_value() as usize) / 255;
    const LO_BITS: usize = (HI_BITS >> 7);

    (num.wrapping_sub(LO_BITS) & !num & HI_BITS) != 0
}

/// iterate byte-by-byte to find a zero in a u8 array
#[inline]
fn find_zero(s: &[u8]) -> usize {
    for (i, c) in s.iter().enumerate() {
        if *c == 0 {
            return i;
        }
    }
    s.len()
}

/// Return the number of bytes before the first instance of a null byte in `s`, or s.len() if no null byte was found/
#[inline]
pub fn strlen(s: &[u8]) -> usize {
    let offset = s.len() % WORD_SIZE;

    for (i, c) in s.iter().take(offset).enumerate() {
        if *c == 0 {
            return i;
        }
    }

    let word_slice = unsafe {
        slice::from_raw_parts(
            (s as *const [u8] as *const u8).offset(offset as isize) as *const usize,
            (s.len() - offset) / WORD_SIZE,
        )
    };

    for (i, c) in word_slice.iter().enumerate() {
        if has_null_byte(*c) {
            let b = offset + i * WORD_SIZE;
            return b + find_zero(&s[b..]);
        }
    }
    if s.last() == Some(&0) {
        s.len() - 1
    } else {
        s.len()
    }
}

/// Read a little endian u16 from a u8 slice.
///
/// `i` is the u16's offset, in two-byte blocks (i.e. start reading at b[i * 2]).
/// `b` must be _at least_ 2 bytes long.
#[inline(always)]
pub fn read_le_u16(b: &[u8], i: usize) -> u16 {
    if b.len() < 2 {
        return !0;
    }
    // this is a doozy of a line, although rust makes this look way more complicated than it is.
    // all this is doing is casting b to a u16 array, then indexing it with i, and swapping the bytes if
    // this is a big endian machine.
    u16::from_le(unsafe { (&*(b as *const [u8] as *const [u16]))[i] })
}

/// Generic function to get the terminfo INVALID value.
///
/// I'm waiting for `const fn` to be stabalized before using it here, but
/// these should really end up compiling to a literal in most cases anyway so it's not a _huge_ concern.  
#[inline(always)]
pub fn invalid<T>() -> T
where
    T: From<u16>,
{
    T::from(65535)
}

impl StringTable {
    pub fn from_slice(s: &[u8]) -> StringTable {
        StringTable {
            table: Vec::from(s),
        }
    }
    pub fn new() -> StringTable {
        StringTable { table: Vec::new() }
    }

    pub fn add<T: AsRef<str>>(&mut self, s: T) -> usize {
        let start = self.table.len();
        self.table.extend(s.as_ref().bytes());
        self.table.push(0);
        start
    }

    #[inline]
    pub fn get(&self, offset: usize) -> Result<&str> {
        if offset > self.table.len() {
            return Err(ErrorKind::OutOfRange(offset, self.table.len())
                .context(ErrorKind::FailedToReadStringFromTable)
                .into());
        }

        let slice = &self.table[offset..];

        Ok(unsafe { transmute(&slice[..strlen(slice)]) })
    }

    #[inline]
    pub fn get_iter(&self, offset: usize) -> impl Iterator<Item = &u8> {
        self.table.iter().skip(offset).take_while(|&&c| c != 0)
    }

    #[inline]
    pub fn get_slice(&self, offset: usize) -> Result<&[u8]> {
        if offset > self.table.len() {
            return Err(ErrorKind::OutOfRange(offset, self.table.len())
                .context(ErrorKind::FailedToReadStringFromTable)
                .into());
        }

        let slice = &self.table[offset..];
        Ok(&slice[..strlen(slice)])
    }

    /// This function will be useful in the future
    #[allow(dead_code)]
    pub fn del(&mut self, offset: usize) -> Result<()> {
        if offset > self.table.len() {
            return Err(ErrorKind::OutOfRange(offset, self.table.len())
                .context(ErrorKind::FailedToReadStringFromTable)
                .into());
        }

        let slice = &mut self.table[offset..];
        let len = strlen(slice);

        unsafe {
            ptr::write_bytes(slice.as_mut_ptr(), 255, len - 1);
        }
        Ok(())
    }
}

impl<'a> StrTable<'a> {
    pub fn new(src: &'a [u8]) -> StrTable<'a> {
        StrTable { table: src }
    }

    #[inline]
    pub fn get(&self, offset: usize) -> Result<&str> {
        if offset > self.table.len() {
            return Err(ErrorKind::OutOfRange(offset, self.table.len())
                .context(ErrorKind::FailedToReadStringFromTable)
                .into());
        }

        let slice = &self.table[offset..];

        Ok(unsafe { transmute(&slice[..strlen(slice)]) })
    }

    #[inline]
    pub fn get_iter(&self, offset: usize) -> impl Iterator<Item = &u8> {
        self.table.iter().skip(offset).take_while(|&&c| c != 0)
    }

    pub fn to_string_table(&self) -> StringTable {
        StringTable {
            table: Vec::from(self.table),
        }
    }

    pub fn split(&self, idx: usize) -> (StringTable, StringTable) {
        (
            StringTable::from_slice(&self.table[..idx]),
            StringTable::from_slice(&self.table[idx..]),
        )
    }
}

#[cfg(test)]
mod test {
    use util;

    #[test]
    fn has_null_bytes() {
        assert_eq!(util::has_null_byte(0xFF00FFFF), true);
        assert_eq!(util::has_null_byte(!0), false);
        assert_eq!(
            util::has_null_byte(
                0b01010101010101010101010101010101010101010101010101010101010101010
            ),
            false
        );
        assert_eq!(
            util::has_null_byte(
                0b01010101010101000000101010101010101010101010101010101010101010101
            ),
            false
        );
        assert_eq!(
            util::has_null_byte(
                0b01010101010101000001010101010101010101010101010101010101010101010
            ),
            false
        );
        assert_eq!(
            util::has_null_byte(0b010101010101010000000001110101010101010101010101010101010101010),
            true
        );
    }
}
