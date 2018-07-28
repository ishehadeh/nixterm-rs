use errors::*;
use failure::ResultExt;
use futures;
use lang::printf::PrintfArgs;
use lang::Argument;
use std::collections::VecDeque;
use std::result;
use std::sync::mpsc;

pub struct Parser<'a> {
    src: &'a [u8],
    slice: &'a [u8],
    buffer: VecDeque<Op<'a>>,
}

pub struct Span<'a> {
    slice: &'a [u8],
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Op<'a> {
    /// Push an argument onto the stack
    Push(usize),

    NoOp,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitAnd,
    BitOr,
    BitXor,
    Less,
    Greater,
    Equal,
    Invert,
    Not,

    /// increment the first two arguments
    IncrementArgs,

    /// Pop the stack, if the result is a string push it's length, otherwise fail.
    StrLen,

    /// Pop the stack, if the top value is non-empty string, a non-null char, or a non-zero number then jump
    BranchTrue(usize),

    /// Pop the stack, if the top value is an empty string, a null char, or zero then jump
    BranchFalse(usize),

    /// Ignore the next `x` ops
    Jump(usize),

    /// Pop the stack and print
    Print(PrintfArgs),

    /// Print a string literal
    PrintSlice(&'a [u8]),
}

impl<'a> Parser<'a> {
    pub fn new(src: &'a [u8]) -> Parser<'a> {
        Parser {
            src: src,
            slice: src,
            buffer: VecDeque::with_capacity(4),
        }
    }

    pub fn parse(&mut self) -> Result<()> {
        while self.slice.len() > 0 {
            self.next_instruction()?;
        }
        Ok(())
    }

    fn add_instruction(&mut self, op: Op<'a>) {
        self.buffer.push_back(op)
    }

    fn parse_until(&mut self, stop: &[u8]) -> Result<()> {
        if self.slice[0] == b'%' {
            for &c in stop {
                if c == self.slice[1] {
                    break;
                }
            }
        }

        while self.slice.len() >= 2 {
            // println!(
            //     "{} ? {}",
            //     self.slice.iter().map(|&c| c as char).collect::<String>(),
            //     stop.iter().map(|&c| c as char).collect::<String>()
            // );
            if self.slice[0] == b'%' {
                for &c in stop {
                    if c == self.slice[1] {
                        return Ok(());
                    }
                }
            }
            self.next_instruction()?;
        }

        Err(ErrorKind::UnexpectedEof.into())
    }

    /// Read up to the next instruction store it & exit.
    fn next_instruction(&mut self) -> Result<()> {
        if self.slice.len() == 0 {
            // EOF
            return Ok(());
        }

        if self.slice[0] != b'%' {
            let pos = self.slice.iter().take_while(|&&c| c != b'%').count();
            self.add_instruction(Op::PrintSlice(&self.slice[..pos]));
            self.slice = &self.slice[pos..];
            return Ok(());
        }

        if self.slice.len() == 1 {
            return Err(ErrorKind::UnexpectedEof.into());
        }

        // The number of characters read
        // initialized to 2 because there must be at least a % and one other character, in some cases there are more.
        let mut read = 2;

        match self.slice[1] {
            b'%' => self.add_instruction(Op::PrintSlice(b"%")),
            b'p' => {
                match self.slice.iter().skip(2).next() {
                    Some(i @ b'1'..=b'9') => self.add_instruction(Op::Push((i - b'1') as usize)),
                    _ => return Err(ErrorKind::InvalidArgumentIdentifier.into()),
                };
                read += 1;
            }
            b'i' => self.add_instruction(Op::IncrementArgs),
            b'l' => self.add_instruction(Op::StrLen),
            b'+' => self.add_instruction(Op::Add),
            b'-' => self.add_instruction(Op::Sub),
            b'*' => self.add_instruction(Op::Mul),
            b'/' => self.add_instruction(Op::Div),
            b'm' => self.add_instruction(Op::Mod),
            b'&' => self.add_instruction(Op::BitAnd),
            b'^' => self.add_instruction(Op::BitXor),
            b'|' => self.add_instruction(Op::BitOr),
            b'=' => self.add_instruction(Op::Equal),
            b'<' => self.add_instruction(Op::Less),
            b'>' => self.add_instruction(Op::Greater),
            b'~' => self.add_instruction(Op::Invert),
            b'!' => self.add_instruction(Op::Not),
            b'?' => {
                // add a placeholder for branch instruction, we will update it later
                self.slice = &self.slice[read..];
                self.parse_until(&[b't'])?;
                self.slice = &self.slice[2..];

                let branch_idx = self.buffer.len();
                self.add_instruction(Op::NoOp);

                self.parse_until(&[b'e', b';'])?;

                if self.slice.len() < 2 {
                    // missing end of if-statement
                    return Err(ErrorKind::UnexpectedEof.into());
                }

                if self.slice[1] == b'e' {
                    // add a placeholder jump instruction, we will update it later
                    let jump_idx = self.buffer.len();
                    self.add_instruction(Op::NoOp);
                    self.buffer[branch_idx] = Op::BranchFalse(self.buffer.len() - 1 - branch_idx);

                    self.slice = &self.slice[2..];
                    self.parse_until(&[b';'])?;

                    // when the IP reaches the end of the previous section jump over the else section.
                    self.buffer[jump_idx] = Op::Jump(self.buffer.len() - jump_idx);
                } else {
                    // if the condition fails jump to the after the %;
                    self.buffer[branch_idx] = Op::BranchFalse(self.buffer.len() - 1 - branch_idx);
                }
                read = 2;
            }
            _ => {
                self.add_instruction(Op::Print(PrintfArgs::parse(&self.slice[1..])?));
                read += self.slice
                    .iter()
                    .skip(1)
                    .take_while(|&&c| {
                        c != b'x' && c != b'X' && c != b'c' && c != b'd' && c != b'o' && c != b's'
                    })
                    .count();
            }
        };

        self.slice = &self.slice[read..];
        Ok(())
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = Result<Op<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.buffer.pop_front() {
            Some(v) => Some(Ok(v)),
            None => match self.next_instruction() {
                Ok(_) => Some(Ok(match self.buffer.pop_front() {
                    Some(v) => v,
                    None => return None,
                })),
                Err(e) => return Some(Err(e)),
            },
        }
    }
}
