#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Argument {
    Integer(i64),
    String(String),
    Char(u8),
}

impl From<String> for Argument {
    fn from(s: String) -> Argument {
        Argument::String(s)
    }
}

impl<'a> From<&'a str> for Argument {
    fn from(s: &'a str) -> Argument {
        Argument::String(String::from(s))
    }
}

impl From<i64> for Argument {
    fn from(s: i64) -> Argument {
        Argument::Integer(s)
    }
}

impl From<i32> for Argument {
    fn from(s: i32) -> Argument {
        Argument::Integer(s as i64)
    }
}

impl From<isize> for Argument {
    fn from(s: isize) -> Argument {
        Argument::Integer(s as i64)
    }
}

impl From<u64> for Argument {
    fn from(s: u64) -> Argument {
        Argument::Integer(s as i64)
    }
}

impl From<u32> for Argument {
    fn from(s: u32) -> Argument {
        Argument::Integer(s as i64)
    }
}

impl From<usize> for Argument {
    fn from(s: usize) -> Argument {
        Argument::Integer(s as i64)
    }
}

impl From<char> for Argument {
    fn from(c: char) -> Argument {
        Argument::Char(c as u8)
    }
}

impl From<u8> for Argument {
    fn from(c: u8) -> Argument {
        Argument::Char(c)
    }
}

impl From<bool> for Argument {
    fn from(b: bool) -> Argument {
        Argument::Integer(if b { 1 } else { 0 })
    }
}
