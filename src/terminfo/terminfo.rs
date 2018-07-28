use std::mem;
use terminfo::errors::*;
use terminfo::fields::*;
use terminfo::util;
use terminfo::util::read_le_u16;

/// TermInfo is immutable terminfo data.
///
/// To mutate `TermInfo` use `TermInfo::into<TermInfoBuf>()` to get a `TermInfoBuf` struct.
/// Generally, the `TermInfo` struct is good for quickly peeking at terminfo file. It is fast,
/// and makes no allocations, however it cannot be modified, and is difficult to use over a long period of time,
/// since the buffer it parsed has to live along side it.
#[derive(Debug, Clone)]
pub struct TermInfo<'a> {
    names: &'a [u8],

    bools: &'a [u8],
    numbers: &'a [u8],
    strings: &'a [u8],
    strtab: util::StrTable<'a>,

    ext: Option<TermInfoExt<'a>>,
}

/// A wrapper around the extended part of a terminfo file.
/// This structure is just used to keep the code clean internally,
/// is exposed to users through the `TermInfo` struct.
#[derive(Debug, Clone)]
pub(crate) struct TermInfoExt<'a> {
    bools: &'a [u8],
    numbers: &'a [u8],
    strings: &'a [u8],
    names: &'a [u8],

    strtab: util::StrTable<'a>,
    nametab_start: usize,
}

/// Split a terminfo file into the fields of a `terminfo` struct.
///
/// This function hardly analyzes the data at all, it just finds each section
/// and creates a new terminfo struct based on those sections.
fn split_terminfo<'a>(bytes: &'a [u8]) -> Result<TermInfo<'a>> {
    let file_size = bytes.len();

    // Terminfo files start with a 12-byte header, made up of 6 16-bit fields.
    if file_size < 12 {
        return Err(ErrorKind::IncompleteTermInfoHeader.into());
    }

    if read_le_u16(bytes, 0) != 0o432 {
        return Err(ErrorKind::InvalidMagicNumber.into());
    }

    // following the magic there is a series of lengths for each section
    let names_size = read_le_u16(bytes, 1) as usize;
    let bools_count = read_le_u16(bytes, 2) as usize;
    let numbers_count = read_le_u16(bytes, 3) as usize;
    let strings_count = read_le_u16(bytes, 4) as usize;
    let strtab_size = read_le_u16(bytes, 5) as usize;

    let mut expected_filesize =
        12 + bools_count + numbers_count * 2 + strings_count * 2 + strtab_size + names_size;
    if bools_count + names_size % 2 != 0 {
        expected_filesize += 1;
    }

    // make sure the file's length and the section lengths match up
    // use <= here because they length might not match exactly, because of extensions
    if expected_filesize > file_size {
        return Err(ErrorKind::IncompleteTermInfo.into());
    }

    let mut slice = &bytes[12..];

    // subtract one so we ignore the null terminator
    let names = &slice[..names_size - 1];
    slice = &slice[names_size..];

    let bools = &slice[..bools_count];

    // 2 byte align
    slice = if (bools_count + names_size) % 2 != 0 {
        &slice[bools_count + 1..]
    } else {
        &slice[bools_count..]
    };

    let numbers = &slice[..numbers_count * 2];
    slice = &slice[numbers_count * 2..];

    let strings = &slice[..strings_count * 2];
    slice = &slice[strings_count * 2..];

    let strtab = &slice[..strtab_size];
    slice = if strtab_size % 2 != 0 {
        &slice[strtab_size + 1..]
    } else {
        &slice[strtab_size..]
    };

    let ext = if expected_filesize < file_size {
        Some(split_terminfo_ext(slice)?)
    } else {
        None
    };

    Ok(TermInfo {
        names: names,
        bools: bools,
        numbers: numbers,
        strings: strings,
        strtab: util::StrTable::new(strtab),
        ext: ext,
    })
}

