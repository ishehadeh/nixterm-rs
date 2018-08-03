use failure::Fail;
use std::mem::transmute;
use std::ptr::write_bytes;
use terminfo::errors::*;
use util::strlen;

#[derive(Debug, Clone)]
pub struct StringTable {
    pub(crate) table: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct StrTable<'a> {
    pub(crate) table: &'a [u8],
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
            write_bytes(slice.as_mut_ptr(), 255, len - 1);
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
