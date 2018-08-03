use terminfo::errors::*;
use terminfo::fields::*;
use terminfo::strtab::StringTable;
use terminfo::{lang, TermInfo};
use util::invalid;

/// The owning, mutable version of `TermInfo`
#[derive(Debug, Clone)]
pub struct TermInfoBuf {
    pub names: Vec<String>,
    bools: Vec<bool>,
    numbers: Vec<u32>,
    strings: Vec<u16>,
    strtab: StringTable,

    ext: Option<TermInfoExtBuf>,
}

#[derive(Debug, Clone)]
struct TermInfoExtBuf {
    bools: Vec<bool>,
    numbers: Vec<u32>,
    strings: Vec<u16>,
    names: Vec<u16>,
    strtab: StringTable,
    nametab: StringTable,
}

impl TermInfoExtBuf {
    fn new() -> TermInfoExtBuf {
        TermInfoExtBuf {
            strtab: StringTable::new(),
            nametab: StringTable::new(),
            names: Vec::new(),
            bools: Vec::new(),
            numbers: Vec::new(),
            strings: Vec::new(),
        }
    }
}

impl TermInfoBuf {
    pub fn from_terminfo(ti: &TermInfo) -> TermInfoBuf {
        let mut tib = TermInfoBuf {
            names: ti.names().map(|n| String::from(n)).collect(),
            bools: ti.get_bools(),
            numbers: ti.get_numbers(),
            strings: ti.get_string_offsets(),
            strtab: ti.get_strtab(),
            ext: None,
        };

        if let Some(ext) = ti.get_ext() {
            let (strtab, nametab) = ext.get_tables();
            tib.ext = Some(TermInfoExtBuf {
                bools: ext.get_bools(),
                numbers: ext.get_numbers(),
                strings: ext.get_string_offsets(),
                names: ext.get_name_offsets(),
                strtab: strtab,
                nametab: nametab,
            });
        }

        tib
    }

    pub fn new() -> TermInfoBuf {
        TermInfoBuf {
            names: Vec::new(),
            bools: Vec::with_capacity(PREDEFINED_BOOLEANS_COUNT),
            numbers: Vec::with_capacity(PREDEFINED_BOOLEANS_COUNT),
            strings: Vec::with_capacity(PREDEFINED_STRINGS_COUNT),
            strtab: StringTable::new(),
            ext: None,
        }
    }

    pub fn has_ext(&self) -> bool {
        self.ext.is_some()
    }

    pub fn boolean(&self, field: BooleanField) -> bool {
        self.bools
            .iter()
            .nth(field as usize)
            .map(|x| *x)
            .unwrap_or(false)
    }

    pub fn number(&self, field: NumericField) -> Option<u32> {
        let x = self.numbers
            .iter()
            .nth(field as usize)
            .map(|x| *x)
            .unwrap_or(invalid());

        if x == invalid() {
            None
        } else {
            Some(x)
        }
    }

    pub fn string(&self, field: StringField) -> Option<&str> {
        if let Ok(s) = self.strtab.get(
            self.strings
                .iter()
                .nth(field as usize)
                .map(|&x| x as usize)
                .unwrap_or(invalid()),
        ) {
            Some(s)
        } else {
            None
        }
    }

