#[macro_use]
extern crate lazy_static;

use std::io::{BufRead, BufReader, BufWriter, Read, Write};

const CR: u8 = 0x0D;
const LF: u8 = 0x0A;
lazy_static! {
    static ref CRLF_BUF: Vec<u8> = vec![CR, LF];
    static ref LF_BUF: Vec<u8> = vec![LF];
}

#[derive(Debug, Clone, Copy)]
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

pub fn convert_to<R: Read, W: Write>(source: R, dest: W, ending: LineEnding) {
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
        dest.write(&buf).unwrap();
        buf.clear();
        if has_line_ending {
            match ending {
                LineEnding::CRLF => {
                    dest.write(&CRLF_BUF).unwrap();
                }
                LineEnding::LF => {
                    dest.write(&LF_BUF).unwrap();
                }
            }
        }
    }
}
