#[macro_use]
extern crate lazy_static;

use std::io::{BufRead, BufReader, BufWriter, Read, Write};

const CR: u8 = 0x0D;
const LF: u8 = 0x0A;
lazy_static! {
    static ref CRLF_BUF: Vec<u8> = vec![CR, LF];
    static ref LF_BUF: Vec<u8> = vec![LF];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    CRLF,
    LF,
}

impl std::fmt::Display for LineEnding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LineEnding::CRLF => write!(f, "crlf"),
            LineEnding::LF => write!(f, "lf"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CrlfStat {
    lf: usize,
    crlf: usize,
}

impl CrlfStat {
    pub fn is_pure(&self) -> Option<LineEnding> {
        if self.lf == 0 && self.crlf != 0 {
            return Some(LineEnding::CRLF);
        }
        if self.lf != 0 && self.crlf == 0 {
            return Some(LineEnding::LF);
        }
        return None;
    }

    pub fn lf(&self) -> usize {
        self.lf
    }

    pub fn crlf(&self) -> usize {
        self.crlf
    }

    pub fn new() -> CrlfStat {
        CrlfStat { lf: 0, crlf: 0 }
    }

    pub fn measure_file<R: Read>(source: R) -> CrlfStat {
        let mut reader = BufReader::new(source);
        let mut buf = vec![];
        let mut stat = CrlfStat::new();
        loop {
            match reader.read_until(LF, &mut buf) {
                Ok(n) => {
                    if n == 0 {
                        break;
                    }
                }
                Err(_) => break,
            };
            if buf.len() >= 2 && buf[buf.len() - 2] == CR {
                stat.crlf += 1;
            } else {
                stat.lf += 1;
            }
            buf.clear();
        }
        return stat;
    }
}

pub fn convert_to<R: Read, W: Write>(
    source: R,
    dest: W,
    ending: LineEnding,
) -> std::io::Result<()> {
    let mut source = BufReader::new(source);
    let mut dest = BufWriter::new(dest);
    let mut buf = vec![];

    loop {
        match source.read_until(LF, &mut buf) {
            Ok(n) => {
                if n == 0 {
                    break;
                }
            }
            Err(_) => break,
        }
        let has_line_ending = match buf.last() {
            None => false,
            Some(c) => *c == LF,
        };
        if has_line_ending {
            buf.pop();
            if buf.last() == Some(&CR) {
                buf.pop();
            }
        }
        dest.write(&buf)?;
        buf.clear();
        if has_line_ending {
            match ending {
                LineEnding::CRLF => {
                    dest.write(&CRLF_BUF)?;
                }
                LineEnding::LF => {
                    dest.write(&LF_BUF)?;
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_stats() {
        let lf_file = std::fs::File::open("test/Cargo.toml.lf").unwrap();
        let stat = CrlfStat::measure_file(lf_file);
        assert_eq!(stat.is_pure(), Some(LineEnding::LF));

        let crlf_file = std::fs::File::open("test/Cargo.toml.crlf").unwrap();
        let stat = CrlfStat::measure_file(crlf_file);
        assert_eq!(stat.is_pure(), Some(LineEnding::CRLF));

        let mixed_file = std::fs::File::open("test/Cargo.toml.mixed").unwrap();
        let stat = CrlfStat::measure_file(mixed_file);
        assert_eq!(stat.is_pure(), None);
        assert_eq!(stat.crlf(), 8);
        assert_eq!(stat.lf(), 6);
    }

    #[test]
    fn test_convert() {
        let lf_file = std::fs::read("test/Cargo.toml.lf").unwrap();
        let crlf_file = std::fs::read("test/Cargo.toml.crlf").unwrap();
        let mixed_file = std::fs::read("test/Cargo.toml.mixed").unwrap();
        let mut dst_buf = vec![];

        convert_to(
            Cursor::new(&lf_file),
            Cursor::new(&mut dst_buf),
            LineEnding::LF,
        )
        .unwrap();
        assert_eq!(dst_buf, lf_file);
        dst_buf.clear();

        convert_to(
            Cursor::new(&lf_file),
            Cursor::new(&mut dst_buf),
            LineEnding::CRLF,
        )
        .unwrap();
        assert_eq!(dst_buf, crlf_file);
        dst_buf.clear();

        convert_to(
            Cursor::new(&crlf_file),
            Cursor::new(&mut dst_buf),
            LineEnding::LF,
        )
        .unwrap();
        assert_eq!(dst_buf, lf_file);
        dst_buf.clear();

        convert_to(
            Cursor::new(&crlf_file),
            Cursor::new(&mut dst_buf),
            LineEnding::CRLF,
        )
        .unwrap();
        assert_eq!(dst_buf, crlf_file);
        dst_buf.clear();

        convert_to(
            Cursor::new(&mixed_file),
            Cursor::new(&mut dst_buf),
            LineEnding::LF,
        )
        .unwrap();
        assert_eq!(dst_buf, lf_file);
        dst_buf.clear();

        convert_to(
            Cursor::new(&mixed_file),
            Cursor::new(&mut dst_buf),
            LineEnding::CRLF,
        )
        .unwrap();
        assert_eq!(dst_buf, crlf_file);
        dst_buf.clear();
    }
}
