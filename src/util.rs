//! Private utility functions and structures
use errors::*;
use failure::ResultExt;
use std::io;
use std::mem::{size_of, transmute};
use std::slice::from_raw_parts;

const WORD_SIZE: usize = size_of::<usize>();
const DIGITS: [u8; 36] = *b"0123456789abcdefghijklmnopqrstuvwxyz";

/// Read a little endian u16 from a u8 slice.
///
/// `i` is the u16's offset, in two-byte blocks (i.e. start reading at b[i * 2]).
/// `b` must be _at least_ 2 bytes long.
#[inline(always)]
pub fn read_le_u16(b: &[u8], i: usize) -> u16 {
    if b.len() < 2 {
        return invalid();
    }
    u16::from_le(unsafe { (&*(b as *const [u8] as *const [u16]))[i] })
}

/// Read a little endian u32 from a u8 slice.
///
/// `i` is the u32's offset, in four-byte blocks (i.e. start reading at b[i * 4]).
/// `b` must be _at least_ 4 bytes long.
#[inline(always)]
pub fn read_le_u32(b: &[u8], i: usize) -> u32 {
    if b.len() < 4 {
        return invalid();
    }

    u32::from_le(unsafe { (&*(b as *const [u8] as *const [u32]))[i] })
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
    if unsafe { transmute::<*const u8, usize>(s as *const [u8] as *const u8) } % WORD_SIZE == 0 {
        let word_slice = unsafe {
            from_raw_parts(
                (s as *const [u8] as *const u8) as *const usize,
                s.len() / WORD_SIZE,
            )
        };

        for (i, c) in word_slice.iter().enumerate() {
            if has_null_byte(*c) {
                return i * WORD_SIZE + find_zero(&s[i * WORD_SIZE..]);
            }
        }
        s.len()
    } else {
        find_zero(s)
    }
}

/// Write a u8 in a ansi-escape code compatible format
#[inline]
pub fn write_u8_ansi<W: io::Write>(w: &mut W, num: u8) -> Result<usize> {
    let mut num_buf = [0u8; 3];
    let mut num_buf_len = 0;
    let mut num = num;

    if num == 0 {
        return Ok(0);
    }

    while num > 0 {
        let c = num % 10;
        num /= 10;
        num_buf[num_buf_len] = DIGITS[c as usize];
        num_buf_len += 1;
    }
    if num_buf_len > 1 {
        num_buf[0] ^= num_buf[num_buf_len - 1];
        num_buf[num_buf_len - 1] ^= num_buf[0];
        num_buf[0] ^= num_buf[num_buf_len - 1];
    }
    w.write(&num_buf[..num_buf_len])
        .context(ErrorKind::FailedWriteToStdout)?;
    Ok(num_buf_len)
}

#[cfg(test)]
mod test {
    use util;

    #[test]
    fn has_null_bytes() {
        assert_eq!(util::has_null_byte(0xFF00FFFFFFFFFFFF), true);
        assert_eq!(util::has_null_byte(0xFFFFFFFFFFFFFF00), true);
        assert_eq!(util::has_null_byte(0xFFFFFFFF00FFFFFF), true);
        assert_eq!(util::has_null_byte(0xFF00FFFFFFFFFFFF), true);
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