fn split_terminfo_ext<'a>(bytes: &'a [u8]) -> Result<TermInfoExt<'a>> {
    let file_size = bytes.len();

    if file_size < 10 {
        return Err(ErrorKind::IncompleteExtendedHeader.into());
    }

    let bools_count = read_le_u16(bytes, 0) as usize;
    let numbers_count = read_le_u16(bytes, 1) as usize;
    let strings_count = read_le_u16(bytes, 2) as usize;
    let strtab_size = read_le_u16(bytes, 3) as usize;
    let strtab_last_offset = read_le_u16(bytes, 4) as usize;

    let names_count = strings_count + numbers_count + bools_count;

    let mut expected_filesize =
        10 + bools_count + numbers_count * 2 + strings_count * 2 + names_count * 2 + strtab_size;
    if bools_count % 2 != 0 {
        expected_filesize += 1;
    }

    if expected_filesize > file_size {
        return Err(ErrorKind::IncompleteExtendedTermInfo.into());
    }

    let mut slice = &bytes[10..];
    let bools = &slice[..bools_count];

    // align the pointer to 2 bytes
    slice = if bools_count % 2 != 0 {
        &slice[bools_count + 1..]
    } else {
        &slice[bools_count..]
    };

    let numbers = &slice[..numbers_count * 2];
    slice = &slice[numbers_count * 2..];

    let strings = &slice[..strings_count * 2];
    slice = &slice[strings_count * 2..];

    let names = &slice[..names_count * 2];
    slice = &slice[names_count * 2..];

    let strtab = &slice[..strtab_last_offset];

    // This is comically slow. It accounts for like 95% of this functions runtime.
    // but I can't find another way to do it so whatever.
    let mut x = names_count + 1;
    let nametab_offset = strtab_last_offset
        - strtab
            .iter()
            .rev()
            .take_while(|&&c| {
                if c == 0 {
                    x -= 1
                }
                x != 0
            })
            .count();

    Ok(TermInfoExt {
        bools: bools,
        numbers: numbers,
        strings: strings,
        strtab: util::StrTable::new(strtab),
        nametab_start: nametab_offset,
        names: names,
    })
}

impl<'a> TermInfoExt<'a> {
    pub(crate) fn get_tables(&self) -> (util::StringTable, util::StringTable) {
        self.strtab.split(self.nametab_start)
    }

    pub(crate) fn get_numbers(&self) -> Vec<u16> {
        self.numbers.chunks(2).map(|n| read_le_u16(n, 0)).collect()
    }

    pub(crate) fn get_string_offsets(&self) -> Vec<u16> {
        self.strings.chunks(2).map(|n| read_le_u16(n, 0)).collect()
    }

    pub(crate) fn get_name_offsets(&self) -> Vec<u16> {
        self.names.chunks(2).map(|n| read_le_u16(n, 0)).collect()
    }

    pub(crate) fn get_bools(&self) -> Vec<bool> {
        self.bools.iter().map(|&b| b != 0).collect()
    }
}

impl<'a> TermInfo<'a> {
    /// Parse a terminfo file from a buffer.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io;
    /// use std::io::prelude::*;
    /// use std::fs::File;
    /// use nixterm::terminfo;
    ///
    /// fn main() {
    ///     let mut data = Vec::new();
    ///
    ///     File::open("/usr/share/terminfo/x/xterm")
    ///         .unwrap()
    ///         .read_to_end(&mut data);
    ///     let info = terminfo::TermInfo::parse(&data).unwrap();
    ///
    ///     assert_eq!(info.boolean(terminfo::AutoLeftMargin), false);
    /// }
    /// ```
    ///
    pub fn parse(bytes: &'a [u8]) -> Result<TermInfo<'a>> {
        split_terminfo(bytes)
    }

    /// Get an iterator over the terminal's name(s)
    ///
    /// The first name is generally the primary one, for example XTerm's first name is "xterm".
    /// The following names will usually be longer. Sometimes they will describe this terminal (e.g. linux-16color's second name is "linux console with 16 colors").
    /// Other names may expand on the first name if it is an acronym (e.g. kitty's second name is "KovIdTTY").
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.names
            .split(|&c| c == b'|')
            .map(|slice| unsafe { mem::transmute::<&[u8], &str>(slice) })
    }

    pub(crate) fn get_strtab(&self) -> util::StringTable {
        self.strtab.to_string_table()
    }

    pub(crate) fn get_numbers(&self) -> Vec<u16> {
        self.numbers.chunks(2).map(|n| read_le_u16(n, 0)).collect()
    }

    pub(crate) fn get_string_offsets(&self) -> Vec<u16> {
        self.strings.chunks(2).map(|n| read_le_u16(n, 0)).collect()
    }