    /// Execute a string
    pub fn exec<'a>(&'a self, field: StringField) -> Option<lang::Executor<'a>> {
        if let Ok(s) = self.strtab.get_slice(
            self.strings
                .iter()
                .nth(field as usize)
                .map(|&x| x as usize)
                .unwrap_or(invalid()),
        ) {
            Some(lang::Executor::new(s))
        } else {
            None
        }
    }

    pub fn ext_index<T: AsRef<str>>(&self, s: T) -> Option<usize> {
        match &self.ext {
            Some(e) => {
                let bytes = s.as_ref().as_bytes();
                for (i, x) in e.names.iter().enumerate() {
                    if e.nametab.get_iter(*x as usize).eq(bytes) {
                        return Some(i);
                    }
                }
                None
            }
            None => None,
        }
    }

    pub fn ext_boolean<T: AsRef<str>>(&self, field: T) -> bool {
        if let Some(ref ext) = self.ext {
            if let Some(idx) = self.ext_index(field) {
                if idx < ext.bools.len() {
                    return ext.bools[idx];
                }
            }
        }
        false
    }

    pub fn ext_number<T: AsRef<str>>(&self, field: T) -> Option<u32> {
        if let Some(ref ext) = self.ext {
            if let Some(idx) = self.ext_index(field) {
                let idx_offset = ext.bools.len();
                if idx >= idx_offset && idx - idx_offset < ext.numbers.len() {
                    return Some(ext.numbers[idx - idx_offset]);
                }
            }
        }
        None
    }

    pub fn ext_string<T: AsRef<str>>(&self, field: T) -> Option<&str> {
        if let Some(ref ext) = self.ext {
            if let Some(idx) = self.ext_index(field) {
                let idx_offset = ext.bools.len() + ext.numbers.len();
                if idx >= idx_offset && idx - idx_offset < ext.strings.len() {
                    return ext.strtab
                        .get(ext.strings[idx - idx_offset] as usize)
                        .map(|x| Some(x))
                        .unwrap_or(None);
                }
            }
        }
        None
    }

    #[inline]
    pub fn set_boolean(&mut self, field: BooleanField, v: bool) -> Result<()> {
        let i = field as usize;
        while self.bools.len() <= i {
            self.bools.push(false)
        }

        self.bools[i] = v;

        Ok(())
    }

    #[inline]
    pub fn set_number(&mut self, field: NumericField, v: u32) -> Result<()> {
        let i = field as usize;
        while self.numbers.len() <= i {
            self.numbers.push(invalid())
        }
        self.numbers[i] = v;

        Ok(())
    }

    pub fn set_string<T: AsRef<str>>(&mut self, field: StringField, v: T) -> Result<()> {
        let i = field as usize;
        while self.strings.len() <= i {
            self.strings.push(invalid())
        }

        let offset = self.strtab.add(v);
        if offset >= invalid::<usize>() - 1 {
            return Err(ErrorKind::MaxStrTabSizeReached.into());
        }

        self.strings[i] = offset as u16;

        Ok(())
    }

    pub fn set_ext_boolean(&mut self, field: String, v: bool) -> Result<()> {
        let idx = self.ext_index(&field);

        if let Some(ref mut ext) = self.ext {
            if ext.bools.len() > u16::max_value() as usize && idx.is_none() {
                return Err(ErrorKind::MaximumCapabilityCountExceeded.into());
            }

            if let Some(x) = idx {
                ext.bools[x] = v;
            } else {
                let offset = ext.nametab.add(field);
                if offset >= invalid::<usize>() - 1 {
                    return Err(ErrorKind::MaxStrTabSizeReached.into());
                }

                ext.bools.push(v);
                ext.names.insert(ext.bools.len(), offset as u16)
            }
            return Ok(());
        }

        let mut ext = TermInfoExtBuf::new();
        ext.bools.push(v);

        let offset = ext.nametab.add(field);
        if offset >= invalid::<usize>() - 1 {
            return Err(ErrorKind::MaxStrTabSizeReached.into());
        }

        ext.names.push(offset as u16);
        self.ext = Some(ext);

        Ok(())
    }

    pub fn set_ext_number(&mut self, field: String, v: u32) -> Result<()> {
        let idx = self.ext_index(&field);
        if let Some(ref mut ext) = self.ext {
            if ext.bools.len() > u16::max_value() as usize && idx.is_none() {
                return Err(ErrorKind::MaximumCapabilityCountExceeded.into());
            }

            if let Some(x) = idx {
                let xoff = ext.bools.len();
                if x > xoff && x < xoff + self.numbers.len() {
                    self.numbers[x - xoff] = v;
                }
            } else {
                let offset = ext.nametab.add(field);
                if offset >= invalid::<usize>() - 1 {
                    return Err(ErrorKind::MaxStrTabSizeReached.into());
                }

                ext.numbers.push(v);
                ext.names.insert(ext.numbers.len(), offset as u16)
            }
            return Ok(());
        }
        let mut ext = TermInfoExtBuf::new();
        ext.numbers.push(v);

        let offset = ext.nametab.add(field);
        if offset >= invalid::<usize>() - 1 {
            return Err(ErrorKind::MaxStrTabSizeReached.into());
        }

        ext.names.push(offset as u16);
        self.ext = Some(ext);

        Ok(())
    }

    pub fn set_ext_string(&mut self, field: String, v: String) -> Result<()> {
        let idx = self.ext_index(&field);

        if let Some(ref mut ext) = self.ext {
            if ext.bools.len() > u16::max_value() as usize && idx.is_none() {
                return Err(ErrorKind::MaximumCapabilityCountExceeded.into());
            }

            let strtab_ref = ext.strtab.add(v) as u16;
            if strtab_ref >= invalid::<u16>() - 1 {
                return Err(ErrorKind::MaxStrTabSizeReached.into());
            }

            if let Some(x) = idx {
                let xoff = ext.bools.len();
                if x > xoff && x < xoff + self.strings.len() {
                    self.strings[x - xoff] = strtab_ref;
                }
            } else {
                let offset = ext.nametab.add(field);
                if offset >= invalid::<usize>() - 1 {
                    return Err(ErrorKind::MaxStrTabSizeReached.into());
                }

                ext.strings.push(strtab_ref);
                ext.names.insert(ext.strings.len(), offset as u16)
            }
            return Ok(());
        }

        let mut ext = TermInfoExtBuf::new();
        let strtab_ref = ext.strtab.add(v) as u16;
        if strtab_ref >= invalid::<u16>() - 1 {
            return Err(ErrorKind::MaxStrTabSizeReached.into());
        }

        ext.strings.push(strtab_ref as u16);
        let offset = ext.nametab.add(field);
        if offset >= invalid::<usize>() - 1 {
            return Err(ErrorKind::MaxStrTabSizeReached.into());
        }

        ext.names.push(offset as u16);
        self.ext = Some(ext);

        Ok(())
    }
}

