use errors::*;
use lang::parser::{Op, Parser};
use lang::Argument;
use std::collections::VecDeque;
use std::io;

pub struct Executor<'a> {
    src: &'a [u8],
    env: ExecutionEnvironment,
    argc: usize,
}

pub struct ExecutionEnvironment {
    stack: VecDeque<Argument>,
    arguments: [Option<Argument>; 9],
}

impl<'a> Executor<'a> {
    pub fn new(src: &'a [u8]) -> Executor<'a> {
        Executor {
            env: ExecutionEnvironment::new(),
            src: src,
            argc: 0,
        }
    }

    /// set argument `i`, this method does nothing if `i` is greater than 8.
    #[inline]
    pub fn argi<U: Into<Argument>>(mut self, i: usize, a: U) -> Executor<'a> {
        if i < 9 {
            self.env.arguments[i] = Some(a.into());
        }
        self
    }

    /// push an argument, if 9 arguments have already been pushed than this method does nothing
    #[inline]
    pub fn arg<U: Into<Argument>>(mut self, a: U) -> Executor<'a> {
        if self.argc < 9 {
            self.env.arguments[self.argc] = Some(a.into());
            self.argc += 1;
        }
        self
    }

    pub fn string(&mut self) -> Result<String> {
        Ok(String::from_utf8(self.vec()?).unwrap())
    }

    pub fn vec(&mut self) -> Result<Vec<u8>> {
        let mut w = Vec::new();
        self.write(&mut w)?;
        Ok(w)
    }

    pub fn write<W: io::Write>(&mut self, w: &mut W) -> Result<()> {
        self.env.write(&mut Parser::new(self.src), w)
    }
}

impl ExecutionEnvironment {
    pub fn new() -> ExecutionEnvironment {
        ExecutionEnvironment {
            stack: VecDeque::new(),
            arguments: [None, None, None, None, None, None, None, None, None],
        }
    }

    pub fn pop_string(&mut self) -> Result<String> {
        match self.pop() {
            Some(Argument::Integer(_)) => {
                Err(ErrorKind::UnexpectedArgumentType("string", "integer").into())
            }
            Some(Argument::String(s)) => Ok(s),
            Some(Argument::Char(c)) => {
                Err(ErrorKind::UnexpectedArgumentType("string", "char").into())
            }
            None => Err(ErrorKind::UnexpectedArgumentType("string", "null").into()),
        }
    }

    pub fn pop_integer(&mut self) -> Result<i64> {
        match self.pop() {
            Some(Argument::Integer(x)) => Ok(x),
            Some(Argument::String(_)) => {
                Err(ErrorKind::UnexpectedArgumentType("integer", "string").into())
            }
            Some(Argument::Char(_)) => {
                Err(ErrorKind::UnexpectedArgumentType("integer", "char").into())
            }
            None => Err(ErrorKind::UnexpectedArgumentType("string", "null").into()),
        }
    }

    pub fn pop_char(&mut self) -> Result<u8> {
        match self.pop() {
            Some(Argument::Integer(_)) => {
                Err(ErrorKind::UnexpectedArgumentType("char", "integer").into())
            }
            Some(Argument::String(_)) => {
                Err(ErrorKind::UnexpectedArgumentType("char", "string").into())
            }
            Some(Argument::Char(c)) => Ok(c),
            None => Err(ErrorKind::UnexpectedArgumentType("char", "null").into()),
        }
    }

    pub fn pop(&mut self) -> Option<Argument> {
        self.stack.pop_back()
    }

    pub fn push<U: Into<Argument>>(&mut self, t: U) {
        self.stack.push_back(t.into())
    }

    fn map_integer2<U: Into<Argument>, F: FnOnce(i64, i64) -> U>(&mut self, f: F) -> Result<()> {
        let x = self.pop_integer()?;
        let y = self.pop_integer()?;

        self.push(f(x, y));
        Ok(())
    }

    fn map_integer<U: Into<Argument>, F: FnOnce(i64) -> U>(&mut self, f: F) -> Result<()> {
        let x = self.pop_integer()?;

        self.push(f(x));
        Ok(())
    }

    fn pop_bool(&mut self) -> bool {
        match self.pop() {
            Some(Argument::Integer(x)) => x != 0,
            Some(Argument::String(s)) => !s.is_empty(),
            Some(Argument::Char(c)) => c != 0,
            None => false,
        }
    }

    pub fn write<'a, W: io::Write>(&mut self, parser: &'a mut Parser<'a>, w: &mut W) -> Result<()> {
        'exe: loop {
            let op = match parser.next() {
                Some(v) => v?,
                None => break,
            };

            match op {
                Op::NoOp => (),
                Op::Push(arg) => {
                    let val = self.arguments[arg].clone().unwrap_or(Argument::Integer(0));
                    self.push(val)
                }
                Op::Jump(ip) => for _ in 0..ip {
                    match parser.next() {
                        Some(Err(e)) => return Err(e),
                        Some(Ok(_)) => (),
                        None => break 'exe,
                    }
                },
                Op::BranchFalse(ip) => if !self.pop_bool() {
                    for _ in 0..ip {
                        match parser.next() {
                            Some(Err(e)) => return Err(e),
                            Some(Ok(_)) => (),
                            None => break 'exe,
                        }
                    }
                },
                Op::BranchTrue(ip) => if self.pop_bool() {
                    for _ in 0..ip {
                        match parser.next() {
                            Some(Err(e)) => return Err(e),
                            Some(Ok(_)) => (),
                            None => break 'exe,
                        }
                    }
                },
                Op::Add => self.map_integer2(|x, y| x + y)?,
                Op::Sub => self.map_integer2(|x, y| x - y)?,
                Op::Div => self.map_integer2(|x, y| x / y)?,
                Op::Mul => self.map_integer2(|x, y| x * y)?,
                Op::Mod => self.map_integer2(|x, y| x % y)?,
                Op::BitAnd => self.map_integer2(|x, y| x & y)?,
                Op::BitOr => self.map_integer2(|x, y| x | y)?,
                Op::BitXor => self.map_integer2(|x, y| x ^ y)?,
                Op::Equal => self.map_integer2(|x, y| x == y)?,
                Op::Greater => self.map_integer2(|x, y| x > y)?,
                Op::Less => self.map_integer2(|x, y| x < y)?,
                Op::Invert => self.map_integer(|x| !x)?,
                Op::Not => self.map_integer(|x| if x != 0 { x == 0 } else { x == 1 })?,
                Op::IncrementArgs => {
                    match self.arguments[0] {
                        Some(Argument::Integer(ref mut x)) => *x += 1,
                        _ => (),
                    };
                    match self.arguments[1] {
                        Some(Argument::Integer(ref mut x)) => *x += 1,
                        _ => (),
                    };
                }
                Op::StrLen => {
                    let x = self.pop_string()?.len();
                    self.push(x);
                }
                Op::Print(p) => {
                    p.print(w, self.pop())?;
                }
                Op::PrintSlice(slice) => {
                    w.write(slice);
                }
            }
        }

        Ok(())
    }
}
