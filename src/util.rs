//! Private utility functions and structures
use errors::*;
use failure::ResultExt;
use memchr::memchr;
use std::io;
use std::mem::{size_of, transmute};
use std::slice::from_raw_parts;

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
/// I'm waiting for `const fn` to be stabilized before using it here, but
/// these should really end up compiling to a literal in most cases anyway so it's not a _huge_ concern.  
#[inline(always)]
pub fn invalid<T>() -> T
where
    T: From<u16>,
{
    T::from(65535)
}

/// Return the number of bytes before the first instance of a null byte in `s`, or s.len() if no null byte is found
#[inline]
pub fn strlen(s: &[u8]) -> usize {
    memchr(0, s).unwrap_or_else(|| s.len())
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