    pub(crate) fn get_bools(&self) -> Vec<bool> {
        self.bools.iter().map(|&b| b != 0).collect()
    }

    pub(crate) fn get_ext(&self) -> &Option<TermInfoExt> {
        &self.ext
    }

    pub(crate) fn ext_index<T: AsRef<str>>(&self, s: T) -> Option<usize> {
        match &self.ext {
            Some(e) => {
                let bytes = s.as_ref().as_bytes();
                for (i, x) in e.names.chunks(2).enumerate() {
                    let num = read_le_u16(x, 0) as usize;
                    if e.strtab.get_iter(num + e.nametab_start).eq(bytes) {
                        return Some(i);
                    }
                }
                None
            }
            None => None,
        }
    }

    /// Get a numeric field.
    ///
    /// Not all terminals will include a value for every field enumerated in `NumericField`.
    pub fn number(&self, field: NumericField) -> Option<u16> {
        let i = field as usize;

        if i * 2 < self.numbers.len() {
            let number = read_le_u16(self.numbers, i);
            if number != util::invalid() {
                Some(number)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get a boolean field.
    ///
    /// Not all terminals will include a value for every field enumerated in `BooleanField`. `boolean` will return false if a value is missing.
    pub fn boolean(&self, field: BooleanField) -> bool {
        let i = field as usize;

        if i < self.bools.len() {
            self.bools[i] != 0
        } else {
            false
        }
    }

    /// Get a string field.
    ///
    /// Not all terminals will include a value for every field enumerated in `StringField`.
    pub fn string(&self, field: StringField) -> Option<&str> {
        let i = field as usize;

        if i * 2 < self.strings.len() {
            let offset = read_le_u16(self.strings, i);
            if offset != util::invalid() {
                return Some(self.strtab.get(offset as usize).unwrap());
            }
        }
        None
    }

    /// Check if the the terminfo file has an extensions section
    ///
    /// If this method returns false then the `TermInfo::ext_*` methods won't fail. However `TermInfo::ext_boolean`
    /// will always return false, and `TermInfo::ext_number` and `TermInfo::ext_string` will always return None.
    pub fn has_ext(&self) -> bool {
        self.ext.is_some()
    }

    /// This method is identical to `TermInfo::boolean`, except the boolean is identified with a string.
    pub fn ext_boolean<T: AsRef<str>>(&self, field: T) -> bool {
        if let Some(ref ext) = self.ext {
            if let Some(idx) = self.ext_index(field) {
                if idx < ext.bools.len() {
                    return ext.bools[idx] != 0;
                }
            }
        }
        false
    }

    /// This method is identified to `Terminfo::number`, except the number is identified by a string.
    pub fn ext_number<T: AsRef<str>>(&self, field: T) -> Option<u16> {
        if let Some(ref ext) = self.ext {
            if let Some(idx) = self.ext_index(field) {
                let idx_offset = ext.bools.len();
                if idx >= idx_offset && idx - idx_offset < ext.numbers.len() {
                    let num = read_le_u16(ext.numbers, idx - idx_offset);
                    if num != util::invalid() {
                        return Some(num);
                    }
                }
            }
        }
        None
    }

    /// This method is identified to `Terminfo::string`, except the string is identified by a string.
    pub fn ext_string<T: AsRef<str>>(&self, field: T) -> Option<&str> {
        if let Some(ref ext) = self.ext {
            if let Some(idx) = self.ext_index(field) {
                let idx_offset = ext.bools.len() + (ext.numbers.len() / 2);
                if idx >= idx_offset && idx - idx_offset < (ext.strings.len() / 2) {
                    let num = read_le_u16(ext.strings, idx - idx_offset);
                    if num != util::invalid() {
                        return ext.strtab
                            .get(num as usize)
                            .map(|x| Some(x))
                            .unwrap_or(None);
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod test {
    const RXVT_INFO: &'static [u8] = include_bytes!("../../test-data/rxvt");
    const XTERM_INFO: &'static [u8] = include_bytes!("../../test-data/xterm");
    const LINUX_16COLOR_INFO: &'static [u8] = include_bytes!("../../test-data/linux-16color");

    use terminfo::*;

    #[test]
    fn names() {
        let rxvt = TermInfo::parse(RXVT_INFO).unwrap();

        assert_eq!(rxvt.names().nth(0), Some("rxvt"));
        assert_eq!(
            rxvt.names().collect::<Vec<&str>>(),
            vec!["rxvt", "rxvt terminal emulator (X Window System)"]
        );
    }

    #[test]
    fn lookup_string() {
        let rxvt = TermInfo::parse(RXVT_INFO).unwrap();
        let xterm = TermInfo::parse(XTERM_INFO).unwrap();
        let l16c = TermInfo::parse(LINUX_16COLOR_INFO).unwrap();

        assert_eq!(rxvt.string(StringField::KeyDown), Some("\u{1b}[B"));
        assert_eq!(rxvt.string(StringField::KeyBackspace), Some("\u{8}"));
        assert_eq!(rxvt.string(StringField::ZeroMotion), None);
        assert_eq!(rxvt.string(StringField::Bell), Some("\u{7}"));

        assert_eq!(xterm.string(StringField::Newline), None);
        assert_eq!(xterm.string(StringField::MetaOff), Some("\u{1b}[?1034l"));
        assert_eq!(xterm.string(StringField::KeyEnter), Some("\u{1b}OM"));
        assert_eq!(xterm.string(StringField::LinefeedIfNotLf), None);

        assert_eq!(l16c.string(StringField::LabF3), None);
        assert_eq!(l16c.string(StringField::InsertLine), Some("\u{1b}[L"));
        assert_eq!(l16c.string(StringField::GetMouse), None);
        assert_eq!(l16c.string(StringField::LinefeedIfNotLf), None);
    }

    #[test]
    fn lookup_bool() {
        let rxvt = TermInfo::parse(RXVT_INFO).unwrap();
        let xterm = TermInfo::parse(XTERM_INFO).unwrap();
        let l16c = TermInfo::parse(LINUX_16COLOR_INFO).unwrap();

        assert_eq!(rxvt.boolean(BooleanField::LinefeedIsNewline), false);
        assert_eq!(rxvt.boolean(BooleanField::AutoRightMargin), true);

        assert_eq!(xterm.boolean(BooleanField::AutoLeftMargin), false);
        assert_eq!(xterm.boolean(BooleanField::LinefeedIsNewline), false);

        assert_eq!(l16c.boolean(BooleanField::HardCursor), false);
        assert_eq!(l16c.boolean(BooleanField::AutoRightMargin), true);
    }

    #[test]
    fn lookup_number() {
        let rxvt = TermInfo::parse(RXVT_INFO).unwrap();
        let xterm = TermInfo::parse(XTERM_INFO).unwrap();
        let l16c = TermInfo::parse(LINUX_16COLOR_INFO).unwrap();

        assert_eq!(rxvt.number(NumericField::Columns), Some(80));
        assert_eq!(rxvt.number(NumericField::Buttons), None);

        assert_eq!(xterm.number(NumericField::WideCharSize), None);
        assert_eq!(xterm.number(NumericField::MaxColors), Some(8));

        assert_eq!(l16c.number(NumericField::BitImageEntwining), None);
        assert_eq!(l16c.number(NumericField::MaxColors), Some(16));
    }

    #[test]
    fn lookup_ext_string() {
        let xterm = TermInfo::parse(XTERM_INFO).unwrap();

        assert_eq!(xterm.has_ext(), true);
        assert_eq!(xterm.ext_string("kUP7"), Some("\u{1b}[1;7A"));
        assert_eq!(xterm.ext_string("kUP8"), None);
    }

    #[test]
    fn lookup_ext_bool() {
        let rxvt = TermInfo::parse(RXVT_INFO).unwrap();

        assert_eq!(rxvt.has_ext(), true);
        assert_eq!(rxvt.ext_boolean("XM"), false);
        assert_eq!(rxvt.ext_boolean("G0"), false);
        assert_eq!(rxvt.ext_boolean("XT"), true);
        assert_eq!(rxvt.ext_boolean("kUP"), false);
    }

    #[test]
    fn lookup_ext_number() {
        let l16c = TermInfo::parse(LINUX_16COLOR_INFO).unwrap();

        assert_eq!(l16c.has_ext(), true);
        assert_eq!(l16c.ext_number("U8"), Some(1));
    }

}
