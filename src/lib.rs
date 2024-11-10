use std::io::{BufRead, Write};

const CR: u8 = b'\r';
const LF: u8 = b'\n';

const CRLF_BUF: [u8; 2] = [CR, LF];
const LF_BUF: [u8; 1] = [LF];

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

#[derive(Debug, Clone, Default)]
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
        None
    }

    pub fn lf(&self) -> usize {
        self.lf
    }

    pub fn crlf(&self) -> usize {
        self.crlf
    }

    pub fn measure_file<R: BufRead>(mut source: R) -> std::io::Result<CrlfStat> {
        let mut buf = vec![];
        let mut stat = CrlfStat::default();
        loop {
            if source.read_until(LF, &mut buf)? == 0 {
                break;
            }
            if buf.ends_with(&CRLF_BUF) {
                stat.crlf += 1;
            } else {
                stat.lf += 1;
            }
            buf.clear();
        }
        Ok(stat)
    }
}

pub fn convert_to<R: BufRead, W: Write>(
    mut source: R,
    mut dest: W,
    ending: LineEnding,
) -> std::io::Result<()> {
    let mut buf = vec![];

    loop {
        if source.read_until(LF, &mut buf)? == 0 {
            break;
        }
        let has_line_ending = buf.last().is_some_and(|c| *c == LF);
        if has_line_ending {
            buf.pop();
            if buf.last() == Some(&CR) {
                buf.pop();
            }
        }
        dest.write_all(&buf)?;
        buf.clear();
        if has_line_ending {
            match ending {
                LineEnding::CRLF => {
                    dest.write_all(&CRLF_BUF)?;
                }
                LineEnding::LF => {
                    dest.write_all(&LF_BUF)?;
                }
            }
        }
    }
    dest.flush()?;

    Ok(())
}

#[cfg(test)]
mod test {
    use std::io::{BufReader, Cursor};

    use super::*;

    #[test]
    fn test_stats() {
        let lf_file = std::fs::File::open("test/Cargo.toml.lf").unwrap();
        let stat = CrlfStat::measure_file(BufReader::new(lf_file)).unwrap();
        assert_eq!(stat.is_pure(), Some(LineEnding::LF));

        let crlf_file = std::fs::File::open("test/Cargo.toml.crlf").unwrap();
        let stat = CrlfStat::measure_file(BufReader::new(crlf_file)).unwrap();
        assert_eq!(stat.is_pure(), Some(LineEnding::CRLF));

        let mixed_file = std::fs::File::open("test/Cargo.toml.mixed").unwrap();
        let stat = CrlfStat::measure_file(BufReader::new(mixed_file)).unwrap();
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