impl<'a> From<TermInfo<'a>> for TermInfoBuf {
    fn from(src: TermInfo<'a>) -> TermInfoBuf {
        TermInfoBuf::from_terminfo(&src)
    }
}

impl<'a> From<&'a TermInfo<'a>> for TermInfoBuf {
    fn from(src: &'a TermInfo<'a>) -> TermInfoBuf {
        TermInfoBuf::from_terminfo(src)
    }
}

#[cfg(test)]
mod test {
    use terminfo::*;

    const RXVT_INFO: &'static [u8] = include_bytes!("../../test-data/rxvt");
    const XTERM_INFO: &'static [u8] = include_bytes!("../../test-data/xterm");
    const LINUX_16COLOR_INFO: &'static [u8] = include_bytes!("../../test-data/linux-16color");

    #[test]
    fn from_terminfo() {
        use std::mem;

        let rxvt = TermInfo::parse(RXVT_INFO).unwrap();
        let xterm = TermInfo::parse(XTERM_INFO).unwrap();
        let l16c = TermInfo::parse(LINUX_16COLOR_INFO).unwrap();

        let rxvt_buf: TermInfoBuf = rxvt.clone().into();
        let xterm_buf: TermInfoBuf = xterm.clone().into();
        let l16c_buf: TermInfoBuf = l16c.clone().into();

        for i in 0..PREDEFINED_BOOLEANS_COUNT {
            let field = unsafe { mem::transmute(i) };

            assert_eq!(rxvt.boolean(field), rxvt_buf.boolean(field));
            assert_eq!(xterm.boolean(field), xterm_buf.boolean(field));
            assert_eq!(l16c.boolean(field), l16c_buf.boolean(field));
        }

        for i in 0..PREDEFINED_NUMERICS_COUNT {
            let field = unsafe { mem::transmute(i) };

            assert_eq!(rxvt.number(field), rxvt_buf.number(field));
            assert_eq!(xterm.number(field), xterm_buf.number(field));
            assert_eq!(l16c.number(field), l16c_buf.number(field));
        }

        for i in 0..PREDEFINED_STRINGS_COUNT {
            let field = unsafe { mem::transmute(i) };

            assert_eq!(rxvt.string(field), rxvt_buf.string(field));
            assert_eq!(xterm.string(field), xterm_buf.string(field));
            assert_eq!(l16c.string(field), l16c_buf.string(field));
        }
    }

    #[test]
    fn lookup_string() {
        let rxvt: TermInfoBuf = TermInfo::parse(RXVT_INFO).unwrap().into();
        let xterm: TermInfoBuf = TermInfo::parse(XTERM_INFO).unwrap().into();
        let l16c: TermInfoBuf = TermInfo::parse(LINUX_16COLOR_INFO).unwrap().into();

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
        let rxvt: TermInfoBuf = TermInfo::parse(RXVT_INFO).unwrap().into();
        let xterm: TermInfoBuf = TermInfo::parse(XTERM_INFO).unwrap().into();
        let l16c: TermInfoBuf = TermInfo::parse(LINUX_16COLOR_INFO).unwrap().into();

        assert_eq!(rxvt.boolean(BooleanField::LinefeedIsNewline), false);
        assert_eq!(rxvt.boolean(BooleanField::AutoRightMargin), true);

        assert_eq!(xterm.boolean(BooleanField::AutoLeftMargin), false);
        assert_eq!(xterm.boolean(BooleanField::LinefeedIsNewline), false);

        assert_eq!(l16c.boolean(BooleanField::HardCursor), false);
        assert_eq!(l16c.boolean(BooleanField::AutoRightMargin), true);
    }

