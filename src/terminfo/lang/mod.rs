mod argument;
pub mod executor;
pub mod parser;
pub mod printf;

pub use self::argument::*;
pub use self::executor::Executor;

#[cfg(test)]
mod tests {
    use terminfo::lang::printf::*;
    use terminfo::lang::*;

    #[test]
    fn printf() {
        let mut buffer = Vec::new();
        PrintfArgs::parse(b"s")
            .unwrap()
            .print(&mut buffer, Some("Hello World!"))
            .unwrap();
        assert_eq!(&buffer, &b"Hello World!");

        buffer.clear();
        PrintfArgs::parse(b"d")
            .unwrap()
            .print(&mut buffer, Some(12))
            .unwrap();
        assert_eq!(&buffer, &b"12");

        buffer.clear();
        PrintfArgs::parse(b": d")
            .unwrap()
            .print(&mut buffer, Some(12))
            .unwrap();
        assert_eq!(&buffer, &b" 12");

        buffer.clear();
        PrintfArgs::parse(b"d")
            .unwrap()
            .print(&mut buffer, Some(i64::max_value()))
            .unwrap();
        assert_eq!(&buffer, b"9223372036854775807");
        buffer.clear();

        PrintfArgs::parse(b":-5d")
            .unwrap()
            .print(&mut buffer, Some(21))
            .unwrap();
        assert_eq!(&buffer, b"21   ");
        buffer.clear();

        PrintfArgs::parse(b".1d")
            .unwrap()
            .print(&mut buffer, Some(21))
            .unwrap();
        assert_eq!(&buffer, b"2");
        buffer.clear();

        PrintfArgs::parse(b".4s")
            .unwrap()
            .print(&mut buffer, Some("Crop Me"))
            .unwrap();
        assert_eq!(&buffer, b"Crop");
        buffer.clear();

        PrintfArgs::parse(b"9.4s")
            .unwrap()
            .print(&mut buffer, Some("Crop Me"))
            .unwrap();
        assert_eq!(&buffer, b"     Crop");
        buffer.clear();

        PrintfArgs::parse(b":-9.4d")
            .unwrap()
            .print(&mut buffer, Some(99))
            .unwrap();
        assert_eq!(&buffer, b"99       ");
        buffer.clear();

        PrintfArgs::parse(b":-9.4d")
            .unwrap()
            .print(&mut buffer, Some(99999))
            .unwrap();
        assert_eq!(&buffer, b"9999     ");
    }

    #[test]
    fn simple() {
        let mut buffer = Vec::new();
        Executor::new(b"%p1%p2%+%d")
            .arg(30)
            .arg(2)
            .write(&mut buffer)
            .unwrap();
        assert_eq!(&buffer, b"32");
        buffer.clear();

        Executor::new(b"%{8}%{22}%+%d%'c'%c")
            .write(&mut buffer)
            .unwrap();
        assert_eq!(&buffer, b"30c");
        buffer.clear();

        Executor::new(b"%p1%p2%+/%d%p1%.1d")
            .arg(40)
            .arg(20)
            .write(&mut buffer)
            .unwrap();
        assert_eq!(&buffer, b"/604");
        buffer.clear();

        Executor::new(b"%i%p1%d.%p2%d")
            .arg(1)
            .arg(2)
            .write(&mut buffer)
            .unwrap();
        assert_eq!(&buffer, b"2.3");
        buffer.clear();

        Executor::new(b"%p1%l%p1\"%s\" is %d characters long!")
            .arg("Hello World")
            .write(&mut buffer)
            .unwrap();
        assert_eq!(
            buffer,
            b"\"Hello World\" is 11 characters long!"
                .iter()
                .map(|&c| c)
                .collect::<Vec<u8>>()
        );
        buffer.clear();

        Executor::new(b"%?%p1%tyes!%;")
            .arg(1)
            .write(&mut buffer)
            .unwrap();
        assert_eq!(&buffer, b"yes!");
        buffer.clear();

        Executor::new(b"%?%p1%tyes!%eno!%;")
            .arg(0)
            .write(&mut buffer)
            .unwrap();
        assert_eq!(&buffer, b"no!");
        buffer.clear();

        Executor::new(b"%?%p9%t\\E(0%e\\E(B%;\\E[0%?%p6%t;1%;%?%p2%t;4%;%?%p1%p3%|%t;7%;%?%p4%t;5%;%?%p7%t;8%;m")
            .arg(1)
            .write(&mut buffer) 
            .unwrap();
        assert_eq!(buffer, b"\\E(B\\E[0;7m");
        buffer.clear();

        Executor::new(b"\x1b[%?%p1%{8}%<%t3%p1%d%e%p1%{16}%<%t9%p1%{8}%-%d%e38;5;%p1%d%;m")
            .arg(3)
            .write(&mut buffer)
            .unwrap();
        assert_eq!(&String::from_utf8(buffer).unwrap(), "\x1b[33m");
    }
}
