use std::io::Error as IoError;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::sync::RwLock;

use super::Rc;

pub struct IoSlice<R: Read + Seek> {
    reader: Rc<RwLock<R>>,
    pos: u64,
    start: u64,
    end: u64,
}

impl<R: Read + Seek> IoSlice<R> {
    pub fn new(reader: Rc<RwLock<R>>, start: u64, end: u64) -> Self {
        Self {
            reader,
            start,
            end,
            pos: start,
        }
    }
}

impl<R: Read + Seek> Read for IoSlice<R> {
    fn read(&mut self, mut buffer: &mut [u8]) -> Result<usize, IoError> {
        let mut reader = self.reader.write().unwrap();
        reader.seek(SeekFrom::Start(self.pos))?;

        let remaining = (self.end - self.pos) as usize;
        if buffer.len() > remaining {
            buffer = &mut buffer[..remaining];
        }

        let n = reader.read(buffer)?;
        self.pos += n as u64;

        Ok(n)
    }
}

impl<R: Read + Seek> Seek for IoSlice<R> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, IoError> {
        match pos {
            SeekFrom::Start(n) => {
                if self.start + n > self.end {
                    self.pos = self.end;
                } else {
                    self.pos = self.start + n;
                }
            }
            SeekFrom::End(n) => {
                if n > 0 {
                    self.pos = self.end;
                } else if n + (self.end as i64) < (self.start as i64) {
                    return Err(IoError::new(
                        std::io::ErrorKind::InvalidInput,
                        "invalid seek to a negative or overflowing position",
                    ));
                } else {
                    self.pos = (self.end as i64 + n) as u64;
                }
            }
            SeekFrom::Current(n) => {
                if n + (self.pos as i64) > (self.end as i64) {
                    self.pos = self.end;
                } else {
                    self.pos = (self.pos as i64 + n) as u64;
                }
            }
        }
        Ok(self.pos)
    }

    fn rewind(&mut self) -> Result<(), IoError> {
        self.pos = self.start;
        Ok(())
    }

    #[cfg(feature = "nightly")]
    fn stream_len(&mut self) -> Result<u64, IoError> {
        Ok(self.end - self.start)
    }

    fn stream_position(&mut self) -> Result<u64, IoError> {
        Ok(self.pos - self.start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Cursor;

    #[test]
    fn read() {
        let data = b"ABCDEF";
        let cursor = Cursor::new(&data);
        let f = Rc::new(RwLock::new(cursor));

        let mut s1 = IoSlice::new(f.clone(), 0, 2);
        let mut s2 = IoSlice::new(f.clone(), 2, 4);
        let mut s3 = IoSlice::new(f.clone(), 4, 6);

        assert_eq!(std::io::read_to_string(&mut s2).unwrap(), "CD");
        assert_eq!(std::io::read_to_string(&mut s1).unwrap(), "AB");
        assert_eq!(std::io::read_to_string(&mut s3).unwrap(), "EF");
        assert_eq!(std::io::read_to_string(&mut s2).unwrap(), "");
    }

    #[test]
    fn seek() {
        let data = b"ABCDEF";
        let cursor = Cursor::new(&data);
        let f = Rc::new(RwLock::new(cursor));

        let mut s1 = IoSlice::new(f.clone(), 0, 2);
        let mut s2 = IoSlice::new(f.clone(), 2, 4);
        let mut s3 = IoSlice::new(f.clone(), 4, 6);

        s2.seek(SeekFrom::Start(1)).unwrap();
        assert_eq!(std::io::read_to_string(&mut s1).unwrap(), "AB");
        assert_eq!(std::io::read_to_string(&mut s2).unwrap(), "D");

        s3.seek(SeekFrom::End(-1)).unwrap();
        assert_eq!(std::io::read_to_string(&mut s3).unwrap(), "F");

        s1.seek(SeekFrom::End(-2)).unwrap();
        assert_eq!(std::io::read_to_string(&mut s3).unwrap(), "");
        assert_eq!(std::io::read_to_string(&mut s1).unwrap(), "AB");

        s1.seek(SeekFrom::Current(-1)).unwrap();
        assert_eq!(std::io::read_to_string(&mut s1).unwrap(), "B");
    }
}