    #[test]
    fn lookup_number() {
        let rxvt: TermInfoBuf = TermInfo::parse(RXVT_INFO).unwrap().into();
        let xterm: TermInfoBuf = TermInfo::parse(XTERM_INFO).unwrap().into();
        let l16c: TermInfoBuf = TermInfo::parse(LINUX_16COLOR_INFO).unwrap().into();

        assert_eq!(rxvt.number(NumericField::Columns), Some(80));
        assert_eq!(rxvt.number(NumericField::Buttons), None);

        assert_eq!(xterm.number(NumericField::WideCharSize), None);
        assert_eq!(xterm.number(NumericField::MaxColors), Some(8));

        assert_eq!(l16c.number(NumericField::BitImageEntwining), None);
        assert_eq!(l16c.number(NumericField::MaxColors), Some(16));
    }

    #[test]
    fn set_string() {
        let mut rxvt: TermInfoBuf = TermInfo::parse(RXVT_INFO).unwrap().into();
        let mut new = TermInfoBuf::new();

        rxvt.set_string(StringField::KeyF10, String::from("~~F10~~"))
            .unwrap();
        rxvt.set_string(StringField::ZeroMotion, String::from("Hello World"))
            .unwrap();

        new.set_string(StringField::WaitTone, "Hi").unwrap();

        assert_eq!(rxvt.string(StringField::KeyF10), Some("~~F10~~"));
        assert_eq!(rxvt.string(StringField::ZeroMotion), Some("Hello World"));
        assert_eq!(new.string(StringField::WaitTone), Some("Hi"));
    }

    #[test]
    fn set_bool() {
        let mut rxvt: TermInfoBuf = TermInfo::parse(RXVT_INFO).unwrap().into();
        let mut new = TermInfoBuf::new();

        rxvt.set_boolean(BooleanField::EatNewlineGlitch, true)
            .unwrap();
        rxvt.set_boolean(BooleanField::TildeGlitch, true).unwrap();
        rxvt.set_boolean(BooleanField::AutoRightMargin, false)
            .unwrap();

        new.set_boolean(BooleanField::XonXoff, true).unwrap();

        assert_eq!(rxvt.boolean(BooleanField::EatNewlineGlitch), true);
        assert_eq!(rxvt.boolean(BooleanField::TildeGlitch), true);
        assert_eq!(rxvt.boolean(BooleanField::AutoRightMargin), false);
        assert_eq!(new.boolean(BooleanField::XonXoff), true);
    }

    #[test]
    fn set_number() {
        let mut rxvt: TermInfoBuf = TermInfo::parse(RXVT_INFO).unwrap().into();
        let mut new = TermInfoBuf::new();

        rxvt.set_number(NumericField::WideCharSize, 2000).unwrap();
        rxvt.set_number(NumericField::Lines, 2).unwrap();

        new.set_number(NumericField::PrintRate, 5).unwrap();

        assert_eq!(rxvt.number(NumericField::WideCharSize), Some(2000));
        assert_eq!(rxvt.number(NumericField::Lines), Some(2));
        assert_eq!(new.number(NumericField::PrintRate), Some(5));
    }

    #[test]
    fn lookup_ext_string() {
        let xterm: TermInfoBuf = TermInfo::parse(XTERM_INFO).unwrap().into();

        assert_eq!(xterm.has_ext(), true);
        assert_eq!(xterm.ext_string("kUP7"), Some("\u{1b}[1;7A"));
        assert_eq!(xterm.ext_string("kUP8"), None);
    }

    #[test]
    fn lookup_ext_bool() {
        let rxvt: TermInfoBuf = TermInfo::parse(RXVT_INFO).unwrap().into();

        assert_eq!(rxvt.has_ext(), true);
        assert_eq!(rxvt.ext_boolean("XM"), false);
        assert_eq!(rxvt.ext_boolean("G0"), false);
        assert_eq!(rxvt.ext_boolean("XT"), true);
        assert_eq!(rxvt.ext_boolean("kUP"), false);
    }

    #[test]
    fn lookup_ext_number() {
        let l16c: TermInfoBuf = TermInfo::parse(LINUX_16COLOR_INFO).unwrap().into();

        assert_eq!(l16c.has_ext(), true);
        assert_eq!(l16c.ext_number("U8"), Some(1));
    }

}
